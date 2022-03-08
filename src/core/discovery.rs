use zeronet_protocol::PeerAddr;

use crate::discovery::tracker::IpPort;

use super::{error::Error, peer::Peer};

#[async_trait::async_trait]
pub trait Discovery {
    async fn discover(&self) -> Result<Vec<Peer>, Error>;
}

impl From<IpPort> for PeerAddr {
    fn from(ip_port: IpPort) -> PeerAddr {
        PeerAddr::parse(ip_port.to_string()).unwrap()
    }
}
