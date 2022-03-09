use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};

use futures::future::join_all;
use serde_bytes::ByteBuf;
use sha1::Digest;
use sha2::Sha512;
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
};
use zerucontent::Content;

use crate::{
    core::{error::*, io::*, models::*, peer::*, site::*, user::*},
    discovery::tracker::IpPort,
    environment::{self, ENV},
    net::Protocol,
    utils::atomic_write,
};
use log::*;
use serde_json::json;

impl Site {
    async fn load_content_from_path(&self, inner_path: String) -> Result<Content, Error> {
        let path = &self.site_path().join(&inner_path);
        if path.exists() {
            let mut file = File::open(path).await?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).await?;
            let content: Content = serde_json::from_slice(&buf)?;
            return Ok(content);
        }
        Err(Error::Err("Content File Not Found".into()))
    }

    async fn download_file_from_peer(
        &self,
        inner_path: String,
        peer: &mut Peer,
    ) -> Result<bool, Error> {
        let path = &self.site_path().join(&inner_path);
        let message = Protocol::new(peer.connection_mut().unwrap())
            .get_file(self.address(), inner_path)
            .await?;
        let parent = path.parent().unwrap();
        if !parent.exists() {
            fs::create_dir_all(parent).await?;
        }
        let mut file = File::create(path).await?;
        file.write_all(&message.body).await?;
        Ok(true)
    }

    async fn download_file(&self, inner_path: String, _peer: Option<Peer>) -> Result<bool, Error> {
        let path = &self.site_path().join(&inner_path);
        if path.exists() {
            let mut file = File::open(path).await?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).await?;
            return Ok(true);
        }
        //TODO!: Download from multiple peers
        let mut peer = self.peers.values().next().unwrap().clone();
        Self::download_file_from_peer(self, inner_path, &mut peer).await
    }

    async fn download_site_files(&self) -> Result<(), Error> {
        let files = self.content().unwrap().files;
        let mut tasks = Vec::new();
        let mut inner_paths = Vec::new();
        for (inner_path, _file) in files {
            inner_paths.push(inner_path.clone());
            let task = self.download_file(inner_path, None);
            tasks.push(task);
        }
        let includes = self.content().unwrap().includes;
        for (inner_path, _file) in includes {
            inner_paths.push(inner_path.clone());
            let task = self.download_file(inner_path, None);
            tasks.push(task);
        }
        //TODO!: Other client may not have an up-to-date site files
        let user_files = self.fetch_changes(1421043090).await?;
        //TODO!: Check for storage Permission
        let mut user_data_files = Vec::new();
        for (inner_path, _file) in user_files {
            if inner_paths.contains(&inner_path) {
                continue;
            }
            user_data_files.push(inner_path.clone());
            let task = self.download_file(inner_path, None);
            tasks.push(task);
        }
        let mut res = join_all(tasks).await;
        let errs = res.drain_filter(|res| !res.is_ok()).collect::<Vec<_>>();
        for err in errs {
            error!("{:?}", err);
        }

        let user_data = user_data_files
            .iter()
            .map(|path| {
                let content = self.load_content_from_path(path.clone());
                content
            })
            .collect::<Vec<_>>();
        let mut content_res = join_all(user_data).await;
        let errs = content_res
            .drain_filter(|res| !res.is_ok())
            .collect::<Vec<_>>();
        for err in errs {
            error!("{:?}", err.err());
        }
        let mut files = vec![];
        content_res.iter_mut().for_each(|content| {
            let content = content.as_ref().unwrap();
            let path = Path::new(&content.inner_path);
            if let Some(parent) = path.parent() {
                let files_inner = content.files.clone();
                for (path, _file) in files_inner {
                    files.push(
                        self.download_file(parent.join(path).to_str().unwrap().to_owned(), None),
                    );
                }
            }
        });
        let mut res = join_all(files).await;
        let errs = res.drain_filter(|res| !res.is_ok()).collect::<Vec<_>>();
        for err in errs {
            error!("{:?}", err);
        }

        Ok(())
    }

    pub async fn load_content(&mut self) -> Result<bool, Error> {
        let buf = fs::read(self.content_path()).await?;
        let buf = ByteBuf::from(buf);
        let content = Content::from_buf(buf).unwrap();
        self.modify_content(content);
        let res = Self::verify_content(self).await;
        Ok(res)
    }

    async fn check_file_integrity(path: impl AsRef<Path>, hash_str: String) -> Result<(), Error> {
        let mut file = File::open(path).await?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await?;
        let digest = Sha512::digest(buf);
        let hash = format!("{:x}", digest)[..64].to_string();
        if hash_str != hash {
            return Err(Error::Err("File integrity check failed".into()));
        }
        Ok(())
    }

    pub async fn check_site_integrity(&self) -> Result<(), Error> {
        let content = self.content().unwrap();
        let files = content.files;
        let mut tasks = Vec::new();
        for (inner_path, file) in files {
            let hash = file.sha512.clone();
            let task = Self::check_file_integrity(self.site_path().join(inner_path), hash);
            tasks.push(task);
        }
        let mut res = join_all(tasks).await;
        let errs = res.drain_filter(|res| res.is_err()).collect::<Vec<_>>();
        for err in &errs {
            error!("{:?}", err);
        }
        if errs.len() > 0 {
            return Err(Error::Err("Site integrity check failed".into()));
        }
        Ok(())
    }

    pub async fn fetch_changes(&self, since: usize) -> Result<HashMap<String, usize>, Error> {
        //TODO!: Download from multiple peers
        let mut peer = self.peers.values().next().unwrap().clone();
        let message = Protocol::new(peer.connection_mut().unwrap())
            .list_modified(self.address(), since)
            .await?;
        let changes = message.modified_files;
        Ok(changes)
    }

    pub async fn get_peers(&self) -> Result<Vec<Peer>, Error> {
        let mut peers = Vec::new();
        for peer in self.peers.values() {
            peers.push(peer.clone());
        }
        Ok(peers)
    }

    pub async fn fetch_peers(&mut self) -> Result<Vec<String>, Error> {
        let addr = (&self.address()).clone();
        let mut peer = self.peers.values().next().unwrap().clone();
        let res = Protocol::new((peer.connection_mut()).unwrap())
            .pex(addr.clone())
            .await?
            .peers
            .iter()
            .map(|bytes| {
                let pair = IpPort::from_bytes(&bytes.to_vec());
                pair.first().unwrap().to_string()
            })
            .collect::<Vec<_>>();
        Ok(res)
    }
}

