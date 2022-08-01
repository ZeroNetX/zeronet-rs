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

    // pub fn request(
    // 	&mut self,
    // 	cmd: &str,
    // 	params: serde_json::Value,
    // ) -> Result<ByteBuf, Error> {
    // 	self.connect()?;
    // 	if let Some(connection) = &mut self.connection {
    // 		let msg = PeerMessage {
    // 			cmd: cmd.to_string(),
    // 			to: None,
    // 			req_id: Some(1),
    // 			params,
    // 			body: ByteBuf::new(),
    // 			peers: vec![],
    // 		};
    // 		let response = connection.request(msg);
    // 		return match response {
    // 			Err(err) => {
    // 				error!("Invalid response: {:?}", err);
    // 				Err(())
    // 			}
    // 			Ok(res) => Ok(res.body),
    // 		};
    // 	}
    // 	Err(())
    // }
    // pub fn get_file(
    // 	&mut self,
    // 	address: &SiteAddress,
    // 	inner_path: &String,
    // ) -> Result<ByteBuf, ()> {
    // 	warn!("Get file is not fully implemented");
    // 	let mut params = HashMap::new();
    // 	params.insert("site", json!(address.to_string()));
    // 	params.insert("location", json!(0));
    // 	params.insert("inner_path", json!(inner_path));
    // 	// {'cmd': 'getHashfield', 'req_id': 1, 'params': {'site': '1CWkZv7fQAKxTVjZVrLZ8VHcrN6YGGcdky'}}
    // 	return self.request("getFile", json!(params));
    // }
    // pub fn ping(&mut self) -> Result<(), ()> {
    // 	let res = self.request("ping", serde_json::Value::Null)?;
    // 	println!("{:?}", res);
    // 	Ok(())
    // }
    // fn pex() {}
    // fn list_modified() {}
    // fn update_hashfield() {}
    // fn find_hash_ids() {}
    // fn send_my_hashfield() {}
    // fn publish() {}
    // fn remove() {}
    // fn on_connection_error() {}
    // fn on_worker_done() {}
}

// impl Actor for Peer {
//     type Context = Context<Self>;
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub enum PeerCommand {
//     StreamFile,
//     GetFile,
//     GetHashfield,
//     Response,
//     Handshake,
//     Ping,
// }

// #[derive(Serialize, Deserialize, Debug, Default)]
// pub struct PeerMessage {
//     cmd: String,
//     #[serde(default, skip_serializing_if = "is_default")]
//     req_id: Option<usize>,
//     #[serde(default, skip_serializing_if = "is_default")]
//     to: Option<usize>,
//     #[serde(default, skip_serializing_if = "is_default")]
//     params: serde_json::Value,
//     #[serde(default, skip_serializing_if = "is_default")]
//     body: ByteBuf,
//     #[serde(default, skip_serializing_if = "is_default")]
//     peers: Vec<HashMap<String, ByteBuf>>,
// }

// pub struct FileGetRequest {
//     pub inner_path: String,
//     pub site_address: SiteAddress,
// }

// impl Message for FileGetRequest {
//     type Result = Result<ByteBuf, Error>;
// }

// impl Handler<FileGetRequest> for Peer {
//     type Result = Result<ByteBuf, Error>;
//     fn handle(&mut self, msg: FileGetRequest, _ctx: &mut Context<Self>) -> Self::Result {
//         self.connect()?;
//         let mut conn = match &mut self.connection {
//             Some(conn) => conn,
//             None => return Err(Error::MissingError),
//         };
//         trace!(
//             "Requesting 'zero://{}/{}' from {}",
//             msg.site_address,
//             msg.inner_path,
//             self.address.to_string()
//         );
//         let request = zeronet_protocol::templates::GetFile {
//             site: msg.site_address.to_string(),
//             inner_path: msg.inner_path,
//             location: 0,
//             file_size: 0,
//         };
//         let res = block_on(conn.request("getFile", json!(request)))?;
//         let response: templates::GetFileResponse = res.body()?;
//         return Ok(response.body);
//     }
// }

// impl Handler<Announce> for Peer {
//     type Result = Result<templates::AnnounceResponse, Error>;
//     fn handle(&mut self, msg: Announce, _ctx: &mut Context<Self>) -> Self::Result {
//         self.connect()?;
//         let mut conn = match &mut self.connection {
//             Some(conn) => conn,
//             None => return Err(Error::MissingError),
//         };
//         let res = block_on(conn.request("announce", msg.req))?;
//         let response: templates::AnnounceResponse = res.body()?;
//         Ok(response)
//     }
// }
