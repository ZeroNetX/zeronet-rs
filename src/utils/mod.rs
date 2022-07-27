pub mod diff;
pub mod msgpack;

use std::default::Default;

use rusqlite::types::{Type, Value};

pub fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

pub fn gen_peer_id() -> String {
    let vec: Vec<u8> = (0..12).map(|_| rand::random::<u8>()).collect();
    let peer_id = format!("-UT3530-{}", base64::encode(&vec));
    peer_id
}

pub fn to_json_value(value: &Value) -> serde_json::Value {
    match value.data_type() {
        Type::Integer => {
            if let Value::Integer(i) = value {
                serde_json::Value::Number(serde_json::Number::from(*i))
            } else {
                unimplemented!()
            }
        }
        Type::Real => {
            if let Value::Real(i) = value {
                serde_json::Value::Number(serde_json::Number::from_f64(*i).unwrap())
            } else {
                unimplemented!()
            }
        }
        Type::Text => {
            if let Value::Text(i) = value {
                serde_json::Value::String(i.to_string())
            } else {
                unimplemented!()
            }
        }
        _ => serde_json::Value::Null,
    }
}