#[async_trait::async_trait]
impl SiteIO for Site {
    fn site_path(&self) -> PathBuf {
        self.data_path.join(self.address())
    }

    fn content_path(&self) -> PathBuf {
        self.site_path().join("content.json")
    }

    async fn init_download(&mut self) -> Result<bool, Error> {
        if !&self.site_path().exists() {
            fs::create_dir_all(self.site_path()).await?;
        }
        let content_exists = self.content_path().exists();
        if !content_exists {
            Self::download_file(self, "content.json".into(), None).await?;
        }
        let verified = Self::load_content(&mut self).await?;
        if verified {
            let _ = self.download_site_files().await;
        }
        Ok(verified)
    }

    async fn load_settings(address: &str) -> Result<SiteSettings, Error> {
        let env = environment::get_env().unwrap();
        let sites_file_path = env.data_path.join("sites.json");
        if !sites_file_path.exists() {
            let mut file = File::create(&sites_file_path).await?;
            let mut settings = SiteSettings::default();
            if address == ENV.homepage {
                settings.permissions.push("ADMIN".to_string());
            }
            let site_file = SiteFile::default().from_site_settings(&settings);
            let s = json! {
                {
                    address: &site_file
                }
            };
            let content = format!("{:#}", s);
            file.write_all(content.as_bytes()).await?;
            Ok(settings)
        } else {
            let mut sites_file = OpenOptions::new().read(true).open(&sites_file_path)?;
            let mut content = String::new();
            sites_file.read_to_string(&mut content)?;
            let mut value: serde_json::Value = serde_json::from_str(&content)?;
            let site_settings = value[address].clone();
            let settings = if site_settings.is_null() {
                let set = SiteSettings::default();
                let site_file: SiteFile = SiteFile::default().from_site_settings(&set);
                value[address] = json! {
                    &site_file
                };
                let content = format!("{:#}", value);
                let mut sites_file = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&sites_file_path)?;
                sites_file.write_all(content.as_bytes())?;
                set
            } else {
                let v = value[address].clone();
                let res = serde_json::from_value(v);
                if let Err(e) = &res {
                    print!("{}", e);
                }
                let site_file: SiteFile = res.unwrap();
                site_file.site_settings()
            };

            Ok(settings)
        }
    }

    async fn save_settings(&self) -> Result<(), Error> {
        let env = environment::get_env().unwrap();
        let sites_file_path = env.data_path.join("sites.json");

        let mut sites_file = File::open(sites_file_path).await?;

        let site_settings = self.settings.clone();
        let content: String = serde_json::to_string(&site_settings)?;

        let mut full_content = String::new();
        sites_file.read_to_string(&mut full_content).await?;
        let _settings: serde_json::Value = serde_json::from_str(&content)?;

        write!(sites_file.into_std().await, "{}", content)?;
        Ok(())
    }
}

