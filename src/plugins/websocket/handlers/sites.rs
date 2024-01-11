use actix::AsyncContext;
use actix_web_actors::ws::WebsocketContext;
use futures::executor::block_on;
use log::*;
use serde::Serialize;
use serde_json::Value;

use super::super::{error::Error, request::Command, response::Message, ZeruWebsocket};
use crate::{
    plugins::site_server::handlers::{
        sites::{DBQueryRequest, SiteInfoListRequest, SiteInfoRequest},
        users::UserSiteData,
    },
    plugins::websocket::events::RegisterChannels,
};

pub fn handle_cert_add(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_cert_select(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_info(
    ws: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    let site_info_req = SiteInfoRequest {};
    let result = block_on(ws.site_addr.send(site_info_req))?;
    if result.is_err() {
        return Err(Error {
            error: String::from("Site info not found"),
        });
    }
    if let Some(map) = block_on(ws.user_controller.send(UserSiteData {
        user_addr: String::from("current"),
        site_addr: ws.address.address.clone(),
    }))
    .unwrap()
    {
        let user_site_data = map.values().last().unwrap();
        let mut site_info = result.unwrap();
        site_info.cert_user_id = user_site_data.get_cert_provider();
        if let Some(auth) = user_site_data.get_auth_pair() {
            site_info.auth_address = auth.auth_address;
        }
        if let Some(key) = user_site_data.get_privkey() {
            if !key.is_empty() {
                site_info.privatekey = true;
            }
        }
        command.respond(site_info)
    } else {
        Err(Error {
            error: String::from("Site info not found"),
        })
    }
}

pub fn handle_db_query(
    ws: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    trace!("Handling DBQuery {:?}", command.cmd);
    match &command.params {
        Value::Array(inner_path) => {
            if let Some(query) = inner_path[0].as_str() {
                let stmt_type = query.split_whitespace().next().unwrap();
                if stmt_type.to_uppercase() != "SELECT" {
                    return Err(Error {
                        error: String::from("Only SELECT queries are allowed"),
                    });
                }
                let params = inner_path.get(1).cloned();
                let res = block_on(ws.site_controller.send(DBQueryRequest {
                    address: ws.address.address.clone(),
                    query: query.to_string(),
                    params,
                }))
                .unwrap()
                .unwrap();
                return command.respond(res);
            }
            error!("{:?}", command);
            Err(Error {
                error: String::from("expecting query, failed to parse"),
            })
        }
        _ => {
            error!("{:?}", command);
            Err(Error {
                error: String::from("Invalid params"),
            })
        }
    }
}

pub fn handle_channel_join(
    _: &ZeruWebsocket,
    ctx: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    trace!("Handling ChannelJoin request");
    if let Some(channels) = command.params.as_object() {
        if let Some(channels) = channels.get("channels") {
            if let Some(channels) = channels.as_array() {
                let mut channels_list: Vec<String> = Vec::new();
                for channel in channels {
                    if let Some(channel) = channel.as_str() {
                        channels_list.push(channel.to_string());
                    }
                }
                ctx.address().do_send(RegisterChannels(channels_list));
                return command.respond("ok");
            }
        }
    }
    Err(Error {
        error: String::from("Invalid params"),
    })
}

pub fn handle_site_list(
    ws: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    trace!("Handling SiteList : {:?}", command.params);
    let connecting = if let Value::Object(map) = &command.params {
        let res = if let Some(Value::Bool(value)) = map.get("connecting_sites") {
            *value
        } else {
            false
        };
        res
    } else {
        false
    };
    let sites = block_on(ws.site_controller.send(SiteInfoListRequest { connecting }))
        .unwrap()
        .unwrap();
    command.respond(sites)
}

pub fn handle_channel_join_all_site(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    debug!("Handling ChannelJoinAllsite request using dummy response");
    command.respond(String::from("ok"))
}

pub fn handle_site_sign(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_publish(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_reload(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_update(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_bad_files(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_list_modified_files(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_pause(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_delete(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

#[derive(Serialize)]
pub struct OptionalLimitStats {
    pub limit: String,
    pub used: isize,
    pub free: isize,
}

pub fn _handle_optional_limit_stats(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    // TODO: replace dummy response with actual response
    warn!("Handling OptionalLimitStats with dummy response");
    let limit_stats = OptionalLimitStats {
        limit: String::from("10%"),
        used: 1000000,
        free: 4000000,
    };
    command.respond(limit_stats)
}
