pub mod error;
mod handlers;
pub mod request;
pub mod response;

use std::collections::HashMap;

use actix::{Actor, Addr, StreamHandler};
use actix_web::{
    web::{Data, Payload, Query},
    HttpRequest, HttpResponse, Result,
};
use actix_web_actors::ws;
use log::*;
use serde::{Deserialize, Serialize};

use self::request::CommandType;
use crate::{
    controllers::{
        handlers::sites::Lookup, server::ZeroServer, sites::SitesController, users::UserController,
    },
    core::{address::Address, site::Site},
    environment::{Environment, ENV},
};
use error::Error;
use request::{Command, UiServerCommandType::*};
use response::Message;

pub async fn serve_websocket(
    req: HttpRequest,
    query: Query<HashMap<String, String>>,
    data: Data<ZeroServer>,
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
    };

    ws::start(websocket, &req, stream)
}

pub struct ZeruWebsocket {
    site_controller: Addr<SitesController>,
    user_controller: Addr<UserController>,
    site_addr: actix::Addr<Site>,
    address: Address,
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
                    error!("Error handling command: {:?}", err);
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
    // user_settings
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
    debug!("Handling ServerInfo request using dummy response");
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
        // user_settings:
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
                SiteInfo => handlers::sites::handle_site_info(self, ctx, command),
                ChannelJoin => handlers::sites::handle_channel_join(self, ctx, command),
                DbQuery => handlers::sites::handle_db_query(self, ctx, command),
                FileGet => handlers::files::handle_file_get(self, ctx, command),
                FileRules => handlers::files::handle_file_rules(self, ctx, command),
                UserGetSettings => handlers::users::handle_user_get_settings(self, ctx, command),
                UserSetSettings => handlers::users::handle_user_set_settings(self, ctx, command),
                UserGetGlobalSettings => {
                    handlers::users::handle_user_get_global_settings(self, ctx, command)
                }
                _ => {
                    error!("Unhandled Ui command: {:?}", command.cmd);
                    return Err(Error {
                        error: "Unhandled command".to_string(),
                    });
                }
            }
        } else if let CommandType::Admin(_cmd) = &command.cmd {
            error!("Unhandled Admin command: {:?}", command.cmd);
            return Err(Error {
                error: "Unhandled Admin command".to_string(),
            });
        } else {
            return Err(Error {
                error: "Unhandled Plugin command".to_string(),
            });
        };

        let j = serde_json::to_string(&response?)?;
        ctx.text(j);

        Ok(())
    }
}
