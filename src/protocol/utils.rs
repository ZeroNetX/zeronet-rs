use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use serde_json::Number;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Bytes(ByteBuf),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}
