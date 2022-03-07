// http tracker functionality
#![allow(dead_code)]

use super::IpPort;

use super::bencode::{decode::parse, Item};

use std::{
    io::{Error, ErrorKind},
    net::SocketAddr,
    str::from_utf8,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

// takes in info_hash and tracker addr, announces and gets peer IpPorts
pub async fn http_announce(
    addr: SocketAddr,
    info_hash: [u8; 20],
    port: u16,
    anounce_route: Option<&str>,
) -> Result<Vec<IpPort>, Error> {
    let mut get: Vec<u8> = vec![];
    // prefix
    let route = if let Some(route) = anounce_route {
        format!("/{}", route)
    } else {
        format!("/announce")
    };
    let mut base: String = format!("GET /{route}?info_hash=");
    // appending each hex as a string
    for byte in &info_hash {
        base.push_str(&format!("%{:02x}", byte));
    }
    // append suffix of get request
    base.push_str("&peer_id=-qB4250-rj6kZQu4P_Mh&port=");
    base.push_str(&format!("{}", port));
    base.push_str("&uploaded=0&downloaded=0&left=1456927919\
    &corrupt=0&key=8B26698B&event=started&numwant=200&compact=1&no_peer_id=1&supportcrypto=1&redundant=0\
    HTTP/1.1\r\n\r\n");
    // convert base to Vec<u8> and append to get vector
    get.extend_from_slice(&base.as_bytes());
    // connect to the tracker
    let mut stream = TcpStream::connect(addr).await?;
    // send the get request to the tracker
    stream.write_all(&get).await?;
    // read it's reply
    let mut buf: Vec<u8> = vec![0; 10000];
    let len;
    match stream.read(&mut buf).await {
        Ok(l) => len = l,
        Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
    }
    buf.truncate(len);
    // remove http header
    let mut count = 0;
    for i in buf.windows(4) {
        count += 1;
        if i.iter().zip(b"\r\n\r\n").all(|(a, b)| *a == *b) {
            break;
        };
    }
    buf.drain(0..count + 4);
    if buf[0] != b'd' {
        buf.insert(0, b'd');
        buf.push(b'e');
    }
    // parse out ip port and return
    let tree: Vec<Item> = parse(&mut buf);
    match &tree[0] {
        Item::Dict(d) => {
            if let Some(e) = d.get("failure reason".as_bytes()) {
                match e {
                    Item::String(s) => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            from_utf8(s).unwrap().to_string(),
                        ));
                    }
                    _ => unreachable!(),
                }
            }
        }
        _ => unreachable!(),
    }
    let peers = tree[0]
        .get_dict()
        .get("peers".as_bytes())
        .unwrap()
        .get_str();

    Ok(IpPort::from_bytes(&peers))
}
