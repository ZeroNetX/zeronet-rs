use actix_web_actors::ws::WebsocketContext;
use futures::executor::block_on;
use log::*;
use serde_json::Value;

use super::super::{error::Error, request::Command, response::Message, ZeruWebsocket};
use crate::controllers::handlers::{
    sites::{DBQueryRequest, SiteInfoRequest},
    users::UserSiteData,
};

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
    if let Some(user_site_data) = block_on(ws.user_controller.send(UserSiteData {
        user_addr: String::from("current"),
        site_addr: ws.address.address.clone(),
    }))
    .unwrap()
    {
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
                let params = inner_path.get(1);
                if params.is_none() {
                    let res = block_on(ws.site_controller.send(DBQueryRequest {
                        address: ws.address.address.clone(),
                        query: query.to_string(),
                    }))
                    .unwrap()
                    .unwrap();
                    return command.respond(res);
                }
                error!("{:?}", command);
                return Err(Error {
                    error: String::from("params are not implemented yet"),
                });
            }
            error!("{:?}", command);
            Err(Error {
                error: String::from("params are not implemented yet"),
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
    _: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    debug!("Handling ChannelJoin request using dummy response");
    command.respond(String::from("ok"))
}
