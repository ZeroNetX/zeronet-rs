use log::{debug, error};
use serde_bytes::ByteBuf;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use time::Instant;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{channel, Receiver, Sender},
};
use zeronet_protocol::{templates::GetHashfield, PeerAddr};

use zeronet_protocol::{
    message::Request as ZeroNetRequest,
    templates::{ErrorResponse, Pex},
    ZeroConnection,
};

use crate::{
    core::{error::Error, peer::Peer},
    environment::ENV,
    protocol::{api::Response, builders, Protocol},
};

use super::sites::SitesController;

pub struct ConnectionController {
    listener: TcpListener,
    sites_controller: SitesController,
    conn_len: usize,
    connections: HashMap<String, usize>,
}

impl ConnectionController {
    pub async fn new(sites_controller: SitesController) -> Self {
        let ser_addr = format!("{}:{}", ENV.fileserver_ip, ENV.fileserver_port);
        println!("Listening on {}", ser_addr);
        let listener = TcpListener::bind(ser_addr).await.unwrap();
        Self {
            listener,
            sites_controller,
            conn_len: 0,
            connections: HashMap::new(),
        }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        loop {
            let incoming = self.listener.accept().await;
            if let Ok(stream) = incoming {
                self.conn_len += 1;
                if let Ok(peer_addr) = stream.0.peer_addr() {
                    let peer_addr = peer_addr.ip().to_string();
                    if self.connections.contains_key(&peer_addr) {
                        self.connections
                            .insert(peer_addr.clone(), self.connections[&peer_addr] + 1);
                        tokio::time::sleep(Duration::from_secs(10)).await;
                    } else {
                        self.connections.insert(peer_addr, 1);
                    }
                    let (req_tx, mut req_rx) = channel(64);
                    let (res_tx, mut res_rx) = channel(64);
                    tokio::spawn(async move {
                        let _ = Self::handle_connection(stream.0, req_tx, &mut res_rx).await;
                    });
                    let msg = req_rx.recv().await;
                    if let Some(req) = msg {
                        let res = self.handle_request(req).await;
                        if let Some(v) = res {
                            let _ = res_tx.send(v).await;
                        }
                    }
                } else {
                    error!("Error : {}", stream.0.peer_addr().unwrap_err());
                }
            };
        }
    }

    async fn handle_connection(
        stream: TcpStream,
        req_tx: Sender<ZeroNetRequest>,
        res_tx: &mut Receiver<Value>,
    ) -> Result<(), Error> {
        let peer_addr = stream.peer_addr()?;
        let stream = stream.into_std().unwrap();
        let mut connection =
            ZeroConnection::new(Box::new(stream.try_clone().unwrap()), Box::new(stream))?;
        let _ = {
            loop {
                let request = connection.recv().await;
                if let Ok(request) = request {
                    let mut protocol = Protocol::new(&mut connection);
                    match request.cmd.as_str() {
                        "handshake" => {
                            let res = protocol.handshake(request.req_id).await;
                            if res.is_err() {
                                error!(
                                    "Error Sending Response: \nTo : {} : {:#?}",
                                    peer_addr,
                                    res.unwrap_err()
                                );
                            }
                        }
                        _req => {
                            println!(
                                "\nFrom : {} : {} : {}",
                                peer_addr,
                                serde_json::to_string_pretty(&request.cmd).unwrap(),
                                serde_json::to_value(request.req_id).unwrap()
                            );
                            let time = Instant::now();
                            //TODO! Optimisation
                            //? For Unknown Sites, send direct Error Response instead for channel roundtrip
                            let _ = req_tx.send(request.clone()).await;
                            let res = res_tx.recv().await;
                            let took = time.elapsed();
                            println!("{} Req {} took : {}", &request.cmd, &request.req_id, took);
                            if let Some(res) = res {
                                let result = protocol.0.respond(request.req_id, res.clone()).await;
                                if result.is_err() {
                                    error!(
                                        "Error Sending Response: \nTo : {} : {:#?}",
                                        peer_addr.to_string(),
                                        result.unwrap_err()
                                    );
                                } else {
                                    debug!(
                                        "Sent Response {}",
                                        serde_json::to_string(&res).unwrap()
                                    );
                                }
                            }
                        }
                    }
                } else {
                    print!(".");
                    break;
                }
            }
        };

        Ok(())
    }

    async fn handle_request(&mut self, req: ZeroNetRequest) -> Option<Value> {
        match req.cmd.as_str() {
            "pex" => self.handle_pex(req),
            "getHashfield" => self.get_hashfield(req),
            _ => {
                println!("Unknown cmd {}", req.cmd);
                None
            }
        }
    }
}

impl ConnectionController {
    fn unknown_site_response() -> Option<Value> {
        let res = ErrorResponse {
            error: "Unknown site".to_string(),
        };
        Some(json!(res))
    }

    fn handle_pex(&mut self, req: ZeroNetRequest) -> Option<Value> {
        if let Ok(res) = req.body::<Pex>() {
            let site = &res.site;
            let need = res.need;
            if self.sites_controller.sites.contains_key(site) {
                let mut peers = res.peers;
                if let Some(peers_onion) = res.peers_onion {
                    peers.extend(peers_onion);
                }
                if let Some(peers_ipv6) = res.peers_ipv6 {
                    peers.extend(peers_ipv6);
                }
                let mut pex_peers = vec![];
                for peer in peers {
                    let addr = PeerAddr::unpack(peer.as_slice());
                    if let Ok(addr) = addr {
                        let peer = Peer::new(addr);
                        pex_peers.push(peer);
                    }
                }
                let site = self.sites_controller.sites.get_mut(site).unwrap();
                let keys = pex_peers
                    .iter()
                    .map(|p| p.address().to_string())
                    .collect::<Vec<_>>();
                for peer in pex_peers {
                    site.add_peer(peer);
                }
                //TODO! Only send connectable peers instead of all peers
                let mut retrived = 0;
                let peers = site
                    .peers
                    .iter()
                    .filter_map(|(key, peer)| {
                        if keys.contains(key) {
                            None
                        } else if retrived < need {
                            retrived += 1;
                            let packed = peer.clone().address().pack();
                            match peer.address() {
                                PeerAddr::IPV4(_, _) => Some(("ipv4", packed)),
                                PeerAddr::IPV6(_, _) => Some(("ipv6", packed)),
                                PeerAddr::OnionV2(_, _) | PeerAddr::OnionV3(_, _) => {
                                    Some(("onion", packed))
                                }
                                _ => unimplemented!(),
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let mut ip_v4 = vec![];
                let mut ip_v6 = vec![];
                let mut onion = vec![];
                for (key, packed) in peers {
                    match key {
                        "ipv4" => ip_v4.push(ByteBuf::from(packed)),
                        "ipv6" => ip_v6.push(ByteBuf::from(packed)),
                        "onion" => onion.push(ByteBuf::from(packed)),
                        _ => unimplemented!(),
                    }
                }
                Some(json!(builders::response::pex(ip_v4, ip_v6, onion)))
            } else {
                Self::unknown_site_response()
            }
        } else {
            error!("Invalid Pex Request {:?}", req);
            None
        }
    }

    fn get_hashfield(&mut self, req: ZeroNetRequest) -> Option<Value> {
        if let Ok(res) = req.body::<GetHashfield>() {
            let site = &res.site;
            if self.sites_controller.sites.contains_key(site) {
                unimplemented!();
            } else {
                Self::unknown_site_response()
            }
        } else {
            error!("Invalid GetHashfield Request {:?}", req);
            None
        }
    }
}
