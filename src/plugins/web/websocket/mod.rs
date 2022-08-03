pub mod error;
pub mod events;
mod handlers;
pub mod request;
pub mod response;

use futures::executor::block_on;
pub use handlers::tracker::SiteAnnounce;

use std::collections::HashMap;

use actix::{Actor, Addr, StreamHandler};
use actix_web::{
    web::{Data, Payload, Query},
    HttpRequest, HttpResponse, Result,
};
use actix_web_actors::ws::{self, WsResponseBuilder};
use log::*;
use serde::{Deserialize, Serialize};

use self::{
    events::WebsocketController,
    handlers::{files::*, sites::*, tracker::*, users::*},
    request::CommandType,
};
use crate::{
    controllers::{
        handlers::sites::{Lookup, SiteInfoRequest},
        server::ZeroServer,
        sites::SitesController,
        users::UserController,
    },
    core::{address::Address, site::Site},
    environment::{Environment, ENV},
    plugins::web::websocket::events::RegisterWSClient,
};
use error::Error;
use request::{AdminCommandType::*, Command, UiServerCommandType::*};
use response::Message;

pub async fn serve_websocket(
    req: HttpRequest,
    query: Query<HashMap<String, String>>,
    data: Data<ZeroServer>,
    controller_data: Data<Addr<WebsocketController>>,
    stream: Payload,
) -> Result<HttpResponse, actix_web::Error> {
    info!("Serving websocket");
    let wrapper_key = query.get("wrapper_key").unwrap();
    let future = data
        .site_controller
        .send(Lookup::Key(String::from(wrapper_key)));
    let (address, addr) = match future.await {
        Ok(Ok(resp)) => resp,
        _ => {
            warn!("Websocket established, but wrapper key invalid");
            return Ok(HttpResponse::Ok().body("Invalid wrapper key"));
        }
    };

    info!("Websocket established for {}", address.get_address_short());
    let websocket = ZeruWebsocket {
        site_controller: data.site_controller.clone(),
        user_controller: data.user_controller.clone(),
        site_addr: addr,
        address,
        channels: vec![],
    };
    let (addr, res) = WsResponseBuilder::new(websocket, &req, stream)
        .start_with_addr()
        .unwrap();
    controller_data.do_send(RegisterWSClient { addr });
    Ok(res)
}

pub struct ZeruWebsocket {
    site_controller: Addr<SitesController>,
    user_controller: Addr<UserController>,
    site_addr: actix::Addr<Site>,
    address: Address,
    channels: Vec<String>,
}

