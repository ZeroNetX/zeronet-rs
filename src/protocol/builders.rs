use crate::{environment::ENV, io::utils::current_unix_epoch};
use zeronet_protocol::templates::*;

pub fn handshake<'a>() -> (&'a str, Handshake) {
    (
        "handshake",
        Handshake {
            version: "0.7.2".into(), //(*ENV.version).into(),
            rev: 4555,               //ENV.rev,
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

pub mod request {
    use std::collections::HashMap;

    use serde_bytes::ByteBuf;
    use serde_json::Value;
    use zeronet_protocol::templates::*;

    ///Peer requests
    pub fn get_file<'a>(
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

    pub fn stream_file<'a>(
        _site: String,
        inner_path: String,
        size: usize,
    ) -> (&'a str, StreamFile) {
        ("streamFile", StreamFile { inner_path, size })
    }

    pub fn pex<'a>(site: String, need: usize) -> (&'a str, Pex) {
        (
            "pex",
            Pex {
                site,
                peers: vec![],
                peers_onion: Some(vec![]),
                need,
            },
        )
    }

    pub fn update_site<'a>(
        site: String,
        inner_path: String,
        body: String,
        diffs: HashMap<String, Vec<Value>>,
    ) -> (&'a str, UpdateFile) {
        (
            "update",
            UpdateFile {
                site,
                inner_path,
                body,
                diffs,
            },
        )
    }

    pub fn list_modified<'a>(site: String, since: usize) -> (&'a str, ListModified) {
        ("listModified", ListModified { site, since })
    }

    pub fn get_hashfield<'a>(site: String) -> (&'a str, GetHashfield) {
        ("getHashfield", GetHashfield { site })
    }

    pub fn set_hashfield<'a>(site: String, hashfield_raw: ByteBuf) -> (&'a str, SetHashfield) {
        (
            "setHashfield",
            SetHashfield {
                site,
                hashfield_raw,
            },
        )
    }

    pub fn find_hash_ids<'a>(site: String, hash_ids: Vec<usize>) -> (&'a str, FindHashIds) {
        ("findHashIds", FindHashIds { site, hash_ids })
    }

    pub fn checkport<'a>(port: u16) -> (&'a str, Checkport) {
        ("checkport", Checkport { port })
    }

    ///Bigfile Plugin
    pub fn get_piece_fields<'a>(site: String) -> (&'a str, GetPieceFields) {
        ("getPieceFields", GetPieceFields { site })
    }

    pub fn set_piece_fields<'a>(
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
}

pub mod response {
    use serde_bytes::ByteBuf;
    use std::collections::HashMap;
    use zeronet_protocol::templates::*;

    ///Peer requests
    pub fn get_file<'a>(body: ByteBuf, size: usize, location: usize) -> (&'a str, GetFileResponse) {
        (
            "getFile",
            GetFileResponse {
                body,
                size,
                location,
            },
        )
    }

    pub fn stream_file<'a>(stream_bytes: usize) -> (&'a str, StreamFileResponse) {
        ("streamFile", StreamFileResponse { stream_bytes })
    }

    pub fn pex<'a>(peers: Vec<ByteBuf>, peers_onion: Vec<ByteBuf>) -> (&'a str, PexResponse) {
        ("pex", PexResponse { peers, peers_onion })
    }

    pub fn update_site<'a>(ok: String) -> (&'a str, UpdateFileResponse) {
        ("update", UpdateFileResponse { ok })
    }

    pub fn list_modified<'a>(
        modified_files: HashMap<String, usize>,
    ) -> (&'a str, ListModifiedResponse) {
        ("listModified", ListModifiedResponse { modified_files })
    }

    pub fn get_hashfield<'a>(hashfield_raw: ByteBuf) -> (&'a str, GetHashfieldResponse) {
        ("getHashfield", GetHashfieldResponse { hashfield_raw })
    }

    pub fn set_hashfield<'a>(ok: bool) -> (&'a str, SetHashfieldResponse) {
        ("setHashfield", SetHashfieldResponse { ok })
    }

    pub fn find_hash_ids<'a>(
        peers: HashMap<usize, Vec<ByteBuf>>,
        peers_onion: HashMap<usize, Vec<ByteBuf>>,
    ) -> (&'a str, FindHashIdsResponse) {
        ("findHashIds", FindHashIdsResponse { peers, peers_onion })
    }

    pub fn checkport<'a>(status: String, ip_external: String) -> (&'a str, CheckportResponse) {
        (
            "checkport",
            CheckportResponse {
                status,
                ip_external,
            },
        )
    }

    ///Bigfile Plugin
    pub fn get_piece_fields<'a>(piecefields_packed: ByteBuf) -> (&'a str, GetPieceFieldsResponse) {
        (
            "getPieceFields",
            GetPieceFieldsResponse { piecefields_packed },
        )
    }

    pub fn set_piece_fields<'a>(ok: bool) -> (&'a str, SetPieceFieldsResponse) {
        ("setPieceFields", SetPieceFieldsResponse { ok })
    }
}
