use std::collections::HashMap;

use serde_bytes::ByteBuf;
use zeronet_protocol::{message::ResponseType, templates::*};

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
        self.0
            .respond(id, ResponseType::Handshake(builder.1))
            .await?;
        Ok(true)
    }

    async fn ping(&mut self, id: usize) -> Result<bool, Error> {
        self.0
            .respond(
                id,
                ResponseType::Ping(PingResponse {
                    body: "Pong!".into(),
                }),
            )
            .await?;
        Ok(true)
    }

    async fn get_file(
        &mut self,
        id: usize,
        body: ByteBuf,
        size: usize,
        location: usize,
    ) -> Result<bool, Error> {
        let builder = get_file(body, size, location);
        self.0.respond(id, ResponseType::GetFile(builder)).await?;
        Ok(true)
    }

    async fn stream_file(
        &mut self,
        id: usize,
        stream_bytes: usize,
        location: usize,
        size: usize,
        bytes: ByteBuf,
    ) -> Result<bool, Error> {
        let builder = stream_file(stream_bytes, location, size);
        self.0
            .respond(id, ResponseType::StreamFile(builder, bytes))
            .await?;
        Ok(true)
    }
    async fn list_modified(
        &mut self,
        id: usize,
        modified_files: HashMap<String, usize>,
    ) -> Result<bool, Error> {
        let builder = list_modified(modified_files);
        self.0
            .respond(id, ResponseType::ListModified(builder))
            .await?;
        Ok(true)
    }

    async fn pex(
        &mut self,
        id: usize,
        peers: Vec<ByteBuf>,
        peers_ipv6: Vec<ByteBuf>,
        peers_onion: Vec<ByteBuf>,
    ) -> Result<bool, Error> {
        let builder = pex(peers, peers_ipv6, peers_onion);
        self.0.respond(id, ResponseType::Pex(builder)).await?;
        Ok(true)
    }

    async fn update(&mut self, id: usize, msg: &str) -> Result<bool, Error> {
        let builder = update_site(msg.into());
        self.0.respond(id, ResponseType::Update(builder)).await?;
        Ok(true)
    }
}
