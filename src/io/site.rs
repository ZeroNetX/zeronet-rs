use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
};

use futures::future::join_all;
use log::*;
use serde_bytes::ByteBuf;
use serde_json::json;
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

use zerucontent::Content;

use crate::{
    core::{error::*, io::*, models::*, peer::*, site::*},
    discovery::tracker::IpPort,
    environment::ENV,
    io::utils::check_file_integrity,
    net::protocol::Protocol,
};

impl Site {
    pub async fn create(&mut self, addr_idx: u32, private_key: &str) -> Result<(), Error> {
        let mut content = Content::create(self.address(), addr_idx);
        content.zeronet_version = ENV.version.clone();
        content.signs_required = 1;
        content.signers_sign =
            zeronet_cryptography::sign(format!("1:{}", self.address()), private_key)?;
        self.modify_content(content);
        self.add_file_data(private_key).await?;
        Ok(())
    }

    async fn add_file_data(&mut self, private_key: &str) -> Result<(), Error> {
        let data_dir = &*ENV.data_path;
        let site_dir = data_dir.join(&self.address());
        fs::create_dir_all(&site_dir).await?;
        let index_path = site_dir.join("index.html");
        let mut file = File::create(&index_path).await?;
        file.write_all(b"Welcome to World of DecentNet, A Peer to Peer Framework for Decentralised App and Services!")
            .await?;
        let _ = &self.add_file_to_content("index.html".into()).await?;

        self.sign_content(private_key).await?;
        self.save_content(None).await?;
        Ok(())
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
        if !parent.is_dir() {
            fs::create_dir_all(parent).await?;
        }
        let mut file = File::create(path).await?;
        file.write_all(&message.body).await?;
        Ok(true)
    }

    pub async fn need_file(&self, inner_path: String, _peer: Option<Peer>) -> Result<bool, Error> {
        self.download_file(inner_path, _peer).await
    }

    async fn download_file(&self, inner_path: String, _peer: Option<Peer>) -> Result<bool, Error> {
        let path = &self.site_path().join(&inner_path);
        if path.is_file() {
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
            .map(|path| self.load_content_from_path(path.clone()))
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
        let res = self.verify_content(true).await?;
        Ok(res)
    }

    pub async fn check_site_integrity(&self) -> Result<Vec<(String, zerucontent::File)>, Error> {
        let content = self.content().unwrap();
        let files = content.files;
        let mut tasks = Vec::new();
        for (inner_path, file) in files {
            let hash = file.sha512.clone();
            let task = check_file_integrity(self.site_path(), inner_path, hash);
            tasks.push(task);
        }
        //TODO!: Verify includes, user data files
        let mut res = join_all(tasks).await;
        let errs = res.drain_filter(|res| res.is_err()).collect::<Vec<_>>();
        for err in &errs {
            println!("{:?}", err);
        }
        if !errs.is_empty() {
            return Err(Error::Err("Site integrity check failed".into()));
        }
        let res = res
            .iter_mut()
            .filter_map(|r| {
                let r = &(r);
                if let Ok(r) = r {
                    let (v, i, h) = r;
                    if *v {
                        None
                    } else {
                        Some((i.clone(), h.clone()))
                    }
                } else {
                    unreachable!()
                }
            })
            .collect::<Vec<_>>();
        Ok(res)
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
                let pair = IpPort::from_bytes(bytes.as_ref());
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
        if !&self.site_path().is_dir() {
            fs::create_dir_all(self.site_path()).await?;
        }
        let content_exists = self.content_path().is_file();
        if !content_exists {
            Self::download_file(self, "content.json".into(), None).await?;
        }
        let verified = self.load_content().await?;
        if verified {
            let _ = self.download_site_files().await;
        }
        self.verify_content(false).await?;
        Ok(verified)
    }

    async fn load_settings(address: &str) -> Result<SiteSettings, Error> {
        let env = &*ENV;
        let sites_file_path = env.data_path.join("sites.json");
        if !sites_file_path.exists() {
            let mut file = File::create(&sites_file_path).await?;
            let settings = SiteSettings::default();
            // let mut settings = SiteSettings::default();
            // if address == ENV.homepage {
            //     settings.permissions.push("ADMIN".to_string());
            // }
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
            let mut sites_file = OpenOptions::new().read(true).open(&sites_file_path).await?;
            let mut content = String::new();
            sites_file.read_to_string(&mut content).await?;
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
                    .open(&sites_file_path)
                    .await?;
                sites_file.write_all(content.as_bytes()).await?;
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
        let env = &*ENV;
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
