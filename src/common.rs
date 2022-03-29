use rusqlite::{params, types::Value, Connection};
use serde_json::Map;
use zeronet_protocol::PeerAddr;

use crate::{
    core::{discovery::Discovery, error::Error, io::*, peer::*, site::*, user::*},
    environment::ENV,
    io::db::DbManager,
    protocol::{api::Request, Protocol},
    utils::to_json_value,
};

pub async fn site_create(user: &mut User, use_master_seed: bool) -> Result<(), Error> {
    let site_data;
    if use_master_seed {
        site_data = user.get_new_site_data();
        println!("\n");
        println!(
            "Site Private Key : {:?} <<< Store this to Safe Place",
            site_data.get_privkey().unwrap()
        );
        println!("Site Address     : {:?}", site_data.address);
        println!("\n");
    } else {
        unimplemented!();
    }
    let mut site = Site::new(&site_data.address, (*ENV).data_path.clone())?;
    site.create(site_data.index.unwrap(), &site_data.get_privkey().unwrap())
        .await?;
    Ok(())
}

pub async fn find_peers(site: &mut Site) -> Result<(), Error> {
    let peers = site.discover().await?;
    for peer in &peers {
        println!("{:?}", peer);
    }
    let mut connections = vec![];
    for mut peer in peers {
        let res = peer.connect();
        if let Err(e) = &res {
            println!("Error : {:?}", e);
            let peer = peer.clone().address().to_string();
            println!("{}", peer);
        } else {
            println!("Connection Successful");
            connections.push(peer);
        }
    }
    let connectable_peers = connections.iter().map(|peer| peer.address().to_string());
    save_peers(connectable_peers).await;
    Ok(())
}

pub async fn rebuild_db(site: &mut Site, db_manager: &mut DbManager) -> Result<(), Error> {
    let has_schema = db_manager.has_schema(&site.address());
    let address = site.address();
    if has_schema.0 {
        let _schema = db_manager.load_schema(&address).unwrap();
        db_manager.connect_db(&address);
        db_manager.create_tables(&address);
        db_manager.load_data(&address).await;
    }
    Ok(())
}

pub async fn db_query(conn: &mut Connection, query: &str) -> Result<(), Error> {
    let mut stmt = conn.prepare(query).unwrap();
    let count = stmt.column_count();
    let names = {
        stmt.column_names()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
    };
    let res = stmt
        // .query(params![]).unwrap();
        .query_map(params![], |row| {
            let mut data_map = Map::new();
            let mut i = 0;
            loop {
                while let Ok(value) = row.get::<_, Value>(i) {
                    let name = names.get(i).unwrap().to_string();
                    i += 1;
                    let value = to_json_value(&value);
                    data_map.insert(name, value);
                }
                if i == count {
                    break;
                }
            }
            Ok(data_map)
        })
        .unwrap();
    for row in res {
        println!("{:#?}", row.unwrap());
    }
    Ok(())
}

pub async fn download_site(site: &mut Site) -> Result<(), Error> {
    let exists = site.content_path().is_file();
    if !exists {
        add_peers_to_site(site).await?;
        println!("Downloading Site");
        site.init_download().await?;
    }
    Ok(())
}

pub async fn site_sign(site: &mut Site, private_key: String) -> Result<(), Error> {
    site.load_content().await?;
    let changes = site.check_site_integrity().await?;
    if changes.is_empty() {
        println!("No changes to sign");
    } else {
        let content = {
            let mut content = site.content().unwrap();
            let mut files = content.files.clone();
            for (inner_path, file) in changes {
                if files.insert(inner_path, file).is_none() {
                    unreachable!();
                };
            }
            content.files = files;
            content
        };
        site.modify_content(content);
        let res = site.verify_content(false).await?;
        if res {
            site.sign_content(&private_key).await?;
            site.save_content(None).await?;
        } else {
            println!("Site Not Signed");
        }
    }
    Ok(())
}

