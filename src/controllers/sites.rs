use std::collections::HashMap;

use actix::{Actor, Addr};
use futures::executor::block_on;
use log::*;
use rusqlite::{params, Connection};
use serde_json::{Map, Value};

use crate::{
    core::{
        address::Address,
        error::Error,
        site::{models::SiteStorage, Site},
    },
    environment::{ENV, SITE_STORAGE},
    io::{db::DbManager, utils::current_unix_epoch},
    utils::to_json_value,
};

pub async fn run() -> Result<Addr<SitesController>, Error> {
    info!("Starting Site Controller.");
    let db_manager = DbManager::new();
    let mut site_controller = SitesController::new(db_manager);
    let site_storage = &*SITE_STORAGE;
    site_controller
        .extend_sites_from_sitedata(site_storage.clone())
        .await;
    let site_controller_addr = site_controller.start();
    Ok(site_controller_addr)
}

pub struct SitesController {
    pub sites: HashMap<String, Site>,
    pub sites_addr: HashMap<Address, Addr<Site>>,
    pub nonce: HashMap<String, Address>,
    pub sites_changed: u64,
    pub db_manager: DbManager,
}

impl SitesController {
    pub fn new(db_manager: DbManager) -> Self {
        Self {
            db_manager,
            sites: HashMap::new(),
            sites_addr: HashMap::new(),
            nonce: HashMap::new(),
            sites_changed: current_unix_epoch(),
        }
    }

    pub fn get(&mut self, address: Address) -> Result<(Address, Addr<Site>), Error> {
        if let Some(addr) = self.sites_addr.get(&address) {
            Ok((address, addr.clone()))
        } else {
            info!(
                "Spinning up actor for site zero://{}",
                address.get_address_short()
            );
            let mut site = Site::new(
                &address.address,
                ENV.data_path.clone().join(&address.address),
            )?;
            block_on(site.load_content())?;
            let site_storage = &*SITE_STORAGE;
            let wrapper_key = site_storage
                .get(&address.address)
                .unwrap()
                .keys
                .wrapper_key
                .clone();
            if !wrapper_key.is_empty() {
                self.nonce.insert(wrapper_key, address.clone());
            }
            if let Some(schema) = self.db_manager.load_schema(&site.address()) {
                self.db_manager.insert_schema(&site.address(), schema);
                self.db_manager.connect_db(&site.address())?;
            }
            let addr = site.start();
            // TODO: Decide whether to spawn actors in syncArbiter
            // let addr = SyncArbiter::start(1, || Site::new());
            self.sites_addr.insert(address.clone(), addr.clone());
            self.update_sites_changed();

            Ok((address, addr))
        }
    }

    pub fn get_by_key(&mut self, key: String) -> Result<(Address, Addr<Site>), Error> {
        if let Some(address) = self.nonce.get(&key) {
            if let Some(addr) = self.sites_addr.get(address) {
                return Ok((address.clone(), addr.clone()));
            }
        }
        error!("No site found for key {}", key);
        Err(Error::MissingError)
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

impl SitesController {
    pub async fn db_query(
        conn: &mut Connection,
        query: &str,
    ) -> Result<Vec<Map<String, Value>>, Error> {
        let mut stmt = conn.prepare(query).unwrap();
        let count = stmt.column_count();
        let names = {
            stmt.column_names()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        };
        let res = stmt
            .query_map(params![], |row| {
                let mut data_map = Map::new();
                let mut i = 0;
                loop {
                    while let Ok(value) = row.get::<_, rusqlite::types::Value>(i) {
                        let name = names.get(i).unwrap().to_string();
                        i += 1;
                        let value = to_json_value(&value);
                        data_map.insert(name, value);
                    }
                    if i == count {
                        break;
                    }
                }
                Ok(data_map)
            })
            .unwrap();
        let res = res.filter_map(|e| e.ok()).collect::<Vec<_>>();
        Ok(res)
    }
}