impl Actor for ZeruWebsocket {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ZeruWebsocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if msg.is_err() {
            error!("Protocol error on websocket message");
            return;
        }
        match msg.unwrap() {
            ws::Message::Ping(msg) => ctx.pong(&msg),
            ws::Message::Text(text) => {
                let command: Command = match serde_json::from_str(&text) {
                    Ok(c) => c,
                    Err(e) => {
                        error!(
                            "Could not deserialize incoming message: {:?} ({:?})",
                            text, e
                        );
                        return;
                    }
                };
                if let Err(err) = self.handle_command(ctx, &command) {
                    debug!("Error handling command: {:?}", err);
                    let _ = handle_error(ctx, command, format!("{:?}", err));
                }
            }
            ws::Message::Binary(_) => {
                warn!("Unhandled binary data received over websocket");
            }
            _ => (),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct WrapperCommand {
    cmd: WrapperCommandType,
    to: isize,
    result: WrapperResponse,
}

#[derive(Serialize, Deserialize)]
pub enum WrapperResponse {
    Empty,
    ServerInfo(Box<ServerInfo>),
    Text(String),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WrapperCommandType {
    Response,
    Error,
    WrapperReady,
    Ping,
    WrapperOpenedWebsocket,
    WrapperClosedWebsocket,
}

#[derive(Serialize, Deserialize)]
pub struct ServerPortOpened {
    ipv4: bool,
    ipv6: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ServerInfo {
    ip_external: bool,
    port_opened: ServerPortOpened,
    platform: String,
    fileserver_ip: String,
    fileserver_port: u16,
    tor_enabled: bool,
    tor_status: String,
    tor_has_meek_bridges: bool,
    ui_ip: String,
    ui_port: u16,
    version: String,
    rev: usize,
    timecorrection: f64,
    language: String,
    debug: bool,
    offline: bool,
    plugins: Vec<String>,
    plugins_rev: HashMap<String, usize>,
    multiuser: bool,
    master_address: String,
    user_settings: HashMap<String, serde_json::Value>,
}

fn handle_ping(
    _: &mut ws::WebsocketContext<ZeruWebsocket>,
    req: &Command,
) -> Result<Message, Error> {
    trace!("Handling ping");
    let pong = String::from("pong");
    req.respond(pong)
}

fn handle_server_info(
    ws: &mut ZeruWebsocket,
    _: &mut ws::WebsocketContext<ZeruWebsocket>,
    req: &Command,
) -> Result<Message, Error> {
    trace!("Handling ServerInfo request");
    //TODO!: Replace Defaults with actual values
    let user = handlers::users::get_current_user(ws)?;
    let env: Environment = (*ENV).clone();
    let server_info = ServerInfo {
        ip_external: false,
        port_opened: ServerPortOpened {
            ipv4: true,
            ipv6: false,
        },
        platform: env.dist,
        fileserver_ip: env.fileserver_ip,
        fileserver_port: env.fileserver_port,
        tor_enabled: false,
        tor_status: String::from("Disabled"),
        tor_has_meek_bridges: false,
        ui_ip: env.ui_ip,
        ui_port: env.ui_port,
        version: env.version,
        rev: env.rev,
        timecorrection: 0f64,
        language: env.lang,
        debug: true,
        offline: false,
        plugins: vec![],
        plugins_rev: HashMap::new(),
        multiuser: false,
        master_address: user.master_address,
        user_settings: user.settings,
    };
    req.respond(server_info)
}

fn handle_error(
    ctx: &mut ws::WebsocketContext<ZeruWebsocket>,
    command: Command,
    text: String,
) -> Result<(), actix_web::Error> {
    let error = WrapperCommand {
        cmd: WrapperCommandType::Error,
        to: command.id,
        result: WrapperResponse::Text(text),
    };
    let j = serde_json::to_string(&error)?;
    ctx.text(j);
    Ok(())
}

impl ZeruWebsocket {
    fn is_admin_site(&mut self) -> Result<bool, Error> {
        let site = block_on(self.site_addr.send(SiteInfoRequest {}))??;
        let res = site
            .settings
            .settings
            .permissions
            .contains(&("ADMIN".to_string()));
        Ok(res)
    }

    fn handle_command(
        &mut self,
        ctx: &mut ws::WebsocketContext<ZeruWebsocket>,
        command: &Command,
    ) -> Result<(), Error> {
        // info!("Handling command: {:?}", command.cmd);
        let response = if let CommandType::UiServer(cmd) = &command.cmd {
            match cmd {
                Ping => handle_ping(ctx, command),
                ServerInfo => handle_server_info(self, ctx, command),
                SiteInfo => handle_site_info(self, ctx, command),
                ChannelJoin => handle_channel_join(self, ctx, command),
                DbQuery => handle_db_query(self, ctx, command),
                FileGet => handle_file_get(self, ctx, command),
                FileRules => handle_file_rules(self, ctx, command),
                UserGetSettings => handle_user_get_settings(self, ctx, command),
                UserSetSettings => handle_user_set_settings(self, ctx, command),
                UserGetGlobalSettings => handle_user_get_global_settings(self, ctx, command),
                AnnouncerInfo => handle_announcer_info(self, ctx, command),
                _ => {
                    debug!("Unhandled Ui command: {:?}", command.cmd);
                    return Err(Error {
                        error: "Unhandled command".to_string(),
                    });
                }
            }
        } else if let CommandType::Admin(cmd) = &command.cmd {
            if !self.is_admin_site()? {
                return Err(Error {
                    error: format!("You don't have permission to run {:?}", cmd),
                });
            }
            match cmd {
                AnnouncerStats => handle_announcer_stats(self, ctx, command),
                ChannelJoinAllsite => handle_channel_join_all_site(self, ctx, command),
                SiteList => handle_site_list(self, ctx, command),
                UserSetGlobalSettings => handle_user_set_global_settings(self, ctx, command),
                _ => {
                    debug!("Unhandled Admin command: {:?}", command.cmd);
                    return Err(Error {
                        error: "Unhandled Admin command".to_string(),
                    });
                }
            }
        } else {
            debug!("Unhandled Plugin command: {:?}", command.cmd);
            command.respond("ok")
            // return Err(Error {
            //     error: "Unhandled Plugin command".to_string(),
            // });
        };

        let j = serde_json::to_string(&response?)?;
        ctx.text(j);

        Ok(())
    }
}
