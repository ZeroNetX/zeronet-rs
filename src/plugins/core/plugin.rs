use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::permission::Permission;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plugin {
    pub name: String,
    pub description: String,
    pub version: String,
    pub revision: i64,
    pub permissions: Vec<Permission>,
    #[serde(skip)]
    pub path: PathBuf,
}

impl Default for Plugin {
    fn default() -> Self {
        Plugin {
            version: "0.0.1".into(),
            revision: 1,
            name: Default::default(),
            description: Default::default(),
            path: Default::default(),
            permissions: Default::default(),
        }
    }
}
