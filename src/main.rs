#![feature(drain_filter)]
#![feature(trait_alias)]

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

use log::*;

use crate::{
    common::*,
    controllers::{connections::ConnectionController, sites::SitesController, *},
    core::{error::Error, io::SiteIO, site::Site},
    environment::*,
    io::db::DbManager,
};

#[actix_web::main]
async fn main() -> Result<(), Error> {
    //TODO! Replace with file based logger with public release.
    pretty_env_logger::init_custom_env("DECENTNET_LOG");
    let user_storage = &*USER_STORAGE;
    let site_storage = &*SITE_STORAGE;
    let mut db_manager = DbManager::new();
    let mut user = user_storage.values().next().unwrap().clone();
    let sub_cmd = (*MATCHES).subcommand();
    if let Some((cmd, _args)) = sub_cmd {
        if let Some(mut site_args) = _args.values_of("site") {
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
                        let peer_id = (&site.peers.keys().next()).unwrap().clone();
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
        } else if let Some(mut peer_args) = _args.values_of("peer") {
            let peer = peer_args.next().unwrap();
            info!("{:?}", peer);
            match cmd {
                "peerPing" => peer_ping(peer).await?,
                _ => {
                    warn!("Unknown command: {}", cmd);
                }
            }
        } else {
            match cmd {
                "siteCreate" => site_create(&mut user, true).await?,
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
        let user_controller_addr = users::run().unwrap();
        let sites_controller_addr = sites::run().await.unwrap();
        let _ = server::run(sites_controller_addr, user_controller_addr).await;
    }
    Ok(())
}
