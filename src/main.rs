#![feature(drain_filter)]

pub mod common;
pub mod controllers;
#[allow(unused)]
pub mod core;
pub mod discovery;
pub mod environment;
pub mod io;
pub mod net;
pub mod protocol;
pub mod utils;

use crate::{
    common::*,
    controllers::sites::SitesController,
    core::{error::Error, site::Site},
    environment::*,
    io::db::DbManager,
    protocol::Protocol,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let site_storage = &*SITE_STORAGE;
    let user_storage = &*USER_STORAGE;
    let mut db_manager = DbManager::new();
    let mut user = user_storage.values().next().unwrap().clone();
    let sub_cmd = (&*MATCHES).subcommand();
    if let Some((cmd, _args)) = sub_cmd {
        if let Some(mut site_args) = _args.values_of("site") {
            let site_addr = site_args.next().unwrap();
            let mut site = Site::new(site_addr, (*ENV).data_path.clone())?;
            match cmd {
                "siteNeedFile" => {
                    let inner_path = site_args.next().unwrap();
                    site_need_file(&mut site, inner_path.into()).await?
                }
                "siteDownload" => download_site(&mut site).await?,
                "siteSign" => {
                    let private_key = if let Some(private_key) = site_args.next() {
                        private_key.to_owned()
                    } else {
                        if let Some(key) = user.sites.get(&site.address()).unwrap().get_privkey() {
                            key
                        } else {
                            unreachable!("No private key for site");
                        }
                    };
                    site_sign(&mut site, private_key.into()).await?
                }
                "siteVerify" => check_site_integrity(&mut site).await?,
                "dbRebuild" => rebuild_db(&mut site, &mut db_manager).await?,
                "dbQuery" => {
                    let schema = db_manager.load_schema(&site.address()).unwrap();
                    db_manager.insert_schema(&site.address(), schema);
                    db_manager.connect_db(&site.address());
                    let conn = db_manager.get_db(&site.address()).unwrap();
                    let query = site_args.next().unwrap();
                    db_query(conn, query).await?;
                }
                "siteFindPeers" => find_peers(&mut site).await?,
                "sitePeerExchange" => peer_exchange(&mut site).await?,
                "siteFetchChanges" => fetch_changes(&mut site).await?,
                _ => {
                    println!("Unknown command: {}", cmd);
                }
            }
        } else if let Some(mut peer_args) = _args.values_of("peer") {
            let peer = peer_args.next().unwrap();
            println!("{:?}", peer);
            match cmd {
                "peerPing" => peer_ping(peer).await?,
                _ => {
                    println!("Unknown command: {}", cmd);
                }
            }
        } else {
            match cmd {
                "siteCreate" => site_create(&mut user, true).await?,
                "getConfig" => println!("{}", serde_json::to_string_pretty(&client_info())?),
                _ => {
                    println!("Unknown command: {}", cmd);
                    println!("Unknown command: {}", cmd);
                }
            }
        }
    } else {
        println!("No command specified");
        let mut controller = SitesController::new();
        controller.extend_sites_from_sitedata(site_storage.clone());
    }
    Ok(())
}
