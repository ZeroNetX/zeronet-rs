use super::{address::Address as Addr, error::Error, models::SiteSettings, peer::Peer};
use chrono::Utc;
use log::error;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    str::FromStr,
};
use zerucontent::Content;

pub struct Site {
    address: Addr,
    pub peers: HashMap<String, Peer>,
    pub settings: SiteSettings,
    pub data_path: PathBuf,
    content: Option<Content>,
}

impl Site {
    pub fn new(address: &str, data_path: PathBuf) -> Result<Self, Error> {
        let mut settings = SiteSettings::default();
        Ok(Self {
            address: Addr::from_str(address)?,
            peers: HashMap::new(),
            data_path,
            settings,
            content: None,
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

    pub async fn verify_content(&self, content_only: bool) -> Result<bool, Error> {
        if self.content.is_none() {
            Err(Error::Err("No content to verify".into()))
        } else {
            if !content_only {
                self.check_site_integrity().await?;
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
