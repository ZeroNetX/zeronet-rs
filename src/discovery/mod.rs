pub mod tracker;
use zeronet_protocol::{address::ParseError, PeerAddr};

use crate::{
    core::{discovery::Discovery, error::Error, peer::Peer, site::Site},
    discovery::tracker::{announce, get_info_hash, make_addr},
};

use self::tracker::IpPort;

#[async_trait::async_trait]
impl Discovery for Site {
    async fn discover(&self) -> Result<Vec<Peer>, Error> {
        let tracker_addr = "udp://tracker.0x.tf:6969/announce";
        let tracker_addr = make_addr(tracker_addr).unwrap();
        let info_hash = get_info_hash(self.address().to_string());
        let res = announce(tracker_addr, info_hash, 0).await?;
        let res = res
            .iter()
            .filter_map(|p: &IpPort| {
                if p.port > 0 {
                    Some(PeerAddr::parse(p.to_string()))
                } else {
                    None
                }
            })
            .collect::<Result<Vec<PeerAddr>, ParseError>>()?
            .into_iter()
            .map(|a| Peer::new(a))
            .collect();
        Ok(res)
    }
}
