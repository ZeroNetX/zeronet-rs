pub mod builders;

use serde::Serialize;
use serde_json::json;
use zeronet_protocol::{
    message::Response,
    templates::{GetFileResponse, Handshake, PingResponse, StreamFileResponse},
    ZeroConnection,
};

use crate::core::error::Error;

use self::builders::*;

pub struct Protocol<'a>(&'a mut ZeroConnection);

impl<'a> Protocol<'a> {
    pub fn new(connection: &'a mut ZeroConnection) -> Self {
        Protocol(connection)
    }

    async fn jsoned_req<T: Serialize>(&mut self, cmd: &str, req: T) -> Result<Response, Error> {
        let res = self.0.request(cmd, json!(req)).await?;
        Ok(res)
    }

    async fn invoke_with_builder<T: Serialize>(
        &mut self,
        builder: (&str, T),
    ) -> Result<Response, Error> {
        let res = self.jsoned_req(builder.0, builder.1).await?;
        Ok(res)
    }
}

///https://docs.zeronet.dev/1DeveLopDZL1cHfKi8UXHh2UBEhzH6HhMp/help_zeronet/network_protocol/
impl<'a> Protocol<'a> {
    ///#handshake
    pub async fn handshake(&mut self) -> Result<Handshake, Error> {
        let builder = build_handshake();
        let res = self.invoke_with_builder(builder).await?;
        let body: Handshake = res.body()?;
        Ok(body)
    }

    ///#ping
    pub async fn ping(&mut self) -> Result<bool, Error> {
        let res = self.0.request("ping", json!({})).await?;
        let res: PingResponse = res.body()?;
        Ok(res.body == "Pong!")
    }

    ///#getFile
    pub async fn get_file(
        &mut self,
        site: String,
        inner_path: String,
    ) -> Result<GetFileResponse, Error> {
        let builder = build_get_file(site, inner_path, 0, 0);
        let res = self.invoke_with_builder(builder).await?;
        let body: GetFileResponse = res.body()?;
        Ok(body)
    }

    ///#streamFile
    pub async fn stream_file(
        &mut self,
        site: String,
        inner_path: String,
    ) -> Result<StreamFileResponse, Error> {
        let builder = build_stream_file(site, inner_path, 0);
        let res = self.invoke_with_builder(builder).await?;
        let body: StreamFileResponse = res.body()?;
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use zeronet_protocol::PeerAddr;

    use crate::core::peer::Peer;

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
