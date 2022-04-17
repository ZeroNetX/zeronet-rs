use std::collections::HashMap;

use crate::{
    core::error::Error,
    protocol::{
        api::Request,
        builders::{request::*, *},
        Protocol,
    },
};

use serde_bytes::ByteBuf;
use serde_json::Value;
use zeronet_protocol::{message::RequestType, templates::*};

///https://docs.zeronet.dev/1DeveLopDZL1cHfKi8UXHh2UBEhzH6HhMp/help_zeronet/network_protocol/
#[async_trait::async_trait]
impl<'a> Request for Protocol<'a> {
    ///#handshake
    async fn handshake(&mut self) -> Result<Handshake, Error> {
        let builder = handshake();
        let res = self
            .0
            .request(builder.0, RequestType::Handshake(builder.1))
            .await?;
        let body: Handshake = res.body()?;
        Ok(body)
    }

    ///#ping
    async fn ping(&mut self) -> Result<bool, Error> {
        let res = self.0.request("ping", RequestType::Ping(Ping())).await?;
        let res: PingResponse = res.body()?;
        Ok(res.body == "Pong!")
    }

    ///#getFile
    async fn get_file(
        &mut self,
        site: String,
        inner_path: String,
        file_size: usize,
        location: usize,
    ) -> Result<GetFileResponse, Error> {
        let builder = get_file(site, inner_path, file_size, location, None); //TODO! Pass read_bytes to builder
        let res = self
            .0
            .request(builder.0, RequestType::GetFile(builder.1))
            .await?;
        let body: GetFileResponse = res.body()?;
        Ok(body)
    }

    ///#streamFile
    async fn stream_file(
        &mut self,
        site: String,
        inner_path: String,
    ) -> Result<StreamFileResponse, Error> {
        //TODO!: Remove default values from builder, size
        let builder = stream_file(site, inner_path, 0, 0, 0);
        let res = self
            .0
            .request(builder.0, RequestType::StreamFile(builder.1))
            .await?;
        let body: StreamFileResponse = res.body()?;
        Ok(body)
    }

    ///#listModified
    async fn list_modified(
        &mut self,
        site: String,
        since: usize,
    ) -> Result<ListModifiedResponse, Error> {
        let builder = list_modified(site, since);
        let res = self
            .0
            .request(builder.0, RequestType::ListModified(builder.1))
            .await?;
        let body: ListModifiedResponse = res.body()?;
        Ok(body)
    }

    ///#pex
    async fn pex(&mut self, site: String) -> Result<PexResponse, Error> {
        let builder = pex(site, 10);
        let res = self
            .0
            .request(builder.0, RequestType::Pex(builder.1))
            .await?;
        let body: PexResponse = res.body()?;
        Ok(body)
    }

    async fn update(
        &mut self,
        site: String,
        inner_path: String,
        body: ByteBuf,
        diffs: HashMap<String, Vec<Value>>,
        modified: usize,
    ) -> Result<UpdateResponse, Error> {
        let builder = update_site(site, inner_path, body, diffs, modified);
        let res = self
            .0
            .request(builder.0, RequestType::Update(builder.1))
            .await?;
        match res.body() {
            Ok(body) => Ok(body),
            Err(e) => Err(Error::Err(format!("{:?}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use zeronet_protocol::PeerAddr;

    use crate::{core::peer::Peer, protocol::api::Request};

    use super::Protocol;

    #[tokio::test]
    async fn test_protocol() {
        let mut peer = Peer::new(PeerAddr::parse("72.189.0.3:21619".to_string()).unwrap());
        if let Err(err) = peer.connect() {
            println!("Error Connecting : {:?}", err);
        } else {
            println!("Connected");
            let mut protocol = Protocol::new(peer.connection_mut().unwrap());
            let res = protocol.handshake().await;
            if let Err(err) = res {
                println!("Error : {:?}", err);
                assert!(false);
            } else {
                println!("Handshake : {:?}", res.unwrap());
            }
        }
    }
}
