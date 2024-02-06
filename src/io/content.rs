use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use itertools::Itertools;
use log::*;
use regex::Regex;
use serde_json::{json, Map, Value};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};
use zerucontent::{user_contents::PermissionRulesType, Content};

use crate::{
    core::{error::*, io::*, site::*},
    io::utils::get_zfile_info,
};

use super::utils::current_unix_epoch;

#[async_trait::async_trait]
impl ContentMod for Site {
    type ContentType = Content;
    async fn load_content_from_path(&self, inner_path: &str) -> Result<Self::ContentType, Error> {
        let path = &self.site_path().join(&inner_path);
        if path.is_file() {
            let mut file = File::open(path).await?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).await?;
            let content: Content = serde_json::from_slice(&buf)?;
            return Ok(content);
        }
        Err(Error::Err("Content File Not Found".into()))
    }

    async fn add_file_to_content(&mut self, inner_path: PathBuf) -> Result<(), Error> {
        let path = self.site_path().join(&inner_path);
        if path.is_file() {
            let file = get_zfile_info(path).await?;
            let res = &mut self.content_mut(None).unwrap().files;
            res.insert(inner_path.display().to_string(), file);
            Ok(())
        } else {
            return Err(Error::Err("File does not exist".into()));
        }
    }

    async fn sign_content(
        &mut self,
        inner_path: Option<&str>,
        private_key: &str,
    ) -> Result<(), Error> {
        let content = self.content_mut(inner_path).unwrap();
        content.modified = current_unix_epoch().into();
        let sign = content.sign(private_key);
        let address = zeronet_cryptography::privkey_to_pubkey(private_key)?;
        content.signs.insert(address, sign);
        Ok(())
    }

    fn verify_content(&self, inner_path: Option<&str>) -> Result<(), Error> {
        let content = self.content(inner_path).unwrap();
        let verified = content
            .signs
            .keys()
            .into_iter()
            .find_map(|key| {
                if content.verify(key) {
                    Some(true)
                } else {
                    None
                }
            })
            .is_some();
        if verified {
            Ok(())
        } else {
            Err(Error::Err(format!(
                "Content verification failed for Site : {}",
                self.address()
            )))
        }
    }

    async fn save_content(&mut self, inner_path: Option<&str>) -> Result<(), Error> {
        let content = self.content(inner_path).unwrap();
        let content_json = serde_json::to_string_pretty(&content)?;
        let inner_path = inner_path.unwrap_or("content.json");
        let path = self.site_path().join(inner_path);
        if path.exists() {
            fs::remove_file(&path).await?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&path)
            .await?;
        file.write_all(content_json.as_bytes()).await?;
        Ok(())
    }
}

impl Site {
    pub fn get_valid_signers(&self, inner_path: &str) -> impl IntoIterator<Item = String> {
        let mut valid_signers = HashSet::new();
        if inner_path == "content.json" {
            if let Some(content) = self.content(Some(inner_path)) {
                valid_signers.extend(content.signers.clone());
            }
        } else {
            if let Some(rules) = self.get_file_rules(inner_path) {
                if let Some(signers) = rules.get("signers") {
                    if let Value::String(signers) = signers {
                        valid_signers.insert(signers.clone());
                    }
                }
            }
        }
        valid_signers.insert(self.address().to_string());
        valid_signers
    }

    /// Get File Rules for Given inner_path
    /// If inner_path doesn't end with "content.json"
    pub fn get_file_rules(&self, inner_path: &str) -> Option<Value> {
        let mut path = String::new();
        if !inner_path.ends_with("content.json") {
            let file_info = self.get_file_info(inner_path, false)?;
            let inner_path = &file_info["content_inner_path"].as_str()?;
            path = inner_path.to_string();
        };
        let inner_path = if path.is_empty() { inner_path } else { &path };
        if inner_path == "content.json" {
            let mut rules = HashMap::<_, Value>::new();
            let value = json!(self
                .get_valid_signers(inner_path)
                .into_iter()
                .collect::<Vec<String>>());
            rules.insert("signers".to_string(), value);
            return Some(json!(rules));
        } else {
            let mut dirs = inner_path.split('/').collect::<Vec<_>>();
            let mut inner_path_parts = vec![];
            inner_path_parts.insert(0, dirs.pop().unwrap());
            inner_path_parts.insert(0, dirs.pop().unwrap());
            loop {
                let content_inner_path = dirs.join("/");
                if let Some(parent_content) = self.content(Some(&content_inner_path)) {
                    if !parent_content.includes.is_empty() {
                        let includes = parent_content
                            .includes
                            .get(&inner_path_parts.join("/"))
                            .unwrap();
                        return Some(json!(includes));
                    } else if parent_content.user_contents.is_some() {
                        let content = self.content(Some(inner_path))?;
                        let user_content_rules =
                            self.get_user_content_rules(&parent_content, &inner_path, &content);
                        return Some(json!(user_content_rules));
                    }
                } else if dirs.is_empty() {
                    break;
                } else {
                    inner_path_parts.insert(0, dirs.pop().unwrap());
                }
            }
        }
        None
    }

