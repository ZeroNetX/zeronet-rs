use std::collections::HashMap;

use actix::Handler;
use actix_web_actors::ws::WebsocketContext;
use futures::executor::block_on;
use log::*;
use serde::{Deserialize, Serialize};

use super::super::{
    error::Error, events::EventType::AnnouncerInfo, request::Command, response::Message,
    ZeruWebsocket,
};
use crate::{
    controllers::sites::SitesController,
    core::{address::Address, site::Site},
};

pub fn handle_announcer_info(
    ws: &ZeruWebsocket,
    _ctx: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    warn!("Handling AnnouncerStats request");
    let address = ws.address.address.clone();
    command.respond(AnnouncerInfo {
        address,
        stats: HashMap::new(),
    })
}

pub fn handle_announcer_stats(
    _ws: &ZeruWebsocket,
    _ctx: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    warn!("Handling AnnouncerStats request with dummy response");
    // TODO: actually return announcer stats
    let mut stats: HashMap<String, _> = HashMap::new();
    stats.insert(
        String::from("zero://boot3rdez4rzn36x.onion:15441"),
        AnnouncerStats {
            status: String::from("announced"),
            num_request: 0,
            num_success: 0,
            num_error: 0,
            time_request: 0.0,
            time_last_error: 0.0,
            time_status: 0.0,
            last_error: String::from("Not implemented yet"),
        },
    );
    command.respond(stats)
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AnnouncerStats {
    pub status: String,
    pub num_request: usize,
    pub num_success: usize,
    pub num_error: usize,
    pub time_request: f64,
    pub time_last_error: f64,
    pub time_status: f64,
    pub last_error: String,
}

#[derive(actix::Message)]
#[rtype(result = "()")]
pub struct SiteAnnounce {
    pub address: Address,
}

impl Handler<SiteAnnounce> for Site {
    type Result = ();

    fn handle(&mut self, _msg: SiteAnnounce, _ctx: &mut Self::Context) -> Self::Result {
        warn!("Handling SiteAnnounce request with dummy response");
        let peers = block_on(self.find_peers()).unwrap();
        self.add_peers(peers);
    }
}

impl Handler<SiteAnnounce> for SitesController {
    type Result = ();

    fn handle(&mut self, msg: SiteAnnounce, _ctx: &mut Self::Context) -> Self::Result {
        let (_, site) = self.get(msg.address.clone()).unwrap();
        site.do_send(msg)
    }
}
