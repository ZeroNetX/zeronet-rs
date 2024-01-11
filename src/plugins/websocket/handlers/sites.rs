use actix::AsyncContext;
use actix_web_actors::ws::WebsocketContext;
use futures::executor::block_on;
use log::*;
use serde::Serialize;
use serde_json::{Value, json};

use super::super::{error::Error, request::Command, response::Message, ZeruWebsocket};
use crate::{
    environment::SITE_PERMISSIONS_DETAILS,
    plugins::site_server::handlers::{
        sites::{DBQueryRequest, SiteInfoListRequest, SiteInfoRequest},
        users::UserSiteData,
    },
    plugins::{
        site_server::handlers::{
            sites::{
                SiteBadFilesRequest, SiteDeleteRequest, SitePauseRequest, SitePermissionAddRequest,
                SitePermissionRemoveRequest, SiteResumeRequest, SiteSetSettingsValueRequest,
            },
            users::UserSiteDataDeleteRequest,
        },
        websocket::events::RegisterChannels,
    },
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

pub fn handle_site_info(ws: &ZeruWebsocket, command: &Command) -> Result<Message, Error> {
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
        if let Value::Object(params) = &command.params {
            if let Some(Value::String(path)) = params.get("file_status") {
                site_info.event = Some(json!(["file_done", path])); //TODO!: get file status
            }
        } 
        command.respond(site_info)
    } else {
        Err(Error {
            error: String::from("Site info not found"),
        })
    }
}

pub fn handle_db_query(ws: &ZeruWebsocket, command: &Command) -> Result<Message, Error> {
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
    ctx: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    trace!("Handling ChannelJoin request");
    let channel = &command.params;
    let mut list = vec![];
    match channel {
        Value::Object(map) => {
            if let Some(Value::Array(channels)) = map.get("channels") {
                for channel in channels {
                    if let Some(channel) = channel.as_str() {
                        list.push(channel.to_string());
                    }
                }
            }
        }
        Value::String(channel) => {
            list.push(channel.to_string());
        }
        _ => {
            return Err(Error {
                error: String::from("Invalid params"),
            })
        }
    }
    ctx.address().do_send(RegisterChannels(list));
    command.respond("ok")
}

pub fn handle_site_list(ws: &ZeruWebsocket, command: &Command) -> Result<Message, Error> {
    trace!("Handling SiteList : {:?}", command.params);
    let mut connecting = false;
    if let Value::Object(map) = &command.params {
        if let Some(Value::Bool(value)) = map.get("connecting_sites") {
            connecting = *value;
        }
    }
    let sites = block_on(ws.site_controller.send(SiteInfoListRequest { connecting }))
        .unwrap()
        .unwrap();
    command.respond(sites)
}

pub fn handle_channel_join_all_site(
    _: &ZeruWebsocket,
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

pub fn handle_site_bad_files(ws: &ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    trace!("Handling SiteBadFiles request");
    let bad_files = block_on(ws.site_controller.send(SiteBadFilesRequest {
        address: ws.address.address.clone(),
    }))?;
    cmd.respond(bad_files)
}

pub fn handle_site_list_modified_files(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_site_pause(ws: &mut ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    let res = block_on(ws.site_controller.send(SitePauseRequest {
        address: ws.address.address.clone(),
    }))?;
    ws.update_websocket();
    if res.is_err() {
        return Err(Error {
            error: format!("Unknown site: {}", ws.address.address),
        });
    }
    cmd.respond("Paused")
}

pub fn handle_site_resume(ws: &ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    let res = block_on(ws.site_controller.send(SiteResumeRequest {
        address: ws.address.address.clone(),
    }))?;
    if res.is_err() {
        return Err(Error {
            error: format!("Unknown site: {}", ws.address.address),
        });
    }
    cmd.respond("Resumed")
}

pub fn handle_site_delete(ws: &ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    let res = block_on(ws.site_controller.send(SiteDeleteRequest {
        address: ws.address.address.clone(),
    }))?;
    let data_res = block_on(ws.user_controller.send(UserSiteDataDeleteRequest {
        user_addr: String::from("current"),
        site_addr: ws.address.address.clone(),
    }))?;
    if data_res.is_err() | res.is_err() {
        return Err(Error {
            error: format!("Unknown site: {}", ws.address.address),
        });
    }
    cmd.respond("Deleted")
}

pub fn handle_site_set_settings_value(ws: &ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    let params = cmd.params.as_array().unwrap();
    let key = params[0].as_str().unwrap();
    if key != "modified_files_notification" {
        return Err(Error {
            error: format!("Can't change this key"),
        });
    }
    let res = block_on(ws.site_controller.send(SiteSetSettingsValueRequest {
        address: ws.address.address.clone(),
        key: key.to_string(),
        value: params[1].clone(),
    }))?;
    if res.is_err() {
        return Err(Error {
            error: format!("Unknown site: {}", ws.address.address),
        });
    }
    cmd.respond("ok")
}

pub fn handle_permission_add(ws: &ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    let params = cmd.params.as_str().unwrap();
    let res = block_on(ws.site_controller.send(SitePermissionAddRequest {
        address: ws.address.address.clone(),
        permission: params.to_string(),
    }))?;
    if res.is_err() {
        return Err(Error {
            error: format!("Unknown site: {}", ws.address.address),
        });
    }
    cmd.respond("ok")
}

pub fn handle_permission_remove(ws: &ZeruWebsocket, cmd: &Command) -> Result<Message, Error> {
    let params = cmd.params.as_str().unwrap();
    let res = block_on(ws.site_controller.send(SitePermissionRemoveRequest {
        address: ws.address.address.clone(),
        permission: params.to_string(),
    }))?;
    if res.is_err() {
        return Err(Error {
            error: format!("Unknown site: {}", ws.address.address),
        });
    }
    cmd.respond("ok")
}

pub fn handle_permission_details(cmd: &Command) -> Result<Message, Error> {
    let key = cmd.params.as_str().unwrap();
    let details = SITE_PERMISSIONS_DETAILS
        .get(key)
        .cloned()
        .ok_or_else(|| Error {
            error: format!("Unknown permission: {}", key),
        });
    cmd.respond(details?)
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
