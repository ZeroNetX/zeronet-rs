use std::net::{TcpListener, TcpStream};

use zeronet_protocol::ZeroConnection;

use crate::core::error::Error;

pub async fn start(ip: &str, port: u16) -> Result<(), Error> {
    let listener = TcpListener::bind(format!("{}:{}", ip, port))?;
    let mut incoming = listener.incoming();

    while let Some(stream) = incoming.next() {
        if let Ok(stream) = stream {
            handle_connection(stream).await?;
        }
    }

    Ok(())
}

async fn handle_connection(stream: TcpStream) -> Result<(), Error> {
    let mut connection =
        ZeroConnection::new(Box::new(stream.try_clone().unwrap()), Box::new(stream))?;
    let request = connection.recv().await?;
    println!("{:?}", request);
    Ok(())
}