    pub fn get_user_content_rules(
        &self,
        parent_content: &Content,
        inner_path: &str,
        content: &Content,
    ) -> Map<String, Value> {
        let user_contents = parent_content.user_contents.as_ref().unwrap();
        let user_address = if !parent_content.meta.inner_path.is_empty() {
            let parent_content_dir = parent_content
                .meta
                .inner_path
                .clone()
                .replace("content.json", "");
            let re = Regex::new(r"([A-Za-z0-9]*?)/").unwrap();
            re.captures(&inner_path[parent_content_dir.len()..])
                .unwrap()[1]
                .to_string()
        } else {
            let re = Regex::new(r".*/([A-Za-z0-9]*?)/.*?$").unwrap();
            re.captures(inner_path).unwrap()[1].to_string()
        };

        let (user_urn, cert_user_id) = match &content.cert {
            Some(cert) => {
                let user_urn = format!("{}/{}", cert.auth_type, cert.user_id); // web/nofish@zeroid.bit
                let cert_user_id = cert.user_id.to_string();
                (user_urn, cert_user_id)
            }
            None => ("n-a/n-a".to_string(), "n-a".to_string()),
        };

        let mut rules = Map::new();
        let r = if user_contents.permissions.contains_key(&user_address) {
            user_contents
                .permissions
                .get(&user_address)
                .unwrap()
                .clone() // Default rules based on address
        } else {
            user_contents
                .permissions
                .get(&cert_user_id)
                .unwrap()
                .clone() // Default rules based on username
        };
        let mut banned = false;
        if let PermissionRulesType::None(false) = r {
            banned = true;
        } else if let PermissionRulesType::Rules(r) = r {
            banned = false;
            rules.extend(json!(r).as_object().unwrap().clone());
        }

        for (permission_pattern, permission_rules) in &user_contents.permission_rules {
            if !Regex::new(&permission_pattern).unwrap().is_match(&user_urn) {
                continue;
            }
            let permission_rules = json!(permission_rules);
            let permission_rules = permission_rules.as_object().unwrap();
            for (key, val) in permission_rules.iter() {
                match rules.get(key) {
                    Some(value) => match (val, value) {
                        (Value::Number(val), Value::Number(value)) => {
                            if val.as_u64().unwrap() > value.as_u64().unwrap() {
                                rules.insert(key.to_string(), json!(val.clone()));
                            }
                        }
                        (Value::String(val), Value::String(value)) => {
                            if val.len() > value.len() {
                                rules.insert(key.to_string(), json!(val.clone()));
                            }
                        }
                        (Value::Array(val), Value::Array(value)) => {
                            let mut new_val = value.clone();
                            new_val.extend(val.clone());
                            rules.insert(key.to_string(), json!(new_val));
                        }
                        _ => {}
                    },
                    None => {
                        rules.insert(key.to_string(), val.clone());
                    }
                }
            }
        }

        // Accepted cert signers
        rules.insert(
            "cert_signers".to_string(),
            json!(user_contents.cert_signers),
        );
        rules.insert(
            "cert_signers_pattern".to_string(),
            json!(user_contents.cert_signers_pattern),
        );

        if !rules.contains_key("signers") {
            rules.insert("signers".to_string(), json!(Vec::<String>::new()));
        }

        if !banned {
            if let Some(signers) = rules.get_mut("signers") {
                signers
                    .as_array_mut()
                    .unwrap()
                    .push(json!(user_address.clone())); // Add user as valid signer
            }
        }
        rules.insert("user_address".to_string(), json!(user_address));
        rules.insert("includes_allowed".to_string(), json!(false));

        rules
    }

