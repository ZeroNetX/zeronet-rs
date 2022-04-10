use std::path::PathBuf;

use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};
use zerucontent::Content;

use crate::{
    core::{error::*, io::*, site::*},
    io::utils::get_zfile_info,
};

use super::utils::current_unix_epoch;

#[async_trait::async_trait]
impl ContentMod for Site {
    async fn load_content_from_path(&self, inner_path: String) -> Result<Content, Error> {
        let path = &self.site_path().join(&inner_path);
        if path.is_file() {
            let mut file = File::open(path).await?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).await?;
            let content: Content = serde_json::from_slice(&buf)?;
            return Ok(content);
        }
        Err(Error::Err("Content File Not Found".into()))
    }

    async fn add_file_to_content(&mut self, inner_path: PathBuf) -> Result<(), Error> {
        let path = self.site_path().join(&inner_path);
        if path.is_file() {
            let file = get_zfile_info(path).await?;
            let res = &mut self.content_mut(None).unwrap().files;
            res.insert(inner_path.display().to_string(), file);
            Ok(())
        } else {
            return Err(Error::Err("File does not exist".into()));
        }
    }

    async fn sign_content(
        &mut self,
        inner_path: Option<&str>,
        private_key: &str,
    ) -> Result<(), Error> {
        let content = self.content_mut(inner_path).unwrap();
        content.modified = current_unix_epoch() as usize;
        let sign = content.sign(private_key.to_string());
        let address = zeronet_cryptography::privkey_to_pubkey(private_key)?;
        content.signs.insert(address, sign);
        Ok(())
    }

    async fn save_content(&mut self, inner_path: Option<&str>) -> Result<(), Error> {
        let content = self.content(inner_path).unwrap();
        let content_json = serde_json::to_string_pretty(&content)?;
        let inner_path = inner_path.unwrap_or("content.json");
        let path = self.site_path().join(inner_path);
        if path.exists() {
            fs::remove_file(&path).await?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&path)
            .await?;
        file.write_all(content_json.as_bytes()).await?;
        Ok(())
    }
}
