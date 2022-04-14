use serde::Serialize;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    select,
};
use zeronet_protocol::requestable::Requestable;

use crate::{
    core::error::Error,
    protocol::{
        message::{Request, Response},
        msgpack::pack,
    },
};

// T: Requestable,
struct Connection<R, W>
where
    R: AsyncRead + Send,
    W: AsyncWrite + Send,
{
    reader: R,
    writer: W,
    requests: usize,
    is_closed: bool,
}

impl<R, W> Connection<R, W>
where
    R: AsyncRead + Send + Unpin,
    W: AsyncWrite + Send + Unpin,
{
    pub fn new(reader: R, writer: W) -> Self {
        Connection {
            reader,
            writer,
            requests: 0,
            is_closed: false,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    pub async fn recv<T: Requestable>(&mut self) -> Result<(), Error> {
        let mut buf: Vec<u8> = vec![];
        let bytes = self.reader.read_to_end(&mut buf).await;
        if bytes.is_err() {
            return Err(Error::Err("read error".into()));
        } else {
            // let res = unpack::<Message>(&buf);
            // if res.is_err() {
            //     return Err(Error::Err("unpack error".into()));
            // }
            // let req = res.unwrap();
            // self.requests += 1;
        }
        Ok(())
    }

    pub async fn request<Body: Serialize>(&mut self, cmd: &str, body: Body) -> Result<(), Error> {
        self.requests += 1;
        let req = Request {
            cmd: cmd.to_string(),
            req_id: self.requests,
            params: body,
        };
        let req = pack(req).unwrap();
        self.send(&req).await
    }

    pub async fn respond<Body: Serialize>(&mut self, cmd: &str, body: Body) -> Result<(), Error> {
        self.requests += 1;
        let req = Response {
            cmd: cmd.to_string(),
            to: 0,
            body,
        };
        let req = pack(req).unwrap();
        self.send(&req).await
    }

    pub async fn send(&mut self, request: &[u8]) -> Result<(), Error> {
        let data = Box::pin(&mut self.writer).write_all(&request).await;
        Ok(())
    }
}

pub async fn serve() {
    let mut stream = tokio::net::TcpStream::connect("").await.unwrap();
    let (reader, writer) = stream.split();
    let mut conn = Connection::new(reader, writer);
    select! {
        _ = conn.reader.readable() => {
            // conn.recv().await.unwrap();
        },
        // _ = conn.write() => {},
    }
}
