use std::path::PathBuf;

use zerucontent::Content;

use super::error::Error;

#[async_trait::async_trait]
pub trait SiteIO {
    fn site_path(&self) -> PathBuf;
    fn content_path(&self) -> PathBuf;
    fn get_path(&self, inner_path: &str) -> Result<PathBuf, Error>;
    fn get_inner_path(&self, path: &str) -> Result<PathBuf, Error>;
    // async fn content_exists(&self) -> Result<bool, Error>;
    async fn init_download(&mut self) -> Result<bool, Error>;
    async fn load_storage(path: &str) -> Result<bool, Error>;
    async fn save_storage(&self) -> Result<bool, Error>;
}

#[async_trait::async_trait]
pub trait UserIO {
    type IOType;
    async fn load() -> Result<Self::IOType, Error>;
    async fn save(&self) -> Result<bool, Error>;
}

#[async_trait::async_trait]
pub trait ContentMod {
    async fn load_content_from_path(&self, inner_path: String) -> Result<Content, Error>;
    async fn add_file_to_content(&mut self, path: PathBuf) -> Result<(), Error>;
    async fn sign_content(
        &mut self,
        inner_path: Option<&str>,
        private_key: &str,
    ) -> Result<(), Error>;
    fn verify_content(&self, inner_path: Option<&str>) -> Result<(), Error>;
    async fn save_content(&mut self, inner_path: Option<&str>) -> Result<(), Error>;
}
