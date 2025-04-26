use futures::executor::block_on;
use log::*;
use serde_json::Value;
use sha2::{Digest, Sha512};
use std::{
    collections::HashMap,
    fs::read,
    io::{Read, Write},
    net::{IpAddr, Ipv6Addr},
    path::{Path, PathBuf},
    str::FromStr,
    time::SystemTime,
};
use tokio::{fs::File, io::AsyncReadExt};
use zerucontent::File as ZFile;

use crate::{
    core::{
        error::Error,
        io::UserIO,
        site::models::SiteStorage,
        user::{
            models::{AuthPair, Cert, SiteData},
            User,
        },
    },
    environment::{DEF_PEERS_FILE_PATH, DEF_TRACKERS_FILE_PATH, ENV},
};

pub fn current_unix_epoch() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub async fn get_zfile_info(path: impl AsRef<Path>) -> Result<ZFile, Error> {
    let file = File::open(&path).await;
    if let Err(_err) = file {
        return Err(Error::FileNotFound(format!(
            "File Not Found at Path {:?}",
            path.as_ref()
        )));
    }
    let mut buf = Vec::new();
    file.unwrap().read_to_end(&mut buf).await?;
    let size = buf.len();
    let digest = Sha512::digest(buf);
    let sha512 = format!("{digest:x}")[..64].to_string();
    Ok(ZFile { size, sha512 })
}

pub async fn check_file_integrity<'a>(
    site_path: PathBuf,
    inner_path: &'a str,
    hash_str: &str,
) -> Result<(bool, &'a str, ZFile), Error> {
    let hash = get_zfile_info(site_path.join(inner_path)).await?;
    if hash_str != hash.sha512 {
        return Ok((false, inner_path, hash));
    }
    Ok((true, inner_path, hash))
}

const IP_V6_FOR_TEST: &str = "2607:f8b0:4006:81e::200e";

pub fn ipv6_supported() -> bool {
    let addr = IpAddr::from(Ipv6Addr::from_str(IP_V6_FOR_TEST).unwrap());
    //TODO! Replace with better testing mechanism
    ping::ping(addr, None, None, None, None, None).is_ok()
}

