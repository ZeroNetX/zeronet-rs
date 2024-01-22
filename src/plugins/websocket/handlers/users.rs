use std::collections::HashMap;

use futures::executor::block_on;
use log::*;
use serde_json::{json, Value};

use super::super::{error::Error, request::Command, response::Message, ZeruWebsocket};
use crate::{
    core::user::User,
    plugins::site_server::handlers::users::{UserRequest, UserSetSiteCertRequest, UserSettings},
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

pub fn handle_user_get_settings(ws: &ZeruWebsocket, command: &Command) -> Result<Message, Error> {
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

pub fn handle_user_set_settings(ws: &ZeruWebsocket, command: &Command) -> Result<Message, Error> {
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
        ..Default::default()
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
    command: &Command,
) -> Result<Message, Error> {
    let user = get_current_user(ws)?;
    command.respond(user.settings)
}

pub fn handle_user_set_global_settings(
    ws: &ZeruWebsocket,
    command: &Command,
) -> Result<Message, Error> {
    info!("Handling UserSetGlobalSettings");
    if let Value::Array(value) = command.params.clone() {
        let content_map = value.first();
        if let Some(Value::Object(settings)) = content_map {
            #[allow(clippy::unnecessary_to_owned)]
            let settings = settings
                .to_owned()
                .into_iter()
                .collect::<HashMap<String, Value>>();
            let mut content_map = HashMap::new();
            content_map.insert(ws.address.clone(), settings);
            let _ = block_on(ws.user_controller.send(UserSettings {
                set: true,
                global: true,
                user_addr: String::from("current"),
                settings: Some(content_map),
                ..Default::default()
            }))?;
            command.respond("ok")
        } else {
            command.respond(Error {
                error: String::from("Invalid settings"),
            })
        }
    } else {
        Err(Error {
            error: String::from("Invalid User Settings"),
        })
    }
}

pub fn _handle_user_show_master_seed(
    ws: &ZeruWebsocket,
    command: &Command,
) -> Result<Message, Error> {
    let user = get_current_user(ws)?;
    command.respond(user.get_master_seed())
}

pub fn handle_cert_set(ws: &ZeruWebsocket, command: &Command) -> Result<Message, Error> {
    trace!("Handling CertSet with command: {:?}", command);
    let site = ws.address.address.clone();
    let provider = command.params[0].as_str().unwrap().to_string();
    let _ = block_on(ws.user_controller.send(UserSetSiteCertRequest {
        user_addr: String::from("current"),
        site_addr: site,
        provider,
    }))?;
    command.respond("ok")
}

pub fn handle_cert_list(ws: &ZeruWebsocket, command: &Command) -> Result<Message, Error> {
    let user = get_current_user(ws)?;
    let site = ws.address.address.clone();
    let curr_site_auth_addr = user
        .sites
        .get(&site)
        .unwrap()
        .get_auth_pair()
        .unwrap()
        .auth_address;
    let mut certs = vec![];
    for (domain, cert) in user.certs {
        let auth_addr = cert.get_auth_pair().auth_address;
        let map = json!({
            "auth_address": auth_addr,
            "auth_type": cert.auth_type,
            "auth_user_name": cert.auth_user_name,
            "domain": domain,
            "selected": auth_addr == curr_site_auth_addr,
        });
        certs.push(map);
    }
    command.respond(certs)
}
