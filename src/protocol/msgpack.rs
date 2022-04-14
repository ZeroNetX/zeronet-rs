use rmp_serde::{
    decode::{self, Error as DecodeError},
    encode::{self, Error as EncodeError},
};
use serde::{de::DeserializeOwned, Serialize};

pub fn pack(val: impl Serialize + Sized) -> Result<Vec<u8>, EncodeError> {
    encode::to_vec_named(&val)
}

pub fn unpack<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, DecodeError> {
    decode::from_slice(&bytes)
}

pub fn write_packed(
    val: impl Serialize + Sized,
    writer: &mut dyn std::io::Write,
) -> Result<(), EncodeError> {
    encode::write(writer, &val)
}

#[cfg(test)]
mod tests {
    use super::{pack, unpack};
    use crate::protocol::message::Response;
    use serde_bytes::ByteBuf;
    use serde_json::json;

    #[test]
    fn pack_unpack() {
        let val = json!({
            "cmd": "ping",
            "req_id": 0,
        });

        let res = pack(val);
        assert!(res.is_ok());
        let bytes = res.unwrap();
        let res = unpack::<serde_json::Value>(&bytes);
        assert!(res.is_ok());
    }

    #[test]
    fn test_bin() {
        use super::super::templates::GetFileResponse;
        let bytes = b"0";
        let len = bytes.len();
        let v = GetFileResponse {
            body: ByteBuf::from(bytes),
            location: 0,
            size: len,
        };
        let res = Response {
            cmd: "getFile".to_string(),
            to: 0,
            body: v,
        };
        let res = pack(res);
        assert!(res.is_ok());
        let bytes = [
            131, 163, 99, 109, 100, 167, 103, 101, 116, 70, 105, 108, 101, 162, 116, 111, 0, 164,
            98, 111, 100, 121, 131, 164, 98, 111, 100, 121, 196, 1, 48, 168, 108, 111, 99, 97, 116,
            105, 111, 110, 0, 164, 115, 105, 122, 101, 1,
        ];
        assert_eq!(res.unwrap(), bytes);
    }
}
