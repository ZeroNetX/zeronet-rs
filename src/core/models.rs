use chrono::Utc;
use serde::{Deserialize, Serialize};

use std::collections::{BTreeMap, HashMap};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AuthPair {
    pub auth_address: String,
    auth_privkey: String,
}

impl AuthPair {
    pub fn new(auth_address: String, auth_privkey: String) -> Self {
        AuthPair {
            auth_address,
            auth_privkey,
        }
    }

    pub fn get_auth_privkey(&self) -> &str {
        &self.auth_privkey
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Cert {
    auth_pair: AuthPair,
    pub auth_type: String,
    pub auth_user_name: String,
    cert_sign: String,
}

impl Cert {
    pub fn new(
        auth_pair: AuthPair,
        auth_type: String,
        auth_user_name: String,
        cert_sign: String,
    ) -> Self {
        Cert {
            auth_pair,
            auth_type,
            auth_user_name,
            cert_sign,
        }
    }

    pub fn get_auth_pair(&self) -> AuthPair {
        self.auth_pair.clone()
    }

    pub fn get_cert_sign(&self) -> &str {
        &self.cert_sign
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SiteData {
    #[serde(skip_serializing)]
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cert_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auth_pair: Option<AuthPair>,
    #[serde(skip_serializing_if = "Option::is_none")]
    privkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    settings: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    plugin_data: HashMap<String, serde_json::Value>,
}

impl SiteData {
    pub fn new(address: String) -> Self {
        SiteData {
            address,
            index: None,
            cert_provider: None,
            auth_pair: None,
            privkey: None,
            settings: None,
            plugin_data: HashMap::new(),
        }
    }

    pub fn create(address: String, index: u32, auth_pair: AuthPair, privkey: String) -> Self {
        SiteData {
            address,
            index: Some(index),
            cert_provider: None,
            auth_pair: Some(auth_pair),
            privkey: Some(privkey),
            settings: None,
            plugin_data: HashMap::new(),
        }
    }

    pub fn with_index(&mut self, index: u32) -> Self {
        self.index = Some(index);
        self.to_owned()
    }

    pub fn get_index(&self) -> Option<u32> {
        self.index.clone()
    }

    pub fn get_cert_provider(&self) -> Option<String> {
        self.cert_provider.clone()
    }

    pub fn add_cert_provider(&mut self, cert_provider: String) {
        self.cert_provider = Some(cert_provider);
    }

    pub fn delete_cert_provider(&mut self) {
        self.cert_provider = None;
    }

    pub fn with_auth_pair(&mut self, auth_pair: AuthPair) -> Self {
        self.auth_pair = Some(auth_pair);
        self.to_owned()
    }

    pub fn get_auth_pair(&self) -> Option<AuthPair> {
        self.auth_pair.clone()
    }

    pub fn with_privkey(&mut self, priv_key: String) -> Self {
        self.privkey = Some(priv_key);
        self.to_owned()
    }

    pub fn get_privkey(&self) -> Option<String> {
        self.privkey.clone()
    }

    pub fn get_settings(&self) -> Option<serde_json::Value> {
        self.settings.clone()
    }

    pub fn set_settings(&mut self, settings: serde_json::Value) -> Self {
        self.settings = Some(settings);
        self.to_owned()
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SiteSettings {
    pub peers: usize,
    pub serving: bool,
    pub modified: f64,
    pub own: bool,
    pub permissions: Vec<String>,
    pub size: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SiteFile {
    added: usize,
    ajax_key: String,
    auth_key: String,
    bytes_recv: usize,
    bytes_sent: usize,
    cache: SiteCache,
    downloaded: usize,
    modified: f64,
    optional_downloaded: usize,
    own: bool,
    peers: usize,
    permissions: Vec<String>,
    serving: bool,
    size: usize,
    size_files_optional: usize,
    size_optional: usize,
    wrapper_key: String,
}

impl Default for SiteFile {
    fn default() -> SiteFile {
        SiteFile {
            added: Utc::now().timestamp() as usize,
            ajax_key: String::default(),
            auth_key: String::default(),
            bytes_recv: 0,
            bytes_sent: 0,
            cache: SiteCache::default(),
            downloaded: 0,
            modified: 0.0,
            optional_downloaded: 0,
            own: false,
            peers: 0,
            permissions: vec![],
            serving: true,
            size: 0,
            size_files_optional: 0,
            size_optional: 0,
            wrapper_key: String::default(),
        }
    }
}

impl SiteFile {
    pub fn site_settings(self) -> SiteSettings {
        SiteSettings {
            peers: self.peers,
            serving: self.serving,
            modified: self.modified,
            own: self.own,
            permissions: self.permissions,
            size: self.size,
        }
    }

    pub fn from_site_settings(self, settings: &SiteSettings) -> Self {
        Self {
            peers: settings.peers,
            serving: settings.serving,
            modified: settings.modified,
            own: settings.own,
            permissions: settings.permissions.clone(),
            size: settings.size,
            ..self
        }
    }

    pub fn update_wrapper_key(&mut self, wrapper_key: String) {
        self.wrapper_key = wrapper_key;
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SiteCache {
    bad_files: BTreeMap<String, usize>,
    hashfield: String,
}
