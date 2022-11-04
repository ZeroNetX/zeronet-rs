use decentnet_protocol::address::PeerAddr;
use rusqlite::params;

use crate::{
    controllers::sites::SitesController,
    core::{peer::Peer, site::Site},
};

impl SitesController {
    pub fn load_peers(&mut self, site: &Site) {
        let addr = site.address();
        let conn = self.db_manager.get_db("content_db").unwrap();
        let site_id = conn
            .query_row("SELECT * FROM site WHERE address = ?", params![addr], |a| {
                a.get::<_, i32>(0)
            })
            .unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT address, port FROM peer WHERE site_id = ? AND address NOT LIKE '%.onion' AND reputation > 0", //TODO! Don't filter out onion peers
            )
            .unwrap();
        let peers = stmt
            .query_map(params![site_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
            })
            .unwrap();
        for peer in peers {
            let (ip, port) = peer.unwrap();
            let peer = format!("{}:{}", ip, port);
            let peer = Peer::new(PeerAddr::parse(peer).unwrap());
            self.sites.get_mut(&addr).unwrap().add_peer(peer)
        }
    }
}
