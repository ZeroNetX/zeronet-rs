use std::collections::HashMap;

use actix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::{error::Error, site::models::SiteInfo};

use super::{handlers::tracker::AnnouncerStats, ServerInfo, ZeruWebsocket};

pub struct WebsocketController {
    pub listeners: Vec<Addr<ZeruWebsocket>>,
}

impl Actor for WebsocketController {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct RegisterWSClient {
    pub addr: Addr<ZeruWebsocket>,
}

impl Handler<RegisterWSClient> for WebsocketController {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: RegisterWSClient, _ctx: &mut Context<Self>) -> Self::Result {
        self.listeners.push(msg.addr);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Message)]
#[rtype(result = "()")]
#[serde(rename_all = "camelCase")]
pub enum ServerEvent {
    Event { cmd: String, params: EventType },
}

#[allow(clippy::enum_variant_names)]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum EventType {
    AnnouncerInfo {
        address: String,
        stats: HashMap<String, AnnouncerStats>,
    },
    ServerInfo(ServerInfo),
    SiteInfo(SiteInfo),
}

impl Handler<ServerEvent> for ZeruWebsocket {
    type Result = ();

    fn handle(&mut self, msg: ServerEvent, ctx: &mut Self::Context) {
        ctx.text(serde_json::to_string(&msg).unwrap());
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct RegisterChannels(pub Vec<String>);

impl Handler<RegisterChannels> for ZeruWebsocket {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: RegisterChannels, _ctx: &mut Self::Context) -> Self::Result {
        let channels = msg.0;
        println!("RegisteringChannels: {:?}", channels);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{super::handlers::tracker::AnnouncerStats, EventType, ServerEvent};

    fn def_ann_info() -> ServerEvent {
        let mut map = HashMap::new();
        map.insert(
            String::from("zero://boot3rdez4rzn36x.onion:15441"),
            AnnouncerStats {
                status: "announced".into(),
                num_request: 20,
                num_success: 15,
                num_error: 5,
                ..Default::default()
            },
        );
        ServerEvent::Event {
            cmd: "setAnnouncerInfo".into(),
            params: EventType::AnnouncerInfo {
                address: "1HELLoE3sFD9569CLCbHEAVqvqV7U2Ri9d".into(),
                stats: map,
            },
        }
    }

    #[test]
    fn test_announcer_info() {
        let info = def_ann_info();
        let json = serde_json::to_string_pretty(&info).unwrap();
        println!("{:#}", json);
    }
}
