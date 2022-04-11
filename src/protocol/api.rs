use std::collections::HashMap;

use crate::core::error::Error;
use serde_bytes::ByteBuf;
use zeronet_protocol::templates::*;

#[async_trait::async_trait]
pub trait Request {
    async fn handshake(&mut self) -> Result<Handshake, Error>;
    async fn ping(&mut self) -> Result<bool, Error>;
    async fn get_file(
        &mut self,
        site: String,
        inner_path: String,
        file_size: usize,
        location: usize,
    ) -> Result<GetFileResponse, Error>;
    async fn stream_file(
        &mut self,
        site: String,
        inner_path: String,
    ) -> Result<StreamFileResponse, Error>;
    async fn list_modified(
        &mut self,
        site: String,
        since: usize,
    ) -> Result<ListModifiedResponse, Error>;
    async fn pex(&mut self, site: String) -> Result<PexResponse, Error>;
    async fn update(
        &mut self,
        site: String,
        inner_path: String,
        body: String,
        diffs: HashMap<String, Vec<serde_json::Value>>,
        modified: usize,
    ) -> Result<UpdateFileResponse, Error>;
}

#[async_trait::async_trait]
pub trait Response {
    async fn handshake(&mut self, id: usize) -> Result<bool, Error>;
    async fn ping(&mut self, id: usize) -> Result<bool, Error>;
    async fn get_file(&mut self, id: usize, site: ByteBuf) -> Result<bool, Error>;
    async fn stream_file(
        &mut self,
        id: usize,
        stream_bytes: usize,
        location: usize,
        size: usize,
    ) -> Result<bool, Error>;
    async fn list_modified(
        &mut self,
        id: usize,
        modified_files: HashMap<String, usize>,
    ) -> Result<bool, Error>;
    async fn pex(
        &mut self,
        id: usize,
        peers: Vec<ByteBuf>,
        peers_ipv6: Vec<ByteBuf>,
        peers_onion: Vec<ByteBuf>,
    ) -> Result<bool, Error>;
    async fn update(&mut self, id: usize, msg: &str) -> Result<bool, Error>;
}
