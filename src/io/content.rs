use std::{collections::HashMap, path::PathBuf};

use log::*;
use serde_json::{json, Map, Value};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};
use zerucontent::Content;

use crate::{
    core::{error::*, io::*, site::*},
    io::utils::get_zfile_info,
};

use super::utils::current_unix_epoch;

#[async_trait::async_trait]
impl ContentMod for Site {
    type ContentType = Content;
    async fn load_content_from_path(&self, inner_path: String) -> Result<Self::ContentType, Error> {
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
        content.modified = current_unix_epoch() as usize;
        let sign = content.sign(private_key.to_string());
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
                if content.verify(key.to_string()) {
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
    pub fn get_file_rules(&self, inner_path: &str) -> Option<Value> {
        let mut path = String::new();
        if !inner_path.ends_with("content.json") {
            let file_info = self.get_file_info(inner_path, false);
            if let Some(file_info) = file_info {
                let inner_path = &file_info["inner_path"].as_str();
                if let Some(inner_path) = inner_path {
                    path = inner_path.to_string();
                } else {
                    return None;
                }
            } else {
                return None;
            };
        };
        let inner_path = if path.is_empty() { inner_path } else { &path };
        if inner_path.ends_with("content.json") {
            if let Some(content) = self.content(Some(inner_path)) {
                if inner_path == "content.json" {
                    let mut rules = HashMap::<_, Value>::new();
                    let value = json!(content.signs.keys().cloned().collect::<String>());
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
                                error!("Handle User Content Rules for {}", content_inner_path);
                                return None;
                            }
                        } else if dirs.is_empty() {
                            break;
                        } else {
                            inner_path_parts.insert(0, dirs.pop().unwrap());
                        }
                    }
                }
            }
        }
        None
    }

    fn get_file_info(&self, inner_path: &str, new_file: bool) -> Option<Map<String, Value>> {
        let mut path = inner_path.split('/').collect::<Vec<&str>>();
        let mut file_name = path.pop().unwrap();
        // let site_path = self.site_path();
        let info_map = loop {
            let content_inner_path_dir = path.join("/").clone();
            let content_inner_path = if content_inner_path_dir.is_empty() {
                "content.json".into()
            } else {
                content_inner_path_dir + "/content.json"
            };
            let file_path = self.site_path().join(&content_inner_path);
            let content = self.content(Some(inner_path));
            if file_path.is_file() {
                if let Some(content) = content {
                    let mut map = Map::new();
                    map["content_inner_path"] = Value::String(content_inner_path);
                    map["relative_path"] = Value::String(file_name.into());
                    map["optional"] = Value::Null;
                    if new_file {
                        break Some(map);
                    }
                    if !content.files.is_empty() && content.files.contains_key(file_name) {
                        map["optional"] = Value::Bool(false);
                        let file = content.files.get(file_name).unwrap();
                        map["size"] = json!(file.size);
                        map["sha512"] = json!(file.sha512);
                        break Some(map);
                    }
                    if !content.files_optional.is_empty()
                        && content.files_optional.contains_key(file_name)
                    {
                        map["optional"] = Value::Bool(true);
                        let file = content.files_optional.get(file_name).unwrap();
                        map["size"] = json!(file.size);
                        map["sha512"] = json!(file.sha512);
                        break Some(map);
                    }
                    if content.user_contents.is_some() {
                        if let Value::Object(mut user_contents) =
                            json!(content.user_contents.unwrap())
                        {
                            map.append(&mut user_contents);
                        } else {
                            error!("add user_contents to map");
                            unreachable!();
                        }
                        break Some(map);
                    }
                }
            } else {
                debug!("Add {} to BadFiles", file_path.display());
            }
            if path.is_empty() {
                break None;
            }
            file_name = path.pop().unwrap();
        };
        info_map
    }
}
