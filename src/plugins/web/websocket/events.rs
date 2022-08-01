use std::time::Duration;

use actix::prelude::*;

use super::*;
use crate::core::error::Error;

pub struct WebsocketController {
    pub listeners: Vec<Addr<ZeruWebsocket>>,
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

#[derive(Message)]
#[rtype(result = "()")]
struct ServerEvent {
    event: String,
}

impl Actor for WebsocketController {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_secs(5), |act, _| {
            for listener in &act.listeners {
                listener.do_send(ServerEvent {
                    event: "{\"cmd\":\"setAnnouncerInfo\", \"params\":{\"address\":null,\"stats\":{\"custom_tracker\":{\"status\":\"announced\"}}}}".to_string(),
                });
            }
        });
    }
}

impl Handler<ServerEvent> for ZeruWebsocket {
    type Result = ();

    fn handle(&mut self, msg: ServerEvent, ctx: &mut Self::Context) {
        ctx.text(msg.event);
    }
}
