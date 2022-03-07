#[allow(unused)]
pub mod core;
pub mod discovery;
pub mod environment;
pub mod io;
pub mod utils;

use environment::ENV;

use crate::core::{
    discovery::Discovery,
    error::Error,
    site::Site,
    user::{User, UserIO},
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _user = User::load()?;
    let site = Site::new(
        "15UYrA7aXr2Nto1Gg4yWXpY3EAJwafMTNk",
        (*ENV).data_path.clone(),
    )?;
    let peers = site.discover().await?;
    let mut connections = vec![];
    for mut peer in peers {
        let res = peer.connect();
        if let Err(e) = &res {
            println!("{:?}", e);
        } else {
            println!("Connection Successful");
            connections.push(peer);
        }
    }
    println!("{:?}", connections.len());
    Ok(())
}
