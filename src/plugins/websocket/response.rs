use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::utils::is_default;

#[derive(Serialize, Deserialize)]
pub struct Message {
    cmd: MessageType,
    #[serde(skip_serializing_if = "is_default")]
    pub id: usize,
    #[serde(skip_serializing_if = "is_default")]
    to: isize,
    result: serde_json::Value,
}

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MessageType {
    Command,
    InjectScript,
    Response,
    Error,
    Ping,
}

impl Message {
    pub fn new(id: isize, body: serde_json::Value) -> Message {
        Message {
            cmd: MessageType::Response,
            to: id,
            result: body,
            id: 0,
        }
    }

    pub fn inject_script(id: isize, body: serde_json::Value) -> Message {
        Message {
            cmd: MessageType::InjectScript,
            to: id,
            result: body,
            id: 0,
        }
    }

    pub fn is_command(&self) -> bool {
        self.cmd == MessageType::Command
    }

    pub fn command() -> Message {
        Message {
            cmd: MessageType::Command,
            to: 0,
            result: json!(null),
            id: 0,
        }
    }
}
