pub mod request;
pub mod response;

use zeronet_protocol::ZeroConnection;

pub struct Protocol<'a>(pub(crate) &'a mut ZeroConnection);

impl<'a> Protocol<'a> {
    pub fn new(connection: &'a mut ZeroConnection) -> Self {
        Protocol(connection)
    }
}

use decentnet_protocol::templates::Handshake;

use crate::{environment::ENV, io::utils::current_unix_epoch};

pub fn handshake<'a>() -> (&'a str, Handshake) {
    (
        "handshake",
        Handshake {
            version: (*ENV.version).into(),
            rev: ENV.rev,
            peer_id: (*ENV.peer_id).into(),
            protocol: "v2".into(),
            use_bin_type: true,
            time: current_unix_epoch(),
            fileserver_port: 26117,
            crypt: None,
            crypt_supported: vec![],
            onion: None,
            port_opened: Some(true),
            target_address: None,
        },
    )
}
