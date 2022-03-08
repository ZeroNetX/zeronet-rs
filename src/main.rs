#[allow(unused)]
pub mod core;
pub mod discovery;
pub mod environment;
pub mod io;
pub mod utils;

// use core::

use environment::ENV;
use serde_json::json;
use zeronet_protocol::{templates, PeerAddr};

use crate::core::{
    discovery::Discovery,
    error::Error,
    peer::Peer,
    site::{Site, SiteIO},
    user::{User, UserIO},
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _user = User::load()?;
    let mut site = Site::new(
        "15UYrA7aXr2Nto1Gg4yWXpY3EAJwafMTNk",
        (*ENV).data_path.clone(),
    )?;
    // let peers
    // site.discover().await?;
    let mut connections = vec![];
    let mut peer = Peer::new(PeerAddr::IPV4([127, 0, 0, 1], 11917));
    peer.connect();
    connections.push(peer);
    // vec![];
    // for mut peer in peers {
    //     let res = peer.connect();
    //     if let Err(e) = &res {
    //         println!("{:?}", e);
    //     } else {
    //         println!("Connection Successful");
    //         connections.push(peer);
    //     }
    // }

    for mut peer in connections {
        let request = zeronet_protocol::templates::Handshake::new();
        let body = json!(request);
        println!("{}", body);

        let res = peer
            .connection_mut()
            .unwrap()
            .request("handshake", body)
            .await;
        // println!("{:?}", res);
        let response: templates::Handshake = res.unwrap().body()?;
        println!("{:?}", response);
        site.peers.insert(response.peer_id.clone(), peer);
    }
    site.init_download().await?;
    Ok(())
}
