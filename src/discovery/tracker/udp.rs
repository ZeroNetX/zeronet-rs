// udp tracker functionality
#![allow(dead_code)]

use super::IpPort;

use std::{io::Error, net::SocketAddr, vec};

use tokio::net::UdpSocket;

use rand::random;
use serde::{Deserialize, Serialize};

// literal magic number used for handshake
const MAGIC: u64 = 0x0417_2710_1980;
// # of peers to request
const PEERS: usize = 32;

// structs to be (de)serialized and sent/received
#[derive(Serialize, Deserialize, Debug)]
struct ConnectReq {
    protocol_id: u64,
    action: u32,
    transaction_id: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct ConnectResp {
    action: u32,
    transaction_id: u32,
    connection_id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnnounceReq {
    connection_id: u64,
    action: u32,
    transaction_id: u32,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    downloaded: u64,
    left: u64,
    uploaded: u64,
    event: u32,
    ip_address: u32,
    key: u32,
    num_want: u32,
    port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnnounceResp {
    action: u32,
    transaction_id: u32,
    interval: u32,
    leechers: u32,
    seeders: u32,
}

impl AnnounceResp {
    fn from_be(&mut self) -> &mut Self {
        self.action = u32::from_be(self.action);
        self.transaction_id = u32::from_be(self.transaction_id);
        self.interval = u32::from_be(self.interval);
        self.leechers = u32::from_be(self.leechers);
        self.seeders = u32::from_be(self.seeders);

        self
    }
}

// announces to udp tracker, gets vector of ip and ports
pub async fn udp_announce(
    addr: SocketAddr,
    info_hash: [u8; 20],
    port: u16,
) -> Result<Vec<IpPort>, Error> {
    // set up udp socket
    let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();

    // init structs and serialize
    let conreq = ConnectReq {
        protocol_id: u64::to_be(MAGIC),
        action: 0,
        transaction_id: random::<u32>(),
    };
    let mut conresp = ConnectResp {
        action: 0,
        transaction_id: 0,
        connection_id: 0,
    };
    let mut serreq: Vec<u8> = bincode::serialize(&conreq).unwrap();
    let mut serresp: Vec<u8> = bincode::serialize(&conresp).unwrap();

    // send connection request and get response
    socket.send_to(&serreq, addr).await?;
    socket.recv_from(&mut serresp).await?;

    // deserialize struct and check tx id
    conresp = bincode::deserialize(&serresp).unwrap();

    // init structs and serialize
    let announce_req = AnnounceReq {
        connection_id: conresp.connection_id,
        action: u32::to_be(1),
        transaction_id: random::<u32>(),
        info_hash,
        peer_id: [1; 20],
        downloaded: 0,
        left: 0,
        uploaded: 0,
        event: 0,
        ip_address: 0,
        key: 0,
        num_want: u32::to_be(200),
        port: u16::to_be(port),
    };
    serreq = bincode::serialize(&announce_req).unwrap();
    let mut resp_buf = vec![0_u8; 32767];

    // send announce request and get response
    socket.send_to(&serreq, addr).await?;
    let bytes = socket.recv_from(&mut resp_buf).await?.0;

    resp_buf.truncate(bytes);
    resp_buf.drain(0..20);

    // deserialize and return peers
    Ok(IpPort::from_bytes(&resp_buf))
}
