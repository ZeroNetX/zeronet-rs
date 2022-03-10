#![feature(drain_filter)]

#[allow(unused)]
pub mod core;
pub mod discovery;
pub mod environment;
pub mod io;
pub mod net;
pub mod utils;

use crate::{
    core::{discovery::Discovery, peer::Peer},
    io::db::DbManager,
    net::Protocol,
};

use environment::ENV;
use zeronet_protocol::PeerAddr;

use crate::core::{error::Error, io::*, site::Site, user::User};

async fn _main_old() -> Result<(), Error> {
    println!("Loading User");
    let _user = User::load()?;
    let site = "15UYrA7aXr2Nto1Gg4yWXpY3EAJwafMTNk";
    println!("Loading Site : {site}");
    let site = Site::new(site, (*ENV).data_path.clone())?;
    let peers = site.discover().await?;
    for peer in &peers {
        println!("{:?}", peer);
    }
    let mut connections = vec![];
    for mut peer in peers {
        let res = peer.connect();
        if let Err(e) = &res {
            println!("Error : {:?}", e);
            let peer = peer.clone().address().to_string();
            println!("{}", peer);
        } else {
            println!("Connection Successful");
            connections.push(peer);
        }
    }
    let connectable_peers = connections.iter().map(|peer| peer.address().to_string());
    _save_peers(connectable_peers).await;

    // let mut peer = Peer::new(PeerAddr::IPV4([127, 0, 0, 1], 11917));
    // let _ = peer.connect();
    // connections.push(peer);
    // vec![];

    // for mut peer in connections {
    //     let request = zeronet_protocol::templates::Handshake::new();
    //     let body = json!(request);
    //     println!("{}", body);

    //     let res = peer
    //         .connection_mut()
    //         .unwrap()
    //         .request("handshake", body)
    //         .await;
    //     // println!("{:?}", res);
    //     let response: templates::Handshake = res.unwrap().body()?;
    //     println!("{:?}", response);
    //     site.peers.insert(response.peer_id.clone(), peer);
    // }
    // site.init_download().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("Loading User");
    let _user = User::load()?;
    let site = "15UYrA7aXr2Nto1Gg4yWXpY3EAJwafMTNk";
    println!("Loading Site : {site}");
    let site = Site::new(site, (*ENV).data_path.clone())?;
    // download_site_(&mut site).await?;
    let mut db_manager = DbManager::new();
    let has_schema = db_manager.has_schema(&site.address());
    let address = site.address();
    if has_schema.0 {
        let _schema = db_manager.load_schema(&address).unwrap();
        db_manager.connect_db(&address);
        db_manager.create_tables(&address);
        db_manager.load_data(&address).await;
    }

    Ok(())
}

async fn _download_site_(site: &mut Site) -> Result<(), Error> {
    let exists = site.content_path().exists();
    if !exists {
        _download_site(site).await?;
    } else {
        // add_peers_to_site(&mut site).await?;
        // site.load_content().await?;
        // let mut peers_cons = vec![];
        // let peers = site.fetch_peers().await?;
        // println!("Found Peers : {:?}", peers);
        // for peer in peers {
        //     let mut peer = Peer::new(PeerAddr::parse(peer).unwrap());
        //     let res = peer.connect();
        //     if let Ok(_) = res {
        //         peers_cons.push(peer);
        //     }
        // }
        // for mut con in peers_cons {
        //     let res = Protocol::new(con.connection_mut().unwrap())
        //         .handshake()
        //         .await?;
        //     println!("Ping Result : {:?} from Peer : {:?}", res, con.address());
        // }
        // let modified = site.content().unwrap().modified;
        // println!("{:?}", modified);
        // site.fetch_changes(1421043090).await?;
        // site.check_site_integrity().await?;
    }
    Ok(())
}

async fn _add_peers_to_site(site: &mut Site) -> Result<(), Error> {
    let peers = _load_peers().await;
    let peers = peers
        .iter()
        .map(|peer| Peer::new(PeerAddr::parse(peer.to_string()).unwrap()))
        .collect::<Vec<_>>();
    for mut peer in peers {
        peer.connect()?;
        let res = Protocol::new(peer.connection_mut().unwrap())
            .handshake()
            .await;
        if let Err(e) = res {
            println!("Error : {:?}", e);
            let peer = peer.clone().address().to_string();
            println!("{}", peer);
        } else {
            let response = res?;
            // println!("Ping Result : {:?}", response);
            site.peers.insert(response.peer_id.clone(), peer);
        }
    }
    Ok(())
}

async fn _download_site(site: &mut Site) -> Result<(), Error> {
    _add_peers_to_site(site).await?;
    println!("Downloading Site");
    site.init_download().await?;
    Ok(())
}

async fn _save_peers(peers: impl Iterator<Item = String>) {
    let mut file = tokio::fs::File::create("data/peers.txt").await.unwrap();
    for peer in peers {
        tokio::io::AsyncWriteExt::write_all(&mut file, peer.as_bytes())
            .await
            .unwrap();
    }
}

async fn _load_peers() -> Vec<String> {
    let mut file = tokio::fs::File::open("data/peers.txt").await.unwrap();
    let mut buf = vec![];
    tokio::io::AsyncReadExt::read_to_end(&mut file, &mut buf)
        .await
        .unwrap();
    let mut peers = vec![];
    for peer in buf.split(|b| (b == b"\n".first().unwrap()) || (b == b"\r".first().unwrap())) {
        peers.push(String::from_utf8(peer.to_vec()).unwrap());
    }
    peers.drain_filter(|peer| !peer.is_empty()).collect()
}
