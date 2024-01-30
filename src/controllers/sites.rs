use std::collections::HashMap;

use actix::{Actor, Addr};
use futures::executor::block_on;
use itertools::Itertools;
use log::*;
use regex::Regex;
use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};

use crate::{
    core::{
        address::Address,
        error::Error,
        io::SiteIO,
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
    for site in site_storage.keys().clone() {
        if let Some(addr) = site_controller.get_site_addr(site).cloned() {
            site_controller.get(&addr)?;
        }
    }
    let site_controller_addr = site_controller.start();
    Ok(site_controller_addr)
}

pub struct SitesController {
    pub sites: HashMap<String, Site>,
    pub sites_addr: HashMap<Address, Addr<Site>>,
    pub ajax_keys: HashMap<String, Address>,
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
            ajax_keys: HashMap::new(),
            nonce: HashMap::new(),
            sites_changed: current_unix_epoch(),
        }
    }

    pub fn get(&mut self, address: &Address) -> Result<(Address, Addr<Site>), Error> {
        let address_str = address.address.clone();
        let mut site;
        let site = if let Some(site) = self.sites.get_mut(&address_str) {
            site
        } else {
            site = Site::new(&address_str, ENV.data_path.join(address_str.clone())).unwrap();
            &mut site
        };
        if let Some(addr) = self.sites_addr.get(&address) {
            if site.content_path().is_file() {
                return Ok((address.clone(), addr.clone()));
            }
        }
        trace!(
            "Spinning up actor for site zero://{}",
            address.get_address_short()
        );
        if !site.content_path().is_file() {
            // info!("Site content does not exist. Downloading...");
            error!("\n\n\nSite content does not exist, Site Download from UiServer not implemented yet, Use siteDownload cmd via cli to download site\n\n\n");
            unimplemented!();
        } else {
            site.modify_storage(site.storage.clone());
            block_on(site.load_content())?;
            if let Some(site_storage) = (*SITE_STORAGE).get(&address.address) {
                let wrapper_key = site_storage.keys.wrapper_key.clone();
                if !wrapper_key.is_empty() {
                    self.nonce.insert(wrapper_key, address.clone());
                }
            }
            if let Some(schema) = self.db_manager.load_schema(&site.address()) {
                self.db_manager.insert_schema(&site.address(), schema);
                self.db_manager.connect_db(&site.address())?;
            }
            self.sites_changed = current_unix_epoch();
        }

        // TODO: Decide whether to spawn actors in syncArbiter
        let addr = site.clone().start();
        self.sites_addr.insert(address.clone(), addr.clone());
        Ok((address.clone(), addr))
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
        self.sites.insert(site.address().into(), site);
        self.update_sites_changed();
    }

    pub fn get_site(&self, site_addr: &str) -> Option<&Site> {
        self.sites.get(site_addr)
    }

    pub fn get_site_addr(&self, site_addr: &str) -> Option<&Address> {
        self.sites.get(site_addr).map(|site| site.addr())
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
                site.modify_storage(site_storage.clone());
                let res = site.load_content().await;
                if res.is_ok() {
                    self.sites.insert(address, site.clone());
                    self.nonce
                        .insert(site_storage.keys.wrapper_key, site.addr().clone());
                    self.ajax_keys
                        .insert(site_storage.keys.ajax_key, site.addr().clone());
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
        params: Option<Value>,
    ) -> Result<Vec<Map<String, Value>>, Error> {
        let (query, params) = if let Some(params) = params {
            Self::parse_query(query, params)
        } else {
            (query.to_string(), None)
        };
        let has_params = params.is_some();
        let mut stmt = conn.prepare(&query).unwrap();
        let count = stmt.column_count();
        let names = {
            stmt.column_names()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
        };
        let res = if has_params {
            stmt.query(params![params])
        } else {
            stmt.query(params![])
        };
        let res = res?.mapped(|row| {
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
        });
        let res = res.filter_map(|e| e.ok()).collect::<Vec<_>>();
        Ok(res)
    }

    pub fn parse_query(query: &str, params: Value) -> (String, Option<Value>) {
        if !params.is_object() {
            return (query.to_string(), Some(params));
        } else if !query.contains('?')
            && !query.contains(':')
            && params.as_object().unwrap().is_empty()
        {
            let query_types = ["SELECT", "INSERT", "UPDATE", "DELETE"];
            let query_type = query.split_whitespace().next().unwrap().to_uppercase();
            if query_types.contains(&query_type.as_str()) {
                return (query.to_string(), None);
            }
        }
        let params = params.as_object().unwrap();
        let query_type = query.split_whitespace().next().unwrap().to_uppercase();
        let mut new_query = String::from(query);
        let mut new_params = vec![];
        if query.contains('?') {
            let query_types = ["SELECT", "DELETE", "UPDATE"];
            if query_types.contains(&query_type.as_str()) {
                let mut query_wheres = vec![];
                let mut values = vec![];
                for (key, value) in params.iter() {
                    if let Value::Array(value) = value {
                        let operator = if key.starts_with("not__") {
                            "NOT IN"
                        } else {
                            "IN"
                        };
                        let field = if key.starts_with("not__") {
                            key.strip_prefix("not__").unwrap()
                        } else {
                            key
                        };
                        let query_values = if value.len() > 100 {
                            let s = value
                                .iter()
                                .map(|value| Self::sqlquote(value.clone()))
                                .join(", ");
                            s
                        } else {
                            let placeholders = vec!["?"; value.len()].join(", ");
                            new_params.extend(value.iter().map(|v| v.to_string()));
                            values.extend(value.iter().cloned());
                            placeholders
                        };
                        query_wheres.push(format!("{} {} ({})", field, operator, query_values));
                    } else {
                        let (key, operator) = if key.starts_with("not__") {
                            (key.replace("not__", ""), "!=")
                        } else if key.ends_with("__like") {
                            (key.replace("__like", ""), "LIKE")
                        } else if key.ends_with('>') {
                            (key.replace('>', ""), ">")
                        } else if key.ends_with('<') {
                            (key.replace('<', ""), "<")
                        } else {
                            (key.to_string(), "=")
                        };
                        query_wheres.push(format!("{key} {operator} ?"));
                        values.push(value.clone());
                        new_params.push(value.to_string());
                    }
                }

                let wheres = if query_wheres.is_empty() {
                    String::from("1")
                } else {
                    query_wheres.join(" AND ")
                };

                let re = Regex::new(r"(.*)[?]").unwrap();
                let wheres = format!("$1 {}", wheres);

                new_query = re.replace(&query, &wheres).into();
            } else {
                let keys = params
                    .keys()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                let values = vec!["?"; params.len()].join(", ");
                let keysvalues = format!("({}) VALUES ({})", keys, values);

                let re = Regex::new(r"\?").unwrap();
                new_query = re.replace_all(&new_query, &keysvalues).to_string();
                new_params = params.values().map(|v| v.to_string()).collect();
            }
            let new_params = if new_params.is_empty() {
                None
            } else {
                Some(json!(new_params))
            };
            (new_query, new_params)
        } else if query.contains(':') {
            let mut new_params_map = Map::new();
            for (key, value) in params {
                if let Value::Array(value) = value {
                    for (idx, val) in value.iter().enumerate() {
                        new_params_map.insert(format!("{}__{}", key, idx), val.clone());
                    }
                    let new_names = (0..value.len())
                        .map(|idx| format!(":{}__{}", key, idx))
                        .collect::<Vec<String>>();
                    let key = regex::escape(&key);
                    let re = Regex::new(&format!(r":{}([)\s]|$)", key)).unwrap();
                    let replacement = format!("({})$1", new_names.join(", "));
                    new_query = re.replace_all(&query, replacement.as_str()).into();
                } else {
                    new_params_map.insert(key.to_string(), value.clone());
                }
            }
            let new_params_map = if new_params_map.is_empty() {
                None
            } else {
                Some(json!(new_params_map))
            };

            (new_query, new_params_map)
        } else {
            unreachable!("Unknown query format");
        }
    }

    fn sqlquote(value: Value) -> String {
        if let Value::Number(value) = value {
            format!("{}", value)
        } else if let Value::String(value) = value {
            return format!("'{}'", value.replace('\'', "''"));
        } else {
            unimplemented!("Value type not implemented yet");
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::controllers::sites::SitesController;

    #[test]
    fn test_sql_quote() {
        use super::*;

        let value = json!(123);
        let res = SitesController::sqlquote(value);
        assert_eq!(res, "123", "Should return quoted string");

        let value = json!("123");
        let res = SitesController::sqlquote(value);
        assert_eq!(res, "'123'", "Should return quoted string");

        let value = json!("123 '123");
        let res = SitesController::sqlquote(value);
        assert_eq!(res, "'123 ''123'", "Should return quoted string");

        let value = json!("1'2'3");
        let res = SitesController::sqlquote(value);
        assert_eq!(res, "'1''2''3'", "Should return quoted string");
    }

    #[test]
    fn test_parse_query_with_non_object_params() {
        let query = "SELECT * FROM table";
        let params = json!(42);
        let (new_query, new_params) = SitesController::parse_query(query, params);
        assert_eq!(new_query, query);
        assert_eq!(new_params, Some(json!(42)));
    }

    #[test]
    fn test_parse_query_with_empty_params() {
        let query = "SELECT * FROM table WHERE ?";
        let params = json!({});
        let (new_query, new_params) = SitesController::parse_query(query, params);
        assert_eq!(new_query, "SELECT * FROM table WHERE  1");
        assert_eq!(new_params, None);
    }

    #[test]
    fn test_parse_query_with_select_query() {
        let query = "SELECT * FROM table WHERE ?";
        let params = json!({
            "id": [1, 2, 3]
        });
        let (new_query, new_params) = SitesController::parse_query(query, params);
        assert_eq!(new_query, "SELECT * FROM table WHERE  id IN (?, ?, ?)");
        assert_eq!(new_params, Some(json!(["1", "2", "3"])));
    }

    #[test]
    fn test_parse_query_with_update_query() {
        let query = "UPDATE table SET ? WHERE id = 1";
        let params = json!({
            "name": "New Name"
        });
        let (new_query, new_params) = SitesController::parse_query(query, params);
        assert_eq!(new_query, "UPDATE table SET  name = ? WHERE id = 1");
        assert_eq!(new_params, Some(json!(["\"New Name\""])));
    }

    #[test]
    fn test_parse_query_with_delete_query() {
        let query = "DELETE FROM table WHERE ?";
        let params = json!({
            "id": [1, 2, 3]
        });
        let (new_query, new_params) = SitesController::parse_query(query, params);
        assert_eq!(new_query, "DELETE FROM table WHERE  id IN (?, ?, ?)");
        assert_eq!(new_params, Some(json!(["1", "2", "3"])));
    }

    #[test]
    #[should_panic(expected = "Unknown query format")]
    fn test_parse_query_with_unknown_format() {
        let query = "UNKNOWN QUERY FORMAT";
        let params = json!({});
        let _ = SitesController::parse_query(query, params);
    }

    #[test]
    fn test_parse_query_with_multiple_params() {
        let query = "UPDATE table WHERE ?";
        let params = json!({
            "name": "New Name",
            "id": 1
        });
        let (new_query, new_params) = SitesController::parse_query(query, params);
        assert_eq!(new_query, "UPDATE table WHERE  id = ? AND name = ?");
        assert_eq!(new_params, Some(json!(["1", "\"New Name\"",])));
    }

    #[test]
    fn test_parse_query_with_colon_params() {
        let query = "SELECT * FROM table WHERE id = :id";
        let params = json!({
            "id": 1
        });
        let (new_query, new_params) = SitesController::parse_query(query, params);
        assert_eq!(new_query, "SELECT * FROM table WHERE id = :id");
        assert_eq!(new_params, Some(json!({"id": 1})));
    }

    #[test]
    fn test_parse_query_with_multiple_colon_params() {
        let query = "UPDATE table SET name = :name WHERE id = :id";
        let params = json!({
            "name": "New Name",
            "id": 1
        });
        let (new_query, new_params) = SitesController::parse_query(query, params);
        assert_eq!(new_query, "UPDATE table SET name = :name WHERE id = :id");
        assert_eq!(new_params, Some(json!({"id": 1, "name": "New Name"})));
    }

    #[test]
    fn test_parse_query_with_missing_colon_param() {
        let query = "SELECT * FROM table WHERE id = :id";
        let params = json!({});
        let (new_query, new_params) = SitesController::parse_query(query, params);
        assert_eq!(new_query, "SELECT * FROM table WHERE id = :id");
        assert_eq!(new_params, None);
    }

    #[test]
    fn test_parse_query_with_insert_query() {
        let query = "INSERT INTO table bio ?";
        let params = json!({
            "name": ["John", "Doe"],
            "age": [32, 30]
        });
        let (new_query, new_params) = SitesController::parse_query(query, params);
        assert_eq!(new_query, "INSERT INTO table bio (age, name) VALUES (?, ?)");
        assert_eq!(new_params, Some(json!(["[32,30]", "[\"John\",\"Doe\"]"])));
    }
}
