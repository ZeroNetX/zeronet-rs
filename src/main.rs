#![feature(drain_filter)]
#![feature(trait_alias)]
#![feature(let_chains)]

pub mod common;
pub mod controllers;
#[allow(unused)]
pub mod core;
pub mod discovery;
pub mod environment;
pub mod io;
pub mod net;
pub mod plugins;
pub mod utils;

use std::path::PathBuf;

use log::*;

use crate::{
    common::*,
    controllers::{connections::ConnectionController, sites::SitesController, *},
    core::{error::Error, io::SiteIO, site::Site},
    environment::*,
    io::db::DbManager,
    plugins::core::manifest::PluginManifest,
};

#[actix_web::main]
async fn main() -> Result<(), Error> {
    let plugins = &*PLUGINS;
    let user_storage = &*USER_STORAGE;
    let site_storage = &*SITE_STORAGE;
    let mut db_manager = DbManager::new();
    let mut user = user_storage.values().next().unwrap().clone();
    let sub_cmd = (*MATCHES).subcommand();
    if let Some((cmd, args)) = sub_cmd {
        if cmd.starts_with("site") && let Some(mut site_args) = args.get_many::<String>("site") {
            let site_addr = site_args.next().unwrap();
            let mut site = Site::new(site_addr, (ENV.data_path.clone()).join(site_addr))?;
            if let Some(storage) = site_storage.get(site_addr).cloned() {
                site.modify_storage(storage);
            }
            match cmd {
                "siteFindPeers" | "siteNeedFile" | "siteDownload" | "siteUpdate"
                | "sitePeerExchange" | "siteFetchChanges" => {
                    add_peers_to_site(&mut site).await?;
                    let mut found_connectable_peer = false;
                    while !found_connectable_peer {
                        let peer_id = site.peers.keys().next().unwrap().clone();
                        let peer = site.peers.values_mut().next().unwrap();
                        let conn = peer.connection_mut().unwrap();
                        let mut protocol = net::Protocol::new(conn);
                        use decentnet_protocol::{interface::RequestImpl, Either};
                        let res = protocol
                            .get_file(site_addr.into(), "content.json".into(), 0, 0, Some(1))
                            .await;
                        if let Ok(res) = res {
                            match res {
                                Either::Success(_) => {
                                    found_connectable_peer = true;
                                }
                                Either::Error(err) => {
                                    if err.error == "Unknown site" {
                                        debug!("Site Not Served by Peer Querying Next Peer");
                                    } else {
                                        error!("Unknown Error : {}", err.error);
                                    }
                                    site.peers.remove(&peer_id);
                                }
                            }
                        } else {
                            error!("Communication Error {:#?}", res);
                        }
                    }
                }
                _ => {}
            }
            match cmd {
                "siteCreate" => site_create(&mut user, true).await?,
                "siteNeedFile" => {
                    let inner_path = site_args.next().unwrap();
                    site_need_file(&mut site, inner_path.into()).await?
                }
                "siteDownload" => download_site(&mut site).await?,
                "siteSign" => {
                    let private_key = if let Some(private_key) = site_args.next() {
                        private_key.to_owned()
                    } else if let Some(key) = user.sites.get(&site.address()) {
                        if let Some(key) = key.get_privkey() {
                            key
                        } else {
                            unreachable!("No private key for site");
                        }
                    } else {
                        unreachable!("No private key for site");
                    };
                    site_sign(&mut site, private_key).await?
                }
                "siteFileEdit" => {
                    let inner_path = site_args.next().unwrap();
                    site_file_edit(&mut site, inner_path.into()).await?;
                }
                "siteUpdate" => {
                    let inner_path = site_args.next().unwrap();
                    site_update(&mut site, Some(inner_path)).await?
                }
                "siteVerify" => check_site_integrity(&mut site).await?,
                "dbRebuild" => rebuild_db(&mut site, &mut db_manager).await?,
                "dbQuery" => {
                    db_query(&mut site, &mut db_manager, site_args.next().unwrap()).await?
                }
                "siteFindPeers" => {
                    let mut connectable_peers = site
                        .peers
                        .values()
                        .into_iter()
                        .map(|peer| peer.address().to_string());
                    save_peers(&mut connectable_peers).await;
                }
                "sitePeerExchange" => peer_exchange(&mut site).await?,
                "siteFetchChanges" => fetch_changes(&mut site).await?,
                _ => {
                    warn!("Unknown command: {}", cmd);
                }
            }
            user.get_site_data(site_addr, true);
            let mut storage = site.storage.clone();
            storage.settings.serving = true;
            site.modify_storage(storage);
            site.save_storage().await?;
        } else if cmd.starts_with("peer") && let Some(mut peer_args) = args.get_many::<String>("peer") {
            let peer = peer_args.next().unwrap();
            info!("{:?}", peer);
            match cmd {
                "peerPing" => peer_ping(peer).await?,
                _ => {
                    warn!("Unknown command: {}", cmd);
                }
            }
        } else if cmd.starts_with("crypt") {
            match cmd {
                "cryptKeyPair" => {
                    let (priv_key, pub_key) = zeronet_cryptography::create();
                    info!(
                        "Your Private key : {}",
                        zeronet_cryptography::privkey_to_wif(priv_key)
                    );
                    info!("Your Public key : {}", pub_key);
                }
                "cryptSign" => {
                    let mut args = args.get_many::<String>("data").unwrap();
                    let data = args.next().unwrap();
                    if let Some(priv_key) = args.next() {
                        match zeronet_cryptography::sign(data.as_str(), priv_key) {
                            Ok(signature) => info!("{}", signature),
                            Err(err) => error!("{}", err),
                        }
                    } else {
                        error!("cryptSign cmd requires private key to sign");
                    };
                }
                "cryptVerify" => {
                    let mut args = args.get_many::<String>("data").unwrap();
                    let data = args.next().unwrap();
                    if let Some(pub_key) = args.next() {
                        if let Some(signature) = args.next() {
                            match zeronet_cryptography::verify(data.as_str(), pub_key, signature) {
                                Ok(_) => info!("Signature Successfully verified"),
                                Err(err) => error!("{}", err),
                            }
                        } else {
                            error!("cryptVerify cmd requires signature as arg to verify");
                        }
                    } else {
                        error!("cryptVerify cmd requires Public key & Signature as args");
                    };
                }
                _ => {
                    warn!("Unknown command: {}", cmd);
                }
            }
        }  else if cmd.starts_with("plugin") {
            match cmd {
                "pluginSign" => {
                    let mut args = args.get_many::<String>("name").unwrap();
                    let name = args.next().unwrap();
                    let manifest = PluginManifest::load(name).await;
                    let manifest_path = PathBuf::from(format!("plugins/{}/manifest.json", name));
                    if manifest.is_ok() &&  let Some(private_key) = args.next() {
                        let manifest = manifest.unwrap().sign_plugin(private_key).await.unwrap();
                        let contents = serde_json::to_string_pretty(&manifest).unwrap();
                        tokio::fs::write(manifest_path, contents).await?;
                    } else {
                        error!("pluginSign cmd requires private key to sign");
                    };
                }
                "pluginVerify" => {
                    let mut args = args.get_many::<String>("name").unwrap();
                    let name = args.next().unwrap();
                    let manifest = PluginManifest::load(name).await.unwrap();
                    let verified = manifest.verify_plugin().await.unwrap_or(false) ;
                    println!("Plugin Verified : {}", verified);
                }
                _ => {
                    warn!("Unknown command: {}", cmd);
                }
            }
        } else {
            match cmd {
                "getConfig" => info!("{}", serde_json::to_string_pretty(&client_info())?),
                _ => {
                    warn!("Unknown command: {}", cmd);
                }
            }
        }
    } else if false {
        let conn = DbManager::connect_db_from_path(&ENV.data_path.join("content.db"))?;
        db_manager.insert_connection("content_db", conn);
        let mut controller = SitesController::new(db_manager);
        controller
            .extend_sites_from_sitedata(site_storage.clone())
            .await;
        let mut con = ConnectionController::new(controller).await?;
        let _ = con.run().await;
    } else {
        info!("Loaded : {} Plugins.", plugins.len());
        let user_controller_addr = users::run().unwrap();
        let sites_controller_addr = sites::run().await.unwrap();
        let _ =
            plugins::site_server::server::run(sites_controller_addr, user_controller_addr).await;
    }
    Ok(())
}
