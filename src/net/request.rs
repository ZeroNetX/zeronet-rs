use std::collections::HashMap;

use serde_bytes::ByteBuf;
use serde_json::Value;

use decentnet_protocol::{
    builders::request::*, interface::RequestImpl, message::RequestType, templates::*,
};

use crate::{
    core::error::Error,
    net::{handshake, Protocol},
};

///https://docs.zeronet.dev/1DeveLopDZL1cHfKi8UXHh2UBEhzH6HhMp/help_zeronet/network_protocol/
#[async_trait::async_trait]
impl<'a> RequestImpl for Protocol<'a> {
    type Error = Error;
    ///#handshake
    async fn handshake(&mut self) -> Result<Handshake, Self::Error> {
        let builder = handshake();
        let res = self
            .0
            .request(builder.0, RequestType::Handshake(builder.1))
            .await?;

        Ok(res.body()?)
    }

    ///#ping
    async fn ping(&mut self) -> Result<bool, Self::Error> {
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
        read_bytes: Option<usize>,
    ) -> Result<GetFileResponse, Self::Error> {
        let builder = get_file(site, inner_path, file_size, location, read_bytes);
        let res = self
            .0
            .request(builder.0, RequestType::GetFile(builder.1))
            .await?;

        Ok(res.body()?)
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

        Ok(res.body()?)
    }

    ///#listModified
    async fn list_modified(
        &mut self,
        site: String,
        since: usize,
    ) -> Result<ListModifiedResponse, Self::Error> {
        let builder = list_modified(site, since);
        let res = self
            .0
            .request(builder.0, RequestType::ListModified(builder.1))
            .await?;

        Ok(res.body()?)
    }

    ///#pex
    async fn pex(&mut self, site: String) -> Result<PexResponse, Error> {
        let builder = pex(site, 10);
        let res = self
            .0
            .request(builder.0, RequestType::Pex(builder.1))
            .await?;

        Ok(res.body()?)
    }

    async fn update(
        &mut self,
        site: String,
        inner_path: String,
        body: ByteBuf,
        diffs: HashMap<String, Vec<Value>>,
        modified: usize,
    ) -> Result<UpdateSiteResponse, Error> {
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
    use decentnet_protocol::{address::PeerAddr, interface::RequestImpl};

    use crate::{core::peer::Peer, net::Protocol};

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