impl UserIO for User {
    type IOType = User;

    fn load() -> Result<Self::IOType, Error> {
        use std::fs::File;
        let start_time = SystemTime::now();
        let file_path = ENV.data_path.join("users.json");
        if !file_path.exists() {
            let mut file = File::create(&file_path)?;
            let user = User::new();
            let content = &format!("{:#}", json!({ &user.master_address: user }));
            file.write_all(content.as_bytes())?;
            Ok(user)
        } else {
            let mut file = File::open(&file_path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let value: HashMap<String, User> = serde_json::from_str(&content)?;
            let user_addr = value.keys().next().unwrap().clone();
            let user = value[&user_addr].clone();
            let end_time = SystemTime::now();
            let duration = end_time.duration_since(start_time).unwrap();
            info!("Loaded user in {} seconds", duration.as_secs());
            Ok(user)
        }
    }

    fn save(&self) -> Result<bool, Error> {
        use std::fs::File;
        let start_time = SystemTime::now();
        let file_path = ENV.data_path.join("users.json");
        let save_user = || -> Result<bool, Error> {
            let file = File::open(&file_path)?;
            use std::io::BufReader;
            let reader = BufReader::new(file);

            let mut users: HashMap<String, serde_json::Value> = serde_json::from_reader(reader)?;

            if !users.contains_key(&self.master_address) {
                users.insert(self.master_address.clone(), json!({})); // Create if not exist
            }

            let user = users.get_mut(&self.master_address).unwrap();
            user["master_seed"] = json!(self.get_master_seed());
            user["sites"] = json!(self.sites);
            user["certs"] = json!(self.certs);
            user["settings"] = json!(self.settings);

            let users_file_content_new = serde_json::to_string_pretty(&json!(users))?;
            let users_file_bytes = std::fs::read(&file_path)?;

            let result = atomic_write(
                &file_path,
                users_file_content_new.as_bytes(),
                &users_file_bytes,
                true,
            );
            result
        };

        if let Err(err_msg) = save_user() {
            error!("Couldn't save user: {:?}", err_msg);
            Err(err_msg)
        } else {
            debug!(
                "Saved in {}s",
                SystemTime::now()
                    .duration_since(start_time)
                    .unwrap()
                    .as_secs_f32()
            );
            Ok(true)
        }
    }
}
