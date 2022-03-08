use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{BufReader, Read, Write},
    path::PathBuf,
    time::SystemTime,
};

use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    core::{error::*, io::*, models::*, site::*, user::*},
    environment::{self, ENV},
    utils::atomic_write,
};
use log::*;
use serde_json::json;
use zeronet_protocol::templates;

impl Site {
    async fn download_file(&self, inner_path: String) -> Result<bool, Error> {
        let req = json!(templates::GetFile {
            site: self.address(),
            inner_path: inner_path.clone(),
            location: 0,
            file_size: 0,
        });
        let mut peer = self.peers.values().next().unwrap().clone();
        let message = peer.connection_mut().unwrap().request("getFile", req).await;
        let body: templates::GetFileResponse = message.unwrap().body().unwrap();

        let mut file = File::create(&self.site_path().join(inner_path)).await?;
        file.write_all(&body.body).await?;
        Ok(true)
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

    async fn init_download(self) -> Result<bool, Error> {
        if !&self.site_path().exists() {
            fs::create_dir_all(&self.site_path())?;
        }
        let content_exists = self.content_path().exists();
        if !content_exists {
            Self::download_file(&self, "content.json".into()).await?;
            let res = Self::verify_content(&self).await;
            return Ok(res);
        } else {
            let res = Self::verify_content(&self).await;
            return Ok(res);
        }
    }

    async fn load_settings(address: &str) -> Result<SiteSettings, Error> {
        let env = environment::get_env().unwrap();
        let sites_file_path = env.data_path.join("sites.json");
        if !sites_file_path.exists() {
            let mut file = File::create(&sites_file_path).await?;
            let mut settings = SiteSettings::default();
            if address == &ENV.homepage {
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
                let site_settings = site_file.site_settings();
                site_settings
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

            let reader = BufReader::new(file);

            let mut users: HashMap<String, serde_json::Value> = serde_json::from_reader(reader)?;

            if users.contains_key(&self.master_address) == false {
                users.insert(self.master_address.clone(), json!({})); // Create if not exist
            }

            let user = users.get_mut(&self.master_address).unwrap();
            user["master_seed"] = json!(self.get_master_seed());
            user["sites"] = json!(self.sites);
            user["certs"] = json!(self.certs);
            user["settings"] = json!(self.settings);

            let users_file_content_new = serde_json::to_string_pretty(&json!(users))?;
            let users_file_bytes = fs::read(&file_path)?;

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
            return Err(err_msg);
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