//TODO!: Rename this to import while depreciating legacy storage
pub fn load_users_file() -> HashMap<String, User> {
    info!("Loading users.json file");
    let users_file = ENV.data_path.join("users.json");
    let mut users = HashMap::<String, User>::new();
    if users_file.exists() {
        let users_file_str = std::fs::read_to_string(&users_file).unwrap();
        if let Ok(users_store) = serde_json::from_str::<HashMap<String, Value>>(&users_file_str) {
            for (username, user_obj) in users_store {
                info!("Loading user: {username}");
                if let Value::Object(user_map) = &user_obj {
                    let mut user = if let Value::String(master_seed) = &user_map["master_seed"] {
                        User::from_seed(master_seed.clone())
                    } else {
                        unimplemented!("No master seed found");
                    };
                    for (key, value) in user_map {
                        match key.as_str() {
                            "certs" => {
                                let mut certs = HashMap::<String, Cert>::new();
                                for (cert_name, cert_value) in value.as_object().unwrap() {
                                    if let Value::Object(cert_map) = cert_value {
                                        let auth_address =
                                            cert_map["auth_address"].as_str().unwrap().to_string();
                                        let auth_privatekey = cert_map["auth_privatekey"]
                                            .as_str()
                                            .unwrap()
                                            .to_string();
                                        let auth_pair =
                                            AuthPair::new(auth_address, auth_privatekey);
                                        let auth_type =
                                            cert_map["auth_type"].as_str().unwrap().to_string();
                                        let auth_user_name = cert_map["auth_user_name"]
                                            .as_str()
                                            .unwrap()
                                            .to_string();
                                        let cert_sign =
                                            cert_map["cert_sign"].as_str().unwrap().to_string();
                                        certs.insert(
                                            cert_name.to_string(),
                                            Cert::new(
                                                auth_pair,
                                                auth_type,
                                                auth_user_name,
                                                cert_sign,
                                            ),
                                        );
                                    }
                                }
                                user.certs.extend(certs);
                            }
                            "master_seed" => {}
                            "settings" => {
                                let sett: HashMap<String, Value> =
                                    serde_json::from_value(value.clone()).unwrap();
                                user.settings.extend(sett);
                            }
                            "sites" => {
                                let mut sites = HashMap::<String, SiteData>::new();
                                if let Value::Object(map) = value {
                                    for (address, obj) in map {
                                        if let Value::Object(site_map) = obj {
                                            let mut site_data = SiteData::new(address.to_string());
                                            let auth_address = site_map["auth_address"]
                                                .as_str()
                                                .unwrap()
                                                .to_string();
                                            let auth_privatekey = site_map["auth_privatekey"]
                                                .as_str()
                                                .unwrap()
                                                .to_string();
                                            let auth_pair =
                                                AuthPair::new(auth_address, auth_privatekey);
                                            site_data.with_auth_pair(auth_pair);
                                            for (key, value) in site_map {
                                                match key.as_str() {
                                                    "auth_address" | "auth_privatekey" => {}
                                                    "privatekey" => {
                                                        let priv_key =
                                                            value.as_str().unwrap().to_string();
                                                        site_data.with_privatekey(priv_key);
                                                    }
                                                    "cert" => {
                                                        let cert_name = value.as_str().unwrap();
                                                        site_data.add_cert_provider(cert_name);
                                                    }
                                                    "index" => {
                                                        let index = value.as_i64().unwrap() as u32;
                                                        site_data.with_index(index);
                                                    }
                                                    "settings" => {
                                                        site_data.set_settings(value.clone());
                                                    }
                                                    _ => site_data
                                                        .add_plugin_data(key.into(), value.clone()),
                                                }
                                            }
                                            sites.insert(address.to_string(), site_data);
                                        }
                                    }
                                }
                                user.sites.extend(sites);
                            }
                            _ => {}
                        }
                    }
                    users.insert(username, user);
                }
            }
        }
    } else {
        let res = std::fs::File::create(users_file);
        if let Ok(mut file) = res {
            let _ = file.write(b"{}");
            let mut user = User::new();
            user.settings.insert("theme".to_string(), "light".into());
            user.settings
                .insert("use_system_theme".to_string(), true.into());
            let res = block_on(user.save());
            if res.is_ok() {
                users.insert(user.master_address.clone(), user);
            } else {
                error!("Failed to save user");
            }
        } else {
            error!("Could not create Empty users.json file");
        }
    }
    users
}

