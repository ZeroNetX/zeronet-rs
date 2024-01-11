use std::collections::HashMap;

use itertools::Itertools;
use log::*;
use tokio::{fs, io::AsyncWriteExt};

use decentnet_protocol::{address::PeerAddr, interface::RequestImpl};

use crate::{
    controllers::sites::SitesController,
    core::{error::Error, io::*, peer::*, site::*, user::*},
    environment::{DEF_PEERS_FILE_PATH, ENV},
    io::db::DbManager,
    net::Protocol,
    utils,
};

pub async fn site_create(user: &mut User, use_master_seed: bool) -> Result<(), Error> {
    let site_data;
    if use_master_seed {
        site_data = user.get_new_site_data(true);
        info!("\n");
        info!(
            "Site Private Key : {:?} <<< Store this to Safe Place",
            site_data.get_privkey().unwrap()
        );
        info!("Site Address     : {:?}", site_data.address);
        info!("\n");
    } else {
        unimplemented!();
    }
    let mut site = Site::new(&site_data.address, ENV.data_path.clone())?;
    site.create(site_data.index.unwrap(), &site_data.get_privkey().unwrap())
        .await?;
    Ok(())
}

pub async fn rebuild_db(site: &mut Site, db_manager: &mut DbManager) -> Result<(), Error> {
    let has_schema = db_manager.has_schema(&site.address());
    let address = site.address();
    if has_schema.0 {
        let schema = db_manager.load_schema(&address).unwrap();
        db_manager.insert_schema(&address, schema);
        db_manager.connect_db(&address)?;
        db_manager.create_tables(&address);
        db_manager.load_data(&address).await;
    }
    Ok(())
}

pub async fn db_query(
    site: &mut Site,
    db_manager: &mut DbManager,
    query: &str,
) -> Result<(), Error> {
    let schema = db_manager.load_schema(&site.address()).unwrap();
    db_manager.insert_schema(&site.address(), schema);
    db_manager.connect_db(&site.address())?;
    let conn = db_manager.get_db(&site.address()).unwrap();
    let res = SitesController::db_query(conn, query, None).await?;
    for row in res {
        info!("{:#?}", row);
    }
    Ok(())
}

pub async fn download_site(site: &mut Site) -> Result<(), Error> {
    info!("Downloading Site");
    site.init_download().await?;
    Ok(())
}

pub async fn site_sign(site: &mut Site, private_key: String) -> Result<(), Error> {
    site.load_content().await?;
    let changes = site.check_site_integrity().await?;
    if changes.is_empty() {
        info!("No changes to sign");
    } else {
        let content = {
            let mut content = site.content(None).unwrap();
            let mut files = content.files;
            for (inner_path, file) in changes {
                if files.insert(inner_path, file).is_none() {
                    unreachable!();
                };
            }
            //TODO! Verify inner content as well
            content.files = files;
            content
        };
        site.modify_content(None, content);
        let res = site.verify_files(false).await?;
        if res {
            site.sign_content(None, &private_key).await?;
            site.save_content(None).await?;
        } else {
            warn!("Site Not Signed");
        }
    }
    Ok(())
}

pub async fn site_file_edit(site: &mut Site, inner_path: String) -> Result<(), Error> {
    let file_path = site.site_path().join(inner_path.clone());
    let mut file_path_str = inner_path.clone();
    file_path_str.push_str(".old");
    let old_path = site.site_path().join(file_path_str);
    if old_path.exists() {
        fs::remove_file(&old_path).await.unwrap();
    }
    fs::copy(&file_path, &old_path).await.unwrap();
    Ok(())
}

pub async fn site_update(site: &mut Site, content: Option<&str>) -> Result<(), Error> {
    site.load_content().await?;
    let inner_path = content.unwrap_or("content.json");
    let path = site.site_path();
    let content = site.content(Some(inner_path)).unwrap();
    let diffs = content.files.keys().filter(|path_str| {
        let mut path_str = (*path_str).clone();
        path_str.push_str(".old");
        path.join(&path_str).exists()
    });
    let mut map = HashMap::new();
    for path in diffs {
        let inner_path = (*path).clone();
        let content_path = site.site_path().join(inner_path.clone());
        let content = fs::read_to_string(content_path.clone()).await?;
        let mut path = path.to_string();
        path.push_str(".old");
        let path = site.site_path().join(path);
        let old_content = fs::read_to_string(&path).await?;
        // fs::remove_file(path).await?;
        let diff = utils::diff::calc_diff(&old_content, &content);
        map.insert(inner_path, diff);
    }
    site.update(inner_path, Some(map)).await;
    Ok(())
}

pub async fn site_need_file(site: &mut Site, inner_path: String) -> Result<(), Error> {
    let download = if inner_path == "content.json" {
        true
    } else {
        site.load_content().await?;
        let content = site.content(None).unwrap();
        let files = content.files;
        let files_res = files.keys().any(|path| path == &inner_path);
        let includes_res = content.includes.keys().any(|path| path == &inner_path);
        let users_res = content
            .includes
            .keys()
            .any(|path| path.starts_with("data/users/"));
        files_res || includes_res || users_res
    };
    if !download {
        info!("Inner Path Not Exists in content.json");
    } else {
        let result = site.need_file(inner_path.clone(), None, None).await;
        if let Err(e) = &result {
            error!("Error : {:?}", e);
        } else {
            info!("File Downloaded : {:?}", inner_path);
        }
    }
    Ok(())
}

pub async fn peer_ping(addr: &str) -> Result<(), Error> {
    let mut peer = Peer::new(PeerAddr::parse(addr).unwrap());
    let res = peer.connect();
    if res.is_ok() {
        let res = Protocol::new(peer.connection_mut().unwrap()).ping().await?;
        info!("Ping Result : {:?} from Peer : {:?}", res, addr);
        return Ok(());
    }
    Err(Error::Err("Peer Not Found".into()))
}

pub async fn peer_exchange(site: &mut Site) -> Result<(), Error> {
    site.load_content().await?;
    let mut peers_cons = vec![];
    let peers = site.fetch_peers().await?;
    info!("Found Peers : {:?}", peers);
    for peer in peers {
        let mut peer = Peer::new(PeerAddr::parse(peer).unwrap());
        let res = peer.connect();
        if res.is_ok() {
            peers_cons.push(peer);
        }
    }
    for mut con in peers_cons {
        let res = Protocol::new(con.connection_mut().unwrap()).ping().await?;
        info!("Ping Result : {:?} from Peer : {:?}", res, con.address());
    }
    Ok(())
}

pub async fn fetch_changes(site: &mut Site) -> Result<(), Error> {
    site.load_content().await?;
    let modified = site.content(None).unwrap().modified;
    info!("{:?}", modified);
    let changes = site.fetch_changes(1421043090).await?;
    info!("{:#?}", changes);
    Ok(())
}

pub async fn check_site_integrity(site: &mut Site) -> Result<(), Error> {
    site.load_content().await?;
    let res = site.verify_files(false).await?;
    if res {
        info!("Site {} verified", site.address());
    } else {
        warn!("Site {} verification failed", site.address());
    }
    Ok(())
}

pub async fn add_peers_to_site(site: &mut Site) -> Result<(), Error> {
    let peers = site.find_peers().await.unwrap();
    site.add_peers(peers);
    Ok(())
}

pub async fn save_peers(peers: &mut impl Iterator<Item = String>) {
    let mut file = tokio::fs::File::create(&*DEF_PEERS_FILE_PATH)
        .await
        .unwrap();
    let peers = peers.join("\n");
    let _ = file.write(peers.as_bytes()).await.unwrap();
}
