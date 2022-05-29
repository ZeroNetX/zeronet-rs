use std::collections::HashMap;

use log::*;

use crate::{
    core::site::{models::SiteStorage, Site},
    environment::ENV,
    io::{db::DbManager, utils::current_unix_epoch},
};

pub struct SitesController {
    pub sites: HashMap<String, Site>,
    pub sites_changed: u64,
    pub db_manager: DbManager,
}

impl SitesController {
    pub fn new(db_manager: DbManager) -> Self {
        Self {
            db_manager,
            sites: HashMap::new(),
            sites_changed: current_unix_epoch(),
        }
    }

    pub fn add_site(&mut self, site: Site) {
        self.sites.insert(site.address(), site);
        self.update_sites_changed();
    }

    pub fn get_site(&self, site_addr: &str) -> Option<&Site> {
        self.sites.get(site_addr)
    }

    pub fn get_site_mut(&mut self, site_addr: &str) -> Option<&mut Site> {
        self.sites.get_mut(site_addr)
    }

    pub fn remove_site(&mut self, address: &str) {
        self.sites.remove(address);
        self.update_sites_changed();
    }

    pub async fn extend_sites_from_sitedata(&mut self, sites: HashMap<String, SiteStorage>) {
        for (address, site_storage) in sites {
            let path = ENV.data_path.join(&address);
            if path.exists() {
                let mut site = Site::new(&address, path).unwrap();
                site.modify_storage(site_storage);
                let res = site.load_content().await;
                if res.is_ok() {
                    self.sites.insert(address, site);
                } else {
                    //TODO! Start Downloading Site Content
                    error!(
                        "Failed to load site {}, Error: {:?}",
                        address,
                        res.unwrap_err()
                    );
                }
            } else {
                warn!("Site Dir with Address: {} not found", address);
            }
        }
        self.update_sites_changed();
    }

    pub fn extend_sites(&mut self, sites: HashMap<String, Site>) {
        self.sites.extend(sites);
        self.update_sites_changed();
    }

    fn update_sites_changed(&mut self) {
        self.sites_changed = current_unix_epoch();
    }
}
