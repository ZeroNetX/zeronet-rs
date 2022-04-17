use itertools::Itertools;
use log::{debug, error};
use serde_bytes::ByteBuf;
use std::io::Read;
use std::time::Duration;
use std::{collections::HashMap, fs::File};
use time::Instant;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{channel, Receiver, Sender},
};

use zeronet_protocol::{
    message::{Request as ZeroNetRequest, ResponseType},
    templates::*,
    PeerAddr, ZeroConnection,
};
use zerucontent::Content;

use crate::{
    core::{error::Error, io::SiteIO, peer::Peer},
    environment::ENV,
    protocol::{
        api::{self, Response},
        builders, Protocol,
    },
    SitesController,
};

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
                        let _ = res_tx.send(res).await;
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
        res_rx: &mut Receiver<ResponseType>,
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
                        "ping" => {
                            let res = protocol.ping(request.req_id).await;
                            if res.is_err() {
                                error!(
                                    "Error Sending Response: \nTo : {} : {:#?}",
                                    peer_addr,
                                    res.unwrap_err()
                                );
                            }
                        }
                        cmd => {
                            debug!(
                                "\nFrom : {} : {} : {}",
                                peer_addr,
                                serde_json::to_string_pretty(&request.cmd).unwrap(),
                                serde_json::to_value(request.req_id).unwrap()
                            );
                            let time = Instant::now();
                            //TODO! Optimisation
                            //? For Unknown Sites, send direct Error Response instead for channel roundtrip
                            if cmd == "update" {
                                let body = request.body::<Update>();
                                if let Ok(res) = body {
                                    let is_body_empty = res.body.is_empty();
                                    let site = res.site;
                                    if is_body_empty {
                                        let res = api::Request::get_file(
                                            &mut protocol,
                                            site,
                                            res.inner_path,
                                            0,
                                            0,
                                        )
                                        .await;
                                        if let Ok(res) = res {
                                            let _bytes = res.body;
                                        }
                                    }
                                } else {
                                    error!(
                                        "Error Parsing Request Body: \nTo : {} : {:#?}",
                                        peer_addr,
                                        body.unwrap_err()
                                    );
                                }
                            }
                            let _ = req_tx.send(request.clone()).await;
                            let res = res_rx.recv().await;
                            let took = time.elapsed();
                            debug!("{} Req {} took : {}", &request.cmd, &request.req_id, took);
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

    async fn handle_request(&mut self, req: ZeroNetRequest) -> ResponseType {
        match req.cmd.as_str() {
            "pex" => self.handle_pex(req),
            "getHashfield" => self.get_hashfield(req),
            "getFile" => self.handle_get_file(req, false),
            "streamFile" => self.handle_get_file(req, true),
            "update" => self.handle_update(req),
            _ => {
                println!("Unknown cmd {}", req.cmd);
                ResponseType::UnknownCmd
            }
        }
    }
}

impl ConnectionController {
    fn unknown_site_response() -> ResponseType {
        let res = ErrorResponse {
            error: "Unknown site".to_string(),
        };
        ResponseType::Err(res)
    }

    fn handle_pex(&mut self, req: ZeroNetRequest) -> ResponseType {
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
                ResponseType::Pex(builders::response::pex(ip_v4, ip_v6, onion))
            } else {
                Self::unknown_site_response()
            }
        } else {
            error!("Invalid Pex Request {:?}", req);
            ResponseType::InvalidRequest
        }
    }

    fn get_hashfield(&mut self, req: ZeroNetRequest) -> ResponseType {
        if let Ok(res) = req.body::<GetHashfield>() {
            let site = &res.site;
            if self.sites_controller.sites.contains_key(site) {
                unimplemented!();
            } else {
                Self::unknown_site_response()
            }
        } else {
            error!("Invalid GetHashfield Request {:?}", req);
            ResponseType::InvalidRequest
        }
    }

    fn handle_get_file(&mut self, req: ZeroNetRequest, streaming: bool) -> ResponseType {
        if let Ok(res) = req.body::<GetFile>() {
            let site = &res.site;
            if self.sites_controller.sites.contains_key(site) {
                let site = self.sites_controller.sites.get_mut(site).unwrap();
                let inner_path = &res.inner_path;
                match site.get_path(inner_path) {
                    Ok(path) => {
                        let location = res.location;
                        let read_bytes = res.read_bytes.unwrap_or(512 * 1024);
                        let file_size = res.file_size;
                        let file = File::open(path).unwrap();
                        let bytes = file.bytes();
                        let file_size_actual = bytes.size_hint().1.unwrap();
                        if location > file_size_actual {
                            ResponseType::Err(ErrorResponse {
                                error: "File read error, Bad file location".to_string(),
                            })
                        } else if file_size > file_size_actual {
                            ResponseType::Err(ErrorResponse {
                                error: "File read error, Bad file size".to_string(),
                            })
                        } else if read_bytes > file_size_actual {
                            ResponseType::Err(ErrorResponse {
                                error: "File read error, File size does not match".to_string(),
                            })
                        } else {
                            let bytes = bytes
                                .skip(location)
                                .take(read_bytes)
                                .filter_map(|a| a.ok())
                                .collect_vec();
                            if streaming {
                                ResponseType::StreamFile(
                                    builders::response::stream_file(
                                        bytes.len(),
                                        location,
                                        file_size,
                                    ),
                                    ByteBuf::from(bytes),
                                )
                            } else {
                                ResponseType::GetFile(builders::response::get_file(
                                    ByteBuf::from(bytes),
                                    file_size_actual,
                                    location + read_bytes,
                                ))
                            }
                        }
                    }
                    Err(err) => ResponseType::Err(ErrorResponse {
                        error: format!("{:?}", err),
                    }),
                }
            } else {
                Self::unknown_site_response()
            }
        } else {
            error!("Invalid GetFile Request {:?}", req);
            ResponseType::InvalidRequest
        }
    }

    fn handle_update(&mut self, req: ZeroNetRequest) -> ResponseType {
        if let Ok(res) = req.body::<Update>() {
            let site = &res.site;
            if self.sites_controller.sites.contains_key(site) {
                let inner_path = &res.inner_path;
                let content_modified = res.modified;
                if !inner_path.ends_with("content.json") {
                    return ResponseType::Err(ErrorResponse {
                        error: "Only content.json update allowed".to_string(),
                    });
                }
                let validate_content = {
                    let site = self.sites_controller.sites.get(site).unwrap();
                    if !site.inner_content_exists(inner_path) {
                        false
                    } else {
                        let exists_in_includes = site
                            .content(None)
                            .unwrap()
                            .includes
                            .keys()
                            .any(|k| k.ends_with(inner_path));
                        if exists_in_includes {
                            true
                        } else {
                            let site_content_modified =
                                site.content(Some(inner_path)).unwrap().modified;
                            content_modified > site_content_modified
                        }
                    }
                };
                if !validate_content {
                    return ResponseType::Ok(OkResponse {
                        ok: "File not changed".to_string(),
                    });
                }
                let body = res.body;
                if body.is_empty() {
                    unimplemented!()
                }
                let content = Content::from_buf(body);
                //TODO!
                if let Ok(_content) = content {
                    let _site = self.sites_controller.sites.get_mut(site).unwrap();
                    ResponseType::Ok(OkResponse {
                        ok: "File updated".to_string(),
                    })
                } else {
                    ResponseType::Err(ErrorResponse {
                        error: "File invalid JSON".to_string(),
                    })
                }
            } else {
                Self::unknown_site_response()
            }
        } else {
            error!("Invalid Update Request {:?}", req);
            ResponseType::InvalidRequest
        }
    }
}