//TODO!: Rename this to import while depreciating legacy storage
pub fn load_sites_file() -> HashMap<String, SiteStorage> {
    info!("Loading sites.json file");
    let mut sites_file = HashMap::new();
    let path = ENV.data_path.join("sites.json");
    if let Ok(mut file) = std::fs::File::open(&path) {
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        if let Ok(sites) = serde_json::from_str::<HashMap<String, Value>>(&buf) {
            for (site_addr, object) in sites {
                if let Value::Object(map) = object {
                    let mut storage = SiteStorage::default();
                    for (key, value) in map {
                        match key.as_ref() {
                            "added" => {
                                storage.stats.added = value.as_u64().unwrap_or_default() as usize
                            }
                            "downloaded" => {
                                storage.stats.downloaded =
                                    value.as_u64().unwrap_or_default() as usize
                            }
                            "modified" => {
                                storage.stats.modified = value.as_u64().unwrap_or_default() as usize
                            }
                            "bytes_recv" => {
                                storage.stats.bytes_recv =
                                    value.as_u64().unwrap_or_default() as usize
                            }
                            "bytes_sent" => {
                                storage.stats.bytes_sent =
                                    value.as_u64().unwrap_or_default() as usize
                            }
                            "peers" => {
                                storage.stats.peers = value.as_u64().unwrap_or_default() as usize
                            }
                            "size" => {
                                storage.stats.size = value.as_u64().unwrap_or_default() as usize
                            }
                            "size_optional" => {
                                storage.stats.size_optional =
                                    value.as_u64().unwrap_or_default() as usize
                            }
                            "autodownloadoptional"
                            | "optional_downloaded"
                            | "optional_help"
                            | "has_bigfile"
                            | "autodownload_bigfile_size_limit" => {
                                storage.plugin_storage.data.insert(key.to_string(), value);
                            }
                            "size_files_optional" => {
                                let value = value.as_u64().unwrap_or_default() as usize;
                                if value > 0 {
                                    unreachable!("size_files_optional is not used anymore");
                                }
                            }
                            "size_limit" => {
                                storage.settings.size_limit =
                                    value.as_u64().unwrap_or_default() as usize
                            }
                            "serving" => {
                                storage.settings.serving = value.as_bool().unwrap_or_default()
                            }
                            "own" => storage.stats.own = value.as_bool().unwrap_or_default(),
                            "modified_files_notification" => {
                                storage.stats.modified_files_notification =
                                    value.as_bool().unwrap_or_default()
                            }
                            "ajax_key" => {
                                storage.keys.ajax_key = value.as_str().unwrap_or_default().into()
                            }
                            "wrapper_key" => {
                                storage.keys.wrapper_key = value.as_str().unwrap_or_default().into()
                            }
                            "permissions" => {
                                storage.settings.permissions = value
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .map(|v| v.as_str().unwrap().to_string())
                                    .collect();
                            }
                            "hashfield" => {
                                storage.cache.hashfield = value.as_str().unwrap_or_default().into()
                            }
                            "bad_files" => {
                                storage
                                    .cache
                                    .bad_files
                                    .insert(key, value.as_i64().unwrap_or_default() as usize);
                            }
                            "piecefields" => {
                                storage
                                    .cache
                                    .piecefields
                                    .insert(key, value.as_str().unwrap_or_default().to_string());
                            }
                            _ => {
                                storage.plugin_storage.data.insert(key, value);
                            }
                        }
                    }
                    sites_file.insert(site_addr, storage);
                }
            }
        }
    } else {
        let res = std::fs::File::create(path);
        if let Ok(mut file) = res {
            let _ = file.write(b"{}");
        } else {
            error!("Could not create Empty sites.json file");
        }
    }
    sites_file
}

pub async fn load_peers() -> Vec<String> {
    load_strings_from_path(&DEF_PEERS_FILE_PATH).await
}

pub fn load_trackers() -> Vec<String> {
    let file = Path::new(&*DEF_TRACKERS_FILE_PATH);
    let buf = read(file);
    if !file.exists() || buf.is_err() {
        return vec![];
    }
    let mut trackers = vec![];
    for peer in buf
        .unwrap()
        .split(|b| (Some(b) == b"\n".first()) || (Some(b) == b"\r".first()))
    {
        trackers.push(String::from_utf8(peer.to_vec()).unwrap());
    }
    trackers.extract_if(.., |peer| !peer.is_empty()).collect()
}

pub async fn load_strings_from_path(path: &PathBuf) -> Vec<String> {
    let path = Path::new(&path);
    let file = File::open(path).await;
    if !path.exists() || file.is_err() {
        return vec![];
    }
    let mut file = file.unwrap();
    let mut buf = vec![];
    let res = tokio::io::AsyncReadExt::read_to_end(&mut file, &mut buf)
        .await
        .is_ok();
    if !res {
        return vec![];
    }
    let mut strings = vec![];
    for peer in buf.split(|b| (Some(b) == b"\n".first()) || (Some(b) == b"\r".first())) {
        if let Ok(peer) = String::from_utf8(peer.to_vec()) {
            strings.push(peer);
        }
    }
    strings.extract_if(.., |peer| !peer.is_empty()).collect()
}
