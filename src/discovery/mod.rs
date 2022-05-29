pub mod tracker;
use futures::future::join_all;
use log::*;
use zeronet_protocol::PeerAddr;

use crate::{
    core::{discovery::Discovery, error::Error, peer::Peer, site::Site},
    discovery::tracker::{announce, get_info_hash, make_addr},
    environment::ENV,
};

use self::tracker::IpPort;

#[async_trait::async_trait]
impl Discovery for Site {
    //TODO? :: Make this to return stream of peers, instead of full result at once.
    async fn discover(&self) -> Result<Vec<Peer>, Error> {
        info!("Discovering peers");
        let mut res_all = vec![];
        let mut futures = vec![];
        for tracker_addr in (*ENV).trackers.clone() {
            let tracker_addr = make_addr(&tracker_addr).unwrap();
            let info_hash = get_info_hash(self.address().to_string());
            let res = announce(tracker_addr, info_hash, 0);
            futures.push(res);
        }
        let results = join_all(futures).await;
        for res in results {
            if let Err(e) = &res {
                error!("Error : {:?}", e);
            } else {
                let mut _res: Vec<Peer> = res
                    .unwrap()
                    .drain_filter(|a| a.port > 0) //consider ips with no port
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|p: &IpPort| Peer::new(PeerAddr::parse(p.to_string()).unwrap()))
                    .collect();
                res_all.append(&mut _res);
            }
        }
        Ok(res_all)
    }
}
