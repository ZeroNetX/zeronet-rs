use std::collections::HashMap;

use serde_bytes::ByteBuf;
use serde_json::json;

use crate::{
    core::error::Error,
    protocol::{
        api::Response as ZeroNetResponse,
        builders::{response::*, *},
        Protocol,
    },
};

#[async_trait::async_trait]
impl<'a> ZeroNetResponse for Protocol<'a> {
    async fn handshake(&mut self, id: usize) -> Result<bool, Error> {
        let builder = handshake();
        self.0.respond(id, json!(builder.1)).await?;
        Ok(true)
    }

    async fn ping(&mut self, id: usize) -> Result<bool, Error> {
        self.0.respond(id, json!({"body":"Pong!"})).await?;
        Ok(true)
    }

    async fn get_file(&mut self, id: usize, body: ByteBuf) -> Result<bool, Error> {
        let builder = get_file(body, 0, 0);
        self.0.respond(id, json!(builder.1)).await?;
        Ok(true)
    }

    async fn stream_file(&mut self, id: usize, stream_bytes: usize) -> Result<bool, Error> {
        let builder = stream_file(stream_bytes);
        self.0.respond(id, json!(builder.1)).await?;
        Ok(true)
    }
    async fn list_modified(
        &mut self,
        id: usize,
        modified_files: HashMap<String, usize>,
    ) -> Result<bool, Error> {
        let builder = list_modified(modified_files);
        self.0.respond(id, json!(builder.1)).await?;
        Ok(true)
    }

    async fn pex(
        &mut self,
        id: usize,
        peers: Vec<ByteBuf>,
        peers_onion: Vec<ByteBuf>,
    ) -> Result<bool, Error> {
        let builder = pex(peers, peers_onion);
        self.0.respond(id, json!(builder.1)).await?;
        Ok(true)
    }
}
