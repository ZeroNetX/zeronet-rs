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
    info!("Handling UserSetSettings");
    let extracted = extract_user_settings(command.params.clone());
    match extracted {
        Ok(settings) => {
            let mut content_map = HashMap::new();
            content_map.insert(ws.address.clone(), settings);
            let _ = block_on(ws.user_controller.send(UserSettings {
                set: true,
                user_addr: String::from("current"),
                site_addr: ws.address.address.clone(),
                settings: Some(content_map),
                ..Default::default()
            }))?;
            command.respond("ok")
        }
        Err(error) => command.respond(Error { error }),
    }
}

pub fn handle_user_get_global_settings(
    ws: &ZeruWebsocket,
    command: &Command,
) -> Result<Message, Error> {
    info!("Handling UserGetGlobalSettings");
    let user = get_current_user(ws)?;
    command.respond(user.settings)
}

pub fn handle_user_set_global_settings(
    ws: &ZeruWebsocket,
    command: &Command,
) -> Result<Message, Error> {
    info!("Handling UserSetGlobalSettings");
    let extracted = extract_user_settings(command.params.clone());
    match extracted {
        Ok(settings) => {
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
        }
        Err(error) => command.respond(Error { error }),
    }
}

fn extract_user_settings(settings: Value) -> Result<HashMap<String, Value>, String> {
    if let Value::Array(value) = settings {
        let content_map = value.first();
        if let Some(Value::Object(settings)) = content_map {
            #[allow(clippy::unnecessary_to_owned)]
            let settings = settings
                .to_owned()
                .into_iter()
                .collect::<HashMap<String, Value>>();

            Ok(settings)
        } else {
            Err(String::from("Invalid settings"))
        }
    } else {
        Err(String::from("Invalid User Settings"))
    }
}

pub fn _handle_user_show_master_seed(
    ws: &ZeruWebsocket,
    command: &Command,
) -> Result<Message, Error> {
    let user = get_current_user(ws)?;
    command.respond(user.get_master_seed())
}

pub fn handle_cert_set(ws: &mut ZeruWebsocket, command: &Command) -> Result<Message, Error> {
    trace!("Handling CertSet with command: {:?}", command);
    let site = ws.address.address.clone();
    let provider = match &command.params {
        Value::String(provider) => Ok(provider.clone()),
        Value::Array(params) => match params.first() {
            Some(Value::String(provider)) => Ok(provider.clone()),
            _ => Err(()),
        },
        Value::Object(params) => match params.get("domain") {
            Some(Value::String(provider)) => Ok(provider.clone()),
            _ => Err(()),
        },
        _ => Err(()),
    };
    if provider.is_err() {
        return Err(Error {
            error: "Invalid params".into(),
        });
    }
    let provider = provider.unwrap();
    let _ = block_on(ws.user_controller.send(UserSetSiteCertRequest {
        user_addr: String::from("current"),
        site_addr: site,
        provider: provider.clone(),
    }))?;
    ws.update_websocket(Some(json!(vec!["cert_changed", &provider])));
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
