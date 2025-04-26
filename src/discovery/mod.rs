pub mod tracker;
use futures::future::join_all;
use log::*;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

use decentnet_protocol::{address::PeerAddr, interface::RequestImpl};

use crate::{
    core::{discovery::Discovery, error::Error, peer::Peer, site::Site},
    discovery::tracker::{announce, get_info_hash, make_addr},
    environment::ENV,
    io::utils::load_peers,
    net::Protocol,
};

use self::tracker::IpPort;

#[async_trait::async_trait]
impl Discovery for Site {
    //TODO? :: Make this to return stream of peers, instead of full result at once.
    async fn discover(&self) -> Result<Vec<Peer>, Error> {
        info!("Discovering peers");
        let mut res_all = vec![];
        let mut futures = vec![];
        for tracker_addr in ENV.trackers.clone() {
            match make_addr(&tracker_addr) {
                Ok(tracker_addr) => {
                    let info_hash = get_info_hash(self.address().to_string());
                    let res = announce(tracker_addr, info_hash, 0, &ENV.peer_id);
                    futures.push(res);
                }
                Err(err) => {
                    error!("Error : {err}");
                }
            }
        }
        let results = join_all(futures).await;
        for res in results {
            if let Err(e) = &res {
                error!("Error : {:?}", e);
            } else {
                let mut _res: Vec<Peer> = res
                    .unwrap()
                    .extract_if(.., |a| a.port > 1) //consider ips with no port
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

impl Site {
    pub async fn find_peers(&mut self) -> Result<Vec<Peer>, Error> {
        let peers = self.discover().await?;
        let mut peers = peers
            .into_iter()
            .chain(
                load_peers()
                    .await
                    .iter()
                    .map(|peer| Peer::new(PeerAddr::parse(peer.to_string()).unwrap())),
            )
            .collect::<Vec<_>>();
        let mut connections = peers
            .par_iter_mut()
            .filter_map(|peer| {
                let res = peer.connect();
                if let Err(e) = &res {
                    error!("Error : {:?}", e);
                    let peer = peer.clone().address().to_string();
                    error!("{}", peer);
                    None
                } else {
                    info!("Connection Successful to {:?}", peer);
                    Some(peer)
                }
            })
            .collect::<Vec<_>>();
        let valid_connections = connections
            .iter_mut()
            .map(|peer| async {
                let res = Protocol::new((*peer).connection_mut().unwrap())
                    .handshake()
                    .await;
                if let Err(e) = res {
                    let peer = &peer.address().to_string();
                    error!("Error on Handshake: {:?} with Peer {:?}", e, peer);
                    None
                } else {
                    Some((*peer).clone())
                }
            })
            .collect::<Vec<_>>();
        let valid_connections = join_all(valid_connections)
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        Ok(valid_connections)
    }
}
