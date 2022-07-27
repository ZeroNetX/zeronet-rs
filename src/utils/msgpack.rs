use rmp_serde::{
    decode::{self, Error as DecodeError},
    encode::{self, Error as EncodeError},
};
use serde::{de::DeserializeOwned, Serialize};

pub fn pack(val: impl Serialize + Sized) -> Result<Vec<u8>, EncodeError> {
    encode::to_vec_named(&val)
}

pub fn unpack<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, DecodeError> {
    decode::from_slice(bytes)
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
}
