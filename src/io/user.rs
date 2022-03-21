use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::SystemTime,
};

use log::*;
use serde_json::json;
use tokio::{
    fs::{read, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    core::{error::Error, io::UserIO, user::User},
    environment::ENV,
};

#[async_trait::async_trait]
impl UserIO for User {
    type IOType = User;

    async fn load() -> Result<Self::IOType, Error> {
        let start_time = SystemTime::now();
        let file_path = ENV.data_path.join("users.json");
        if !file_path.is_file() {
            let mut file = File::create(&file_path).await?;
            let user = User::new();
            let content = &format!("{:#}", json!({ &user.master_address: user }));
            file.write_all(content.as_bytes()).await?;
            Ok(user)
        } else {
            let mut file = File::open(&file_path).await?;
            let mut content = String::new();
            file.read_to_string(&mut content).await?;
            let value: HashMap<String, User> = serde_json::from_str(&content)?;
            let user_addr = value.keys().next().unwrap().clone();
            let user = value[&user_addr].clone();
            let end_time = SystemTime::now();
            let duration = end_time.duration_since(start_time).unwrap();
            info!("Loaded user in {} seconds", duration.as_secs());
            Ok(user)
        }
    }

    async fn save(&self) -> Result<bool, Error> {
        let start_time = SystemTime::now();
        let file_path = ENV.data_path.join("users.json");

        if let Err(err_msg) = self.save_user(file_path).await {
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

impl User {
    async fn save_user(&self, file_path: PathBuf) -> Result<bool, Error> {
        let bytes = read(&file_path).await?;
        let mut users: HashMap<String, serde_json::Value> = serde_json::from_slice(&bytes)?;
        if !users.contains_key(&self.master_address) {
            users.insert(self.master_address.clone(), json!({})); // Create if not exist
        }

        let user = users.get_mut(&self.master_address).unwrap();
        user["master_seed"] = json!(self.get_master_seed());
        user["sites"] = json!(self.sites);
        user["certs"] = json!(self.certs);
        user["settings"] = json!(self.settings);

        let users_file_content_new = serde_json::to_string_pretty(&json!(users))?;
        let users_file_bytes = read(&file_path).await?;

        let result = Self::write_to_disk(
            &file_path,
            users_file_content_new.as_bytes(),
            &users_file_bytes,
            true,
        )
        .await;
        result
    }

    #[async_recursion::async_recursion]
    pub async fn write_to_disk(
        dest: &Path,
        new_content: &[u8],
        content: &[u8],
        retry: bool,
    ) -> Result<bool, Error> {
        let mut options = OpenOptions::new();
        options.write(true).truncate(true);
        let mut file = options.open(dest).await?;
        if let Err(e) = file.write_all(new_content).await {
            error!("Error writing file: {:?}", e);
            //Possible data corruption in old file, overwrite with old content
            if retry {
                return Self::write_to_disk(dest, new_content, content, false).await;
            } else {
                file.write_all(content).await?
            }
        }
        Ok(true)
    }
}
