use self::models::SiteStorage;
use super::{address::Address as Addr, error::Error, peer::Peer};
use log::error;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    str::FromStr,
};
use zerucontent::Content;

pub mod models {
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

#[derive(Clone)]
pub struct Site {
    address: Addr,
    pub peers: HashMap<String, Peer>,
    pub data_path: PathBuf,
    pub storage: SiteStorage,
    content: HashMap<String, Content>,
}

impl Site {
    pub fn new(address: &str, data_path: PathBuf) -> Result<Self, Error> {
        Ok(Self {
            address: Addr::from_str(address)?,
            peers: HashMap::new(),
            data_path,
            content: HashMap::new(),
            storage: SiteStorage::default(),
        })
    }

    pub fn address(&self) -> String {
        self.address.address.clone()
    }

    pub fn content_exists(&self) -> bool {
        self.content.contains_key("content.json")
    }

    pub fn inner_content_exists(&self, inner_path: &str) -> bool {
        self.content.contains_key(inner_path)
    }

    pub fn content(&self, inner_path: Option<&str>) -> Option<Content> {
        self.content
            .get(inner_path.unwrap_or("content.json"))
            .cloned()
    }

    pub fn content_mut(&mut self, inner_path: Option<&str>) -> Option<&mut Content> {
        self.content.get_mut(inner_path.unwrap_or("content.json"))
    }

    pub fn modify_content(&mut self, inner_path: Option<&str>, content: Content) {
        self.content
            .insert(inner_path.unwrap_or("content.json").into(), content);
    }

    pub fn modify_storage(&mut self, storage: SiteStorage) {
        self.storage = storage;
    }
}