    /// Get File Info for given inner_path
    /// if new_file is true default content map values will be returned
    /// Returns None if file not found
    fn get_file_info(&self, inner_path: &str, new_file: bool) -> Option<Map<String, Value>> {
        let mut dirs = inner_path.split('/').collect_vec();
        let mut file_name = dirs.pop()?;
        // let site_path = self.site_path();
        let info_map = loop {
            let content_inner_path_dir = dirs.join("/");
            let content_inner_path = if content_inner_path_dir.is_empty() {
                "content.json".into()
            } else {
                content_inner_path_dir.clone() + "/content.json"
            };
            let file_path = self.site_path().join(&content_inner_path);
            if file_path.is_file() {
                //TODO! Lazy Load Content
                let content = self.content(Some(&content_inner_path));
                if let Some(content) = content {
                    let mut map = Map::new();
                    map.insert("content_inner_path".into(), json!(content_inner_path));
                    map.insert("relative_path".into(), json!(file_name));
                    map.insert("optional".into(), Value::Null);
                    if new_file {
                        break Some(map);
                    }
                    if !content.files.is_empty() && content.files.contains_key(file_name) {
                        map.insert("optional".into(), json!(false));
                        let file = content.files.get(file_name)?;
                        map.insert("size".into(), json!(file.size));
                        map.insert("sha512".into(), json!(file.sha512));
                        break Some(map);
                    }
                    if !content.files_optional.is_empty()
                        && content.files_optional.contains_key(file_name)
                    {
                        map.insert("optional".into(), json!(true));
                        let file = content.files_optional.get(file_name)?;
                        map.insert("size".into(), json!(file.size));
                        map.insert("sha512".into(), json!(file.sha512));
                        break Some(map);
                    }
                    if content.user_contents.is_some() {
                        if let Value::Object(mut user_contents) =
                            json!(content.user_contents.as_ref()?)
                        {
                            map.append(&mut user_contents);
                        } else {
                            error!("add user_contents to map");
                            unreachable!();
                        }
                        let relative_content_path = inner_path
                            .strip_prefix(&content_inner_path_dir)
                            .unwrap_or("");
                        let regex = regex::Regex::new("([A-Za-z0-9]+)/.*").unwrap();
                        if regex.is_match(relative_content_path) {
                            let captures = regex.captures(relative_content_path).unwrap();
                            let user_auth_address = captures.get(1).unwrap().as_str();
                            let path = format!(
                                "{}/{}/content.json",
                                content_inner_path_dir, user_auth_address
                            );
                            map.insert("content_inner_path".into(), path.into());
                        }
                        break Some(map);
                    }
                }
            } else {
                //TODO! Add more tests for this case
                debug!("Add {} to BadFiles", file_path.display());
            }
            if dirs.is_empty() {
                //TODO! Add more tests for this case
                break None;
            }
            file_name = dirs.pop()?;
        };
        info_map
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Map, Value};
    use std::path::PathBuf;

    use crate::io::content::ContentMod;

    use super::Site;

    #[tokio::test]
    async fn test_root_content() {
        let address = "15UYrA7aXr2Nto1Gg4yWXpY3EAJwafMTNk";
        let content_path = "LICENSE";
        let res = test_get_file_info(address, "content.json", content_path).await;
        assert!(res.is_some());
        let res = res.unwrap();
        assert_eq!(res["content_inner_path"], json!("content.json"));
        assert_eq!(res["relative_path"], json!("LICENSE"));
        assert_eq!(res["optional"], json!(false));
        assert_eq!(res["size"], json!(18027));
        assert_eq!(
            res["sha512"],
            json!("d281feecb7d1218e1aea8269f288fcd63385da1a130681fadae77262637cb65f")
        );
    }

    #[tokio::test]
    async fn test_root_user_content() {
        let address = "15UYrA7aXr2Nto1Gg4yWXpY3EAJwafMTNk";
        let content_path = "data/users/content.json";
        let res = test_get_file_info(address, content_path, content_path).await;
        assert!(res.is_some());
        let res = res.unwrap();
        assert_eq!(res["content_inner_path"], json!("data/users/content.json"));
        assert_eq!(res["relative_path"], json!("content.json"));
        assert_eq!(res["optional"], Value::Null);
    }

    #[tokio::test]
    async fn test_root_user_content1() {
        let addr = "15UYrA7aXr2Nto1Gg4yWXpY3EAJwafMTNk";
        let content_path = "data/users/1AmeB7f5wBfJm6iR7MRZfFh65xkJzaVCX7/content.json";
        let path = PathBuf::from(format!("tests/data/{}", addr));
        let mut site = Site::new(addr, path).unwrap();
        load_site_content(&mut site, content_path).await;
        load_site_content(&mut site, "data/users/content.json").await;
        let res = site.get_file_info(content_path, false);
        assert!(res.is_some());
        let res = res.unwrap();
        assert_eq!(
            res["content_inner_path"],
            json!("data/users/1AmeB7f5wBfJm6iR7MRZfFh65xkJzaVCX7/content.json")
        );
        assert_eq!(res["relative_path"], json!("content.json"));
        assert_eq!(res["optional"], Value::Null);
    }

    async fn load_site_content<'a>(site: &'a mut Site, inner_path: &'a str) {
        let res = site.load_content_from_path(inner_path).await;
        let res = res.ok().unwrap();
        site.modify_content(Some(inner_path), res);
    }

    async fn test_get_file_info(
        addr: &str,
        inner_path: &str,
        file_path: &str,
    ) -> Option<Map<String, Value>> {
        let path = PathBuf::from(format!("tests/data/{}", addr));
        let mut site = Site::new(addr, path).unwrap();
        load_site_content(&mut site, inner_path).await;
        site.get_file_info(file_path, false)
    }
}
