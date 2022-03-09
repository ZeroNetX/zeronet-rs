// tracker subfolder and IpPort implementation
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![allow(dead_code)]

pub mod bencode;
pub mod http;
pub mod udp;

use std::{
    io::Error,
    net::{SocketAddr, ToSocketAddrs},
};

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use self::{http::http_announce, udp::udp_announce};
#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct IpPort {
    pub ip: u32,
    pub port: u16,
}

impl IpPort {
    // takes in byte string of ip:port pairs and parses them
    pub fn from_bytes(bytes: &[u8]) -> Vec<Self> {
        let mut peers: Vec<IpPort> = vec![];
        if bytes.len() % 6 != 0 {
            return peers;
        }
        for chunk in bytes.chunks(6) {
            // IpPort is u32 ip, u16 port, 6 bytes
            let peer: IpPort = IpPort {
                // big endian
                ip: u32::from_ne_bytes([chunk[3], chunk[2], chunk[1], chunk[0]]),
                port: u16::from_ne_bytes([chunk[5], chunk[4]]),
            };
            peers.push(peer);
        }

        peers
    }
}

impl std::fmt::Debug for IpPort {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let one: u64 = (u64::from(self.ip) & (0xff << 24)) >> 24;
        let two = (self.ip & (0xff << 16)) >> 16;
        let three = (self.ip & (0xff << 8)) >> 8;
        let four = (self.ip) & 0xff;
        write!(
            f,
            "[ip: {}.{}.{}.{}, port: {}]",
            one, two, three, four, self.port
        )
    }
}

impl std::fmt::Display for IpPort {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let one: u64 = (u64::from(self.ip) & (0xff << 24)) >> 24;
        let two = (self.ip & (0xff << 16)) >> 16;
        let three = (self.ip & (0xff << 8)) >> 8;
        let four = (self.ip) & 0xff;
        write!(f, "{}.{}.{}.{}:{}", one, two, three, four, self.port)
    }
}

// computes info_hash from .torrent bytes
pub fn get_info_hash(addr: String) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(addr.as_bytes());
    hasher.finalize().into()
}

#[derive(Debug, Clone, Copy)]
pub enum Addr {
    Udp(SocketAddr),
    Http(SocketAddr),
}

pub fn make_addr(addr: &str) -> Result<Addr, String> {
    let udp = addr.starts_with("udp");
    let i = addr.find("://");
    let addr = if let Some(i) = i {
        &addr[i + 3..]
    } else {
        addr
    };
    let has_announce = addr.find("/announce");
    let addr = if let Some(i) = has_announce {
        &addr[..i]
    } else {
        addr
    };
    // resolve socketaddr
    match addr.to_socket_addrs().unwrap().next() {
        Some(s) => {
            if udp {
                Ok(Addr::Udp(s))
            } else {
                Ok(Addr::Http(s))
            }
        }
        None => Err("no addr resolved".to_string()),
    }
}

pub async fn announce(addr: Addr, info_hash: [u8; 20], port: u16) -> Result<Vec<IpPort>, Error> {
    match addr {
        Addr::Http(a) => http_announce(a, info_hash, port, None).await,
        Addr::Udp(a) => udp_announce(a, info_hash, port).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::hashes::hex::ToHex;

    #[test]
    fn test_info_hash() {
        let addr = "15UYrA7aXr2Nto1Gg4yWXpY3EAJwafMTNk".to_string();
        let info_hash = get_info_hash(addr);
        let hash = info_hash.to_hex();
        assert_eq!(&hash, "29d191d7caf351ba054a9cb38e8d8477c19bdd1c");
    }

    #[tokio::test]
    async fn test_announce() {
        let tracker_addr = "udp://tracker.0x.tf:6969/announce";
        let site_addr = "15UYrA7aXr2Nto1Gg4yWXpY3EAJwafMTNk".to_string();
        let info_hash = get_info_hash(site_addr.to_string());
        let tracker_addr = make_addr(tracker_addr).unwrap();
        let res = announce(tracker_addr, info_hash, 0).await;
        assert!(res.is_ok());
        println!("{:?}", res.unwrap());
    }
}
