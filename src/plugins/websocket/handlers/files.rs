use std::{fs::File, io::Read, path::Path};

use actix_web_actors::ws::WebsocketContext;
use futures::executor::block_on;
use log::*;
use serde_json::{json, Value};

use super::super::{error::Error, request::Command, response::Message, ZeruWebsocket};
use crate::{
    environment::ENV,
    plugins::site_server::handlers::sites::{FileGetRequest, FileRulesRequest},
};

pub fn handle_file_need(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_file_get(
    ws: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    trace!("Handling FileGet request {:?}", command);
    let msg: FileGetRequest = match serde_json::from_value(command.params.clone()) {
        Ok(m) => m,
        Err(e) => {
            error!("{:?}", e);
            // TODO: error
            FileGetRequest::default()
        }
    };
    let mut path = (*ENV.data_path).to_path_buf();
    path.push(Path::new(&format!("{}/{}", ws.address, msg.inner_path)));
    assert!(msg.format.is_empty() || msg.format == "text");
    assert!(msg.format != "base64");
    if !path.is_file() {
        let res = block_on(ws.site_addr.send(msg))?;
        if res.is_err() {
            return Err(Error {
                error: String::from("File not found"),
            });
        }
    }
    let mut file = File::open(path)?;
    let mut string = String::new();
    file.read_to_string(&mut string)?;
    command.respond(string)
}

pub fn handle_file_rules(
    ws: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    command: &Command,
) -> Result<Message, Error> {
    let msg = match &command.params {
        Value::String(inner_path) => FileRulesRequest {
            inner_path: inner_path.clone(),
        },
        others => match serde_json::from_value(others.clone()) {
            Ok(m) => m,
            Err(_e) => {
                error!("{:?}", command);
                // TODO: error
                FileRulesRequest::default()
            }
        },
    };
    let mut rules = block_on(ws.site_addr.send(msg))?;
    if rules.is_none() {
        //TODO! Don't Send Empty Rules
        // return Err(Error {
        //     error: String::from("File not found"),
        // });
        rules = Some(json!({"":""}));
    }
    command.respond(rules.unwrap())
}

pub fn handle_file_query(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_file_write(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_file_delete(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_file_list(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}

pub fn handle_dir_list(
    _: &ZeruWebsocket,
    _: &mut WebsocketContext<ZeruWebsocket>,
    _: &Command,
) -> Result<Message, Error> {
    unimplemented!("Please File a Bug Report")
}
