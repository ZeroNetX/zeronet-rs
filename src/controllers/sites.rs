use std::collections::HashMap;

use crate::{
    core::site::{models::SiteStorage, Site},
    environment::ENV,
    io::utils::current_unix_epoch,
};

pub struct SitesController {
    pub sites: HashMap<String, Site>,
    pub sites_changed: u64,
}

impl Default for SitesController {
    fn default() -> Self {
        SitesController {
            sites: HashMap::new(),
            sites_changed: current_unix_epoch(),
        }
    }
}

impl SitesController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_site(&mut self, site: Site) {
        self.sites.insert(site.address(), site);
        self.update_sites_changed();
    }

    pub fn remove_site(&mut self, address: &str) {
        self.sites.remove(address);
        self.update_sites_changed();
    }

    pub fn extend_sites_from_sitedata(&mut self, sites: HashMap<String, SiteStorage>) {
        for (address, site_storage) in sites {
            let path = ENV.data_path.join(&address);
            let mut site = Site::new(&address, path).unwrap();
            site.modify_storage(site_storage);
            self.sites.insert(address.to_string(), site);
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
