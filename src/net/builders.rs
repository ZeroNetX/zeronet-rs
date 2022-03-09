use std::time::{SystemTime, UNIX_EPOCH};

use serde_bytes::ByteBuf;
use zeronet_protocol::templates::{
    Checkport, FindHashIds, GetFile, GetHashfield, GetPieceFields, Handshake, ListModified, Pex,
    SetHashfield, SetPieceFields, StreamFile, UpdateFile,
};

use crate::environment::ENV;

pub fn build_handshake<'a>() -> (&'a str, Handshake) {
    (
        "handshake",
        Handshake {
            version: (*ENV.version).into(),
            rev: ENV.rev,
            peer_id: (*ENV.peer_id).into(),
            protocol: "v2".into(),
            use_bin_type: true,
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            fileserver_port: 0,
            crypt: None,
            crypt_supported: vec![],
            onion: None,
            port_opened: Some(false),
            target_address: None,
        },
    )
}

///Peer requests

pub fn build_get_file<'a>(
    site: String,
    inner_path: String,
    file_size: usize,
    location: usize,
) -> (&'a str, GetFile) {
    (
        "getFile",
        GetFile {
            site,
            inner_path,
            file_size,
            location,
        },
    )
}

pub fn build_stream_file<'a>(
    _site: String,
    inner_path: String,
    size: usize,
) -> (&'a str, StreamFile) {
    (
        "streamFile",
        StreamFile {
            // site,
            inner_path,
            size,
        },
    )
}

pub fn build_pex<'a>(site: String, need: usize) -> (&'a str, Pex) {
    (
        "pex",
        Pex {
            site,
            peers: vec![],
            peers_onion: vec![],
            need,
        },
    )
}

pub fn build_update_site<'a>(
    site: String,
    inner_path: String,
    body: ByteBuf,
) -> (&'a str, UpdateFile) {
    (
        "update",
        UpdateFile {
            site,
            inner_path,
            body,
            diffs: vec![],
        },
    )
}

pub fn build_list_modified<'a>(site: String, since: usize) -> (&'a str, ListModified) {
    ("listModified", ListModified { site, since })
}

pub fn build_get_hashfield<'a>(site: String) -> (&'a str, GetHashfield) {
    ("getHashfield", GetHashfield { site })
}

pub fn build_set_hashfield<'a>(site: String, hashfield_raw: ByteBuf) -> (&'a str, SetHashfield) {
    (
        "setHashfield",
        SetHashfield {
            site,
            hashfield_raw,
        },
    )
}

pub fn build_find_hash_ids<'a>(site: String, hash_ids: Vec<usize>) -> (&'a str, FindHashIds) {
    ("findHashIds", FindHashIds { site, hash_ids })
}

pub fn build_checkport<'a>(port: u16) -> (&'a str, Checkport) {
    ("checkport", Checkport { port })
}

///Bigfile Plugin
pub fn build_get_piece_fields<'a>(site: String) -> (&'a str, GetPieceFields) {
    ("getPieceFields", GetPieceFields { site })
}

pub fn build_set_piece_fields<'a>(
    site: String,
    piecefields_packed: ByteBuf,
) -> (&'a str, SetPieceFields) {
    (
        "setPieceFields",
        SetPieceFields {
            site,
            piecefields_packed,
        },
    )
}
