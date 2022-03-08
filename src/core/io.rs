use std::path::PathBuf;

use zerucontent::Content;

use super::{error::Error, models::SiteSettings};

#[async_trait::async_trait]
pub trait SiteIO {
    fn site_path(&self) -> PathBuf;
    fn content_path(&self) -> PathBuf;
    // async fn content(self) -> Result<Content, Error>;
    // async fn content_exists(&self) -> Result<bool, Error>;
    async fn init_download(self) -> Result<bool, Error>;
    async fn load_settings(address: &str) -> Result<SiteSettings, Error>;
    async fn save_settings(&self) -> Result<(), Error>;
}

pub trait UserIO {
    type IOType;
    fn load() -> Result<Self::IOType, Error>;
    fn save(&self) -> Result<bool, Error>;
}