pub async fn site_need_file(site: &mut Site, inner_path: String) -> Result<(), Error> {
    add_peers_to_site(site).await?;
    let download = if inner_path.ends_with("content.json") {
        true
    } else {
        site.load_content().await?;
        let files = site.content().unwrap().files;
        files.keys().any(|path| path == &inner_path)
    };
    if !download {
        println!("Inner Path Not Exists in content.json");
    } else {
        let result = site.need_file(inner_path.clone(), None, None).await;
        if let Err(e) = &result {
            println!("Error : {:?}", e);
        } else {
            println!("File Downloaded : {:?}", inner_path);
        }
    }
    Ok(())
}

pub async fn peer_ping(addr: &str) -> Result<(), Error> {
    let mut peer = Peer::new(PeerAddr::parse(addr).unwrap());
    let res = peer.connect();
    if res.is_ok() {
        let res = Protocol::new(peer.connection_mut().unwrap()).ping().await?;
        println!("Ping Result : {:?} from Peer : {:?}", res, addr);
        return Ok(());
    }
    Err(Error::Err("Peer Not Found".into()))
}

pub async fn peer_exchange(site: &mut Site) -> Result<(), Error> {
    add_peers_to_site(site).await?;
    site.load_content().await?;
    let mut peers_cons = vec![];
    let peers = site.fetch_peers().await?;
    println!("Found Peers : {:?}", peers);
    for peer in peers {
        let mut peer = Peer::new(PeerAddr::parse(peer).unwrap());
        let res = peer.connect();
        if res.is_ok() {
            peers_cons.push(peer);
        }
    }
    for mut con in peers_cons {
        let res = Protocol::new(con.connection_mut().unwrap()).ping().await?;
        println!("Ping Result : {:?} from Peer : {:?}", res, con.address());
    }
    Ok(())
}

pub async fn fetch_changes(site: &mut Site) -> Result<(), Error> {
    add_peers_to_site(site).await?;
    site.load_content().await?;
    let modified = site.content().unwrap().modified;
    println!("{:?}", modified);
    let changes = site.fetch_changes(1421043090).await?;
    println!("{:#?}", changes);
    Ok(())
}

pub async fn check_site_integrity(site: &mut Site) -> Result<(), Error> {
    site.load_content().await?;
    let res = site.verify_content(false).await?;
    if res {
        println!("Site {} verified", site.address());
    } else {
        println!("Site {} verification failed", site.address());
    }
    Ok(())
}

pub async fn add_peers_to_site(site: &mut Site) -> Result<(), Error> {
    let peers = load_peers().await;
    let peers = peers
        .iter()
        .map(|peer| Peer::new(PeerAddr::parse(peer.to_string()).unwrap()))
        .collect::<Vec<_>>();
    for mut peer in peers {
        peer.connect()?;
        let res = Protocol::new(peer.connection_mut().unwrap())
            .handshake()
            .await;
        if let Err(e) = res {
            println!("Error : {:?}", e);
            let peer = peer.clone().address().to_string();
            println!("{}", peer);
        } else {
            let response = res?;
            // println!("Ping Result : {:?}", response);
            site.peers.insert(response.peer_id.clone(), peer);
        }
    }
    Ok(())
}

pub async fn save_peers(peers: impl Iterator<Item = String>) {
    let mut file = tokio::fs::File::create("data/peers.txt").await.unwrap();
    for peer in peers {
        tokio::io::AsyncWriteExt::write_all(&mut file, peer.as_bytes())
            .await
            .unwrap();
    }
}

pub async fn load_peers() -> Vec<String> {
    let mut file = tokio::fs::File::open("data/peers.txt").await.unwrap();
    let mut buf = vec![];
    tokio::io::AsyncReadExt::read_to_end(&mut file, &mut buf)
        .await
        .unwrap();
    let mut peers = vec![];
    for peer in buf.split(|b| (b == b"\n".first().unwrap()) || (b == b"\r".first().unwrap())) {
        peers.push(String::from_utf8(peer.to_vec()).unwrap());
    }
    peers.drain_filter(|peer| !peer.is_empty()).collect()
}
