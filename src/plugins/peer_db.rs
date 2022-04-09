use rusqlite::params;

use crate::{controllers::connections::ConnectionController, core::site::Site};

impl ConnectionController {
    pub fn load_peers(&mut self, site: &Site) {
        let addr = site.address();
        let conn = self.db_manager.get_db("content_db").unwrap();
        let site_id = conn
            .query_row("SELECT * FROM site WHERE address = ?", params![addr], |a| {
                a.get::<_, i32>(0)
            })
            .unwrap();
        let peers = conn.query_row(
            "SELECT * FROM peer WHERE site_id = :site_id",
            params![site_id],
            |r| r.get::<_, String>(1),
        );
    }
}
