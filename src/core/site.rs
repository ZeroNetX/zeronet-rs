use log::error;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    str::FromStr,
};
use zerucontent::Content;

use crate::environment::ENV;

use self::models::SiteStorage;
use super::{address::Address as Addr, error::Error, peer::Peer};

pub mod models {
    use serde::{Deserialize, Serialize};
    use std::collections::{BTreeMap, HashMap};

    use crate::utils::is_default;
    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct SiteInfo {
        pub auth_address: String,
        pub cert_user_id: Option<String>,
        pub address: String,
        pub address_short: String,
        pub address_hash: String,
        pub settings: SiteStorage,
        pub content_updated: f64,
        pub bad_files: usize,
        pub size_limit: usize,
        pub next_size_limit: usize,
        pub peers: usize,
        pub started_task_num: usize,
        pub tasks: usize,
        pub workers: usize,
        pub content: serde_json::Value,
        pub privatekey: bool,
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct SiteStorage {
        #[serde(flatten)]
        pub keys: SiteKeys,
        #[serde(flatten)]
        pub stats: SiteStats,
        #[serde(flatten)]
        pub settings: SiteSettings,
        pub cache: SiteCache,
        #[serde(skip_serializing, skip_deserializing)]
        pub plugin_storage: SitePluginStorage,
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    pub struct SiteCache {
        #[serde(default, skip_serializing_if = "is_default")]
        pub bad_files: BTreeMap<String, usize>,
        #[serde(default, skip_serializing_if = "is_default")]
        pub hashfield: String,
        #[serde(default, skip_serializing_if = "is_default")]
        pub piecefields: BTreeMap<String, String>,
    }

    #[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
    pub struct SitePluginStorage {
        #[serde(default, skip_serializing_if = "is_default")]
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

    pub fn addr(&self) -> Addr {
        self.address.clone()
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

    pub fn get_size(&self) -> usize {
        self.storage.stats.size
    }

    pub fn get_size_limit(&self) -> usize {
        let size_limit = self.storage.settings.size_limit;
        if size_limit == 0 {
            ENV.size_limit
        } else {
            size_limit
        }
    }

    pub fn get_next_size_limit(&self) -> usize {
        let size_limits: [i32; 12] = [
            25, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000,
        ];
        let size = (self.get_size() as f32 * 1.2) as i32;
        let limit = size_limits
            .iter()
            .find(|&&x| x * 1024 * 1024 > size)
            .unwrap_or(&999999);
        *limit as usize
    }
}
