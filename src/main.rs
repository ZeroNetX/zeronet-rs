#![feature(drain_filter)]

pub mod common;
pub mod controllers;
#[allow(unused)]
pub mod core;
pub mod discovery;
pub mod environment;
pub mod io;
pub mod net;
pub mod utils;

use crate::{
    common::*,
    controllers::sites::SitesController,
    core::{error::Error, site::Site},
    environment::*,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let site_storage = &*SITE_STORAGE;
    let user_storage = &*USER_STORAGE;
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
                    let private_key = site_args.next().unwrap();
                    site_sign(&mut site, private_key.into()).await?
                }
                "siteVerify" => check_site_integrity(&mut site).await?,
                "dbRebuild" => rebuild_db(&mut site).await?,
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
