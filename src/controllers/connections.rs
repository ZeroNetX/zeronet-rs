use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use time::Instant;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{Receiver, Sender},
};

use zeronet_protocol::{
    message::Request as ZeroNetRequest,
    templates::{ErrorResponse, Pex},
    ZeroConnection,
};

use crate::{
    core::error::Error,
    protocol::{api::Response, Protocol},
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
        let listener = TcpListener::bind(format!("{}:{}", "159.65.50.3", 26117))
            .await
            .unwrap();
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
                let peer_addr = stream.0.peer_addr().unwrap().ip().to_string();
                if self.connections.contains_key(&peer_addr) {
                    self.connections
                        .insert(peer_addr.clone(), self.connections[&peer_addr] + 1);
                    tokio::time::sleep(Duration::from_secs(10)).await;
                } else {
                    self.connections.insert(peer_addr, 1);
                }
                let (req_tx, mut req_rx) = tokio::sync::mpsc::channel(64);
                let (res_tx, mut res_rx) = tokio::sync::mpsc::channel(64);
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
            };
        }
    }

    async fn handle_request(&mut self, req: ZeroNetRequest) -> Option<Value> {
        match req.cmd.as_str() {
            "pex" => {
                if let Ok(res) = req.body::<Pex>() {
                    let site = res.site;
                    if self.sites_controller.sites.contains_key(&site) {
                        println!("Sending Empty Pex Res for Site : {}", site);
                        Some(json!({}))
                    } else {
                        println!("Unknown Site : {}", site);
                        let res = ErrorResponse {
                            error: "Unknown site".to_string(),
                        };
                        Some(json!(res))
                    }
                } else {
                    println!("Invalid Pex Request {:?}", req);
                    None
                }
            }
            _ => {
                println!("Unknown cmd {}", req.cmd);
                None
            }
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
        let _res = {
            loop {
                let request = connection.recv().await;
                if let Ok(request) = request {
                    let mut protocol = Protocol::new(&mut connection);
                    match request.cmd.as_str() {
                        "handshake" => {
                            let res = protocol.handshake(request.req_id).await;
                            if res.is_err() {
                                println!(
                                    "Error Sending Response: \nTo : {} : {:#?}",
                                    peer_addr.to_string(),
                                    res.unwrap_err()
                                );
                            }
                        }
                        _ => {
                            println!(
                                "\nFrom : {} : {} : {}",
                                peer_addr.to_string(),
                                serde_json::to_string_pretty(&request.cmd).unwrap(),
                                serde_json::to_value(request.req_id).unwrap()
                            );
                            let time = Instant::now();
                            let _ = req_tx.send(request.clone()).await;
                            let res = res_tx.recv().await;
                            let took = time.elapsed();
                            println!("{} Req {} took : {}", &request.cmd, &request.req_id, took);
                            if let Some(res) = res {
                                let _ = protocol.0.respond(request.req_id, res);
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
}
