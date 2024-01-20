use serde::{Deserialize, Serialize};

use crate::utils::is_default;

#[derive(Serialize, Deserialize)]
pub struct Message {
    cmd: MessageType,
    #[serde(skip_serializing_if = "is_default")]
    to: isize,
    result: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageType {
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
        }
    }

    pub fn inject_script(id: isize, body: serde_json::Value) -> Message {
        Message {
            cmd: MessageType::InjectScript,
            to: id,
            result: body,
        }
    }
}
