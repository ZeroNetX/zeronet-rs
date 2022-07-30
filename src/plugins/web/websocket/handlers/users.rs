use std::collections::HashMap;

use actix_web_actors::ws::WebsocketContext;
use futures::executor::block_on;
use log::*;

use super::super::{error::Error, request::Command, response::Message, ZeruWebsocket};
use crate::{
    controllers::handlers::users::{UserRequest, UserSettings},
    core::user::User,
};

pub fn get_current_user(ws: &ZeruWebsocket) -> Result<User, Error> {
    let user = block_on(ws.user_controller.send(UserRequest {
        address: String::from("current"),
    }));
    match user {
        Ok(Some(u)) => Ok(u),
        _ => Err(Error {
            error: String::from("User not found"),
        }),
    }
}

pub fn handle_user_get_settings(
    ws: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    let result = block_on(ws.user_controller.send(UserSettings {
        user_addr: String::from("current"),
        site_addr: ws.address.clone().address,
        ..Default::default()
    }))?;
    if result.is_none() {
        return Err(Error {
            error: String::from("User settings not found"),
        });
    }
    let settings = result.unwrap();
    let mut map = serde_json::Map::new();
    for (key, value) in settings {
        map.insert(key.to_string(), value);
    }
    command.respond(map)
}

pub fn handle_user_set_settings(
    ws: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    warn!("Handling UserGetSettings with dummy response");
    // TODO: actually return user settings
    let user = get_current_user(ws)?;
    let mut map = HashMap::new();
    for (key, value) in user.settings {
        map.insert(key.to_string(), value);
    }

    let mut content_map = HashMap::new();
    content_map.insert(ws.address.clone(), map);

    let result = block_on(ws.user_controller.send(UserSettings {
        set: true,
        user_addr: String::from("current"),
        site_addr: ws.address.clone().address,
        settings: Some(content_map),
    }))?;
    if result.is_none() {
        return Err(Error {
            error: String::from("User settings not found"),
        });
    }

    command.respond("ok")
}

pub fn handle_user_get_global_settings(
    ws: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    let user = get_current_user(ws)?;
    let user_settings = user.settings;
    command.respond(serde_json::to_string(&user_settings)?)
}
