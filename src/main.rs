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
    let mut site = Site::new(site, (*ENV).data_path.clone())?;
    let exists = site.content_path().exists();
    if !exists {
        download_site(&mut site).await?;
    } else {
        site.load_content().await?;
        site.check_site_integrity().await?;
    }

    Ok(())
}

async fn download_site(site: &mut Site) -> Result<(), Error> {
    let peers = load_peers().await;
    let peers = peers
        .iter()
        .map(|peer| Peer::new(PeerAddr::parse(peer.to_string()).unwrap()))
        .collect::<Vec<_>>();
    for mut peer in peers {
        peer.connect()?;
        let res = Protocol::new(peer.connection_mut().unwrap()).ping().await;
        if let Err(e) = res {
            println!("Error : {:?}", e);
            let peer = peer.clone().address().to_string();
            println!("{}", peer);
        } else {
            let response = res?;
            println!("Ping Result : {:?}", response);
            // site.peers.insert(response.peer_id.clone(), peer);
        }
    }
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

async fn load_peers() -> Vec<String> {
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
