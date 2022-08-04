use std::fmt::Debug;

use super::error::Error;
use decentnet_protocol::address::PeerAddr as PeerAddress;
use log::*;
use time::{Duration, OffsetDateTime};
use zeronet_protocol::ZeroConnection;

#[derive(Clone)]
pub struct Peer {
    address: PeerAddress,
    connection: Option<ZeroConnection>,
    reputation: isize,
    time_found: OffsetDateTime,
    time_added: OffsetDateTime,
    time_response: OffsetDateTime,
    last_content_json_update: OffsetDateTime,
    download_bytes: usize,
    download_time: Duration,
    bad_files: usize,
    errors: usize,
}

impl Peer {
    pub fn address(&self) -> &PeerAddress {
        &self.address
    }

    pub fn connection(&self) -> Option<&ZeroConnection> {
        self.connection.as_ref()
    }

    pub fn connection_mut(&mut self) -> Option<&mut ZeroConnection> {
        self.connection.as_mut()
    }

    pub fn set_connection(&mut self, connection: ZeroConnection) {
        self.connection = Some(connection);
    }

    pub fn reputation(&self) -> isize {
        self.reputation
    }

    pub fn set_reputation(&mut self, reputation: isize) {
        self.reputation = reputation;
    }

    pub fn time_found(&self) -> OffsetDateTime {
        self.time_found
    }

    pub fn set_time_found(&mut self, time_found: OffsetDateTime) {
        self.time_found = time_found;
    }

    pub fn time_added(&self) -> OffsetDateTime {
        self.time_added
    }

    pub fn set_time_added(&mut self, time_added: OffsetDateTime) {
        self.time_added = time_added;
    }

    pub fn time_response(&self) -> OffsetDateTime {
        self.time_response
    }

    pub fn set_time_response(&mut self, time_response: OffsetDateTime) {
        self.time_response = time_response;
    }

    pub fn last_content_json_update(&self) -> OffsetDateTime {
        self.last_content_json_update
    }
}

impl Debug for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Peer::address: {}", self.address,)
    }
}

impl Peer {
    pub fn new(address: PeerAddress) -> Peer {
        Peer {
            address,
            connection: None,
            reputation: 0,
            time_found: OffsetDateTime::now_utc(),
            time_added: OffsetDateTime::now_utc(),
            time_response: OffsetDateTime::now_utc(),
            last_content_json_update: OffsetDateTime::now_utc(),
            download_bytes: 0,
            download_time: Duration::seconds(0),
            bad_files: 0,
            errors: 0,
        }
    }
    pub fn connect(&mut self) -> Result<(), Error> {
        if self.connection.is_none() {
            let conn = ZeroConnection::from_address(self.address.clone());
            if conn.is_err() {
                trace!(
                    "Failed to establish connection to {}.",
                    self.address.to_string()
                );
            }
            self.connection = Some(conn?);
        }
        Ok(())
    }
}
