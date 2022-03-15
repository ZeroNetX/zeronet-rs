#![feature(drain_filter)]

pub mod common;
#[allow(unused)]
pub mod core;
pub mod discovery;
pub mod environment;
pub mod io;
pub mod net;
pub mod utils;

use crate::{
    common::*,
    core::{error::Error, io::*, site::*, user::*},
    environment::*,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut user = User::load().await?;
    let sub_cmd = (&*MATCHES).subcommand();
    if let Some((cmd, _args)) = sub_cmd {
        let site = _args.values_of("site").unwrap().into_iter().next().unwrap();
        let mut site = Site::new(site, (*ENV).data_path.clone())?;
        match cmd {
            "siteCreate" => site_create(&mut user, true).await?,
            "siteNeedFile" => {
                let inner_path = site_args.next().unwrap();
                site_need_file(&mut site, inner_path.into()).await?
            }
            "siteDownload" => download_site(&mut site).await?,
            "siteVerify" => check_site_integrity(&mut site).await?,
            "dbRebuild" => rebuild_db(&mut site).await?,
            "getConfig" => println!("{}", serde_json::to_string_pretty(&client_info())?),
            "siteFindPeers" => find_peers(&mut site).await?,
            "sitePeerExchange" => peer_exchange(&mut site).await?,
            "siteFetchChanges" => fetch_changes(&mut site).await?,
            _ => {
                println!("Unknown command: {}", cmd);
            }
        }
    } else {
        println!("No command specified");
    }
    Ok(())
}
