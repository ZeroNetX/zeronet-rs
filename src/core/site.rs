use self::models::SiteStorage;
use super::{address::Address as Addr, error::Error, peer::Peer};
use chrono::Utc;
use log::error;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    str::FromStr,
};
use zerucontent::Content;

pub mod models {
    use chrono::Utc;
    use serde::{Deserialize, Serialize};
    use std::collections::{BTreeMap, HashMap};

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct SiteStorage {
        pub keys: SiteKeys,
        pub stats: SiteStats,
        pub settings: SiteSettings,
        pub cache: SiteCache,
        pub plugin_storage: SitePluginStorage,
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct SiteCache {
        pub bad_files: BTreeMap<String, usize>,
        pub hashfield: String,
        pub piecefields: BTreeMap<String, String>,
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct SitePluginStorage {
        pub data: HashMap<String, serde_json::Value>,
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct SiteStats {
        pub added: usize,
        pub bytes_recv: usize,
        pub bytes_sent: usize,
        pub downloaded: usize,
        pub modified: usize,
        pub size: usize,
        pub size_optional: usize,
        pub own: bool,
        pub modified_files_notification: bool,
        pub peers: usize,
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct SiteSettings {
        pub serving: bool,
        pub own: bool,
        pub permissions: Vec<String>,
        pub size_limit: usize,
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct SiteKeys {
        pub wrapper_key: String,
        pub ajax_key: String,
    }
}

pub struct Site {
    address: Addr,
    pub peers: HashMap<String, Peer>,
    pub data_path: PathBuf,
    pub storage: SiteStorage,
    content: Option<Content>,
}

impl Site {
    pub fn new(address: &str, data_path: PathBuf) -> Result<Self, Error> {
        Ok(Self {
            address: Addr::from_str(address)?,
            peers: HashMap::new(),
            data_path,
            content: None,
            storage: SiteStorage::default(),
        })
    }

    pub fn address(&self) -> String {
        self.address.address.clone()
    }

    fn content_exists(&self) -> bool {
        self.content.is_some()
    }

    pub fn content(&self) -> Option<Content> {
        self.content.clone()
    }

    pub fn content_mut(&mut self) -> Option<&mut Content> {
        self.content.as_mut()
    }

    pub fn modify_content(&mut self, content: Content) {
        self.content = Some(content);
    }

    pub fn modify_storage(&mut self, storage: SiteStorage) {
        self.storage = storage;
    }

    pub async fn verify_content(&self, content_only: bool) -> Result<bool, Error> {
        if self.content.is_none() {
            Err(Error::Err("No content to verify".into()))
        } else {
            if !content_only {
                let res = self.check_site_integrity().await?;
                if !res.is_empty() {
                    return Err(Error::Err(format!(
                        "Site Integrity Check Failed: {:?}",
                        res
                    )));
                }
            }
            let content = self.content.clone().unwrap();
            let verified = content.verify((&self.address()).clone());
            if !verified {
                return Err(Error::Err(format!(
                    "Content verification failed for {}",
                    self.address()
                )));
            } else {
                Ok(verified)
            }
        }
    }
}
