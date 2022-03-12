use log::*;
use serde::{Serialize, Serializer};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::{fmt::Display, str::FromStr};

use super::error::Error;

#[derive(Hash, PartialEq, Eq, Debug, Clone, Default)]
pub struct Address {
    pub address: String,
}

impl Address {
    // digest of Sha256 hash of ASCII encoding
    pub fn get_address_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::default();
        hasher.update(&self.address);
        hasher.finalize().to_vec()
    }

    // digest of Sha1 hash of ACII encoding
    pub fn get_address_sha1(&self) -> Vec<u8> {
        let mut hasher = Sha1::default();
        hasher.update(&self.address);
        hasher.finalize().to_vec()
    }
    // first 6 and last 4 characters of address
    pub fn get_address_short(&self) -> String {
        if self.address.as_str() == "Test" {
            return self.address.clone();
        }
        let l = self.address.len();
        let f = self.address.get(0..6).unwrap();
        let b = self.address.get(l - 5..l).unwrap();
        format!("{f}...{b}")
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.address)
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.address)
    }
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(string: &str) -> Result<Address, Error> {
        let s = String::from(string);
        if string == "Test" {
            return Ok(Address {
                address: String::from(string),
            });
        }
        if s.len() > 34 || s.len() < 33 || !s.starts_with('1') {
            error!(
                "Length should be 34 or 33, was {}, and start with a '1'.",
                string.len(),
            );
            return Err(Error::AddressError(format!(
                "Address length {} is invalid",
                string
            )));
        }
        Ok(Address {
            address: String::from(string),
        })
    }
}

#[cfg(test)]
#[cfg_attr(tarpaulin, ignore)]
mod tests {
    use super::*;

    const ADDR: &str = "1HELLoE3sFD9569CLCbHEAVqvqV7U2Ri9d";
    const ADDR_BYTES_SHA1: [u8; 20] = [
        94, 203, 117, 14, 139, 139, 108, 252, 196, 40, 138, 107, 148, 232, 252, 162, 23, 90, 39,
        140,
    ];
    const ADDR_BYTES_SHA256: [u8; 32] = [
        142, 239, 178, 129, 140, 186, 44, 193, 168, 215, 172, 64, 124, 49, 85, 239, 79, 220, 36,
        50, 4, 164, 198, 156, 248, 78, 156, 105, 136, 53, 31, 56,
    ];
    #[test]
    fn test_creation() {
        let result = Address::from_str(ADDR);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_serialization() {
        let result = Address::from_str(ADDR);
        assert_eq!(result.is_ok(), true, "Encountered error: {:?}", result);
        let address = result.unwrap();
        let result = serde_json::to_string(&address);
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), "\"1HELLoE3sFD9569CLCbHEAVqvqV7U2Ri9d\"");
    }

    #[test]
    fn test_get_address_hash() {
        let result = Address::from_str(ADDR);
        assert_eq!(result.is_ok(), true, "Encountered error: {:?}", result);
        let address_hash = result.unwrap().get_address_hash();
        let b = Vec::from(ADDR_BYTES_SHA256);
        assert_eq!(address_hash, b);
    }
    #[test]
    fn test_get_address_sha1() {
        let result = Address::from_str(ADDR);
        assert_eq!(result.is_ok(), true, "Encountered error: {:?}", result);
        let address_hash = result.unwrap().get_address_sha1();
        let b = Vec::from(ADDR_BYTES_SHA1);
        assert_eq!(address_hash, b);
    }
    #[test]
    fn test_get_address_short() {
        let result = Address::from_str(ADDR);
        assert_eq!(result.is_ok(), true, "Encountered error: {:?}", result);
        let address_hash = result.unwrap().get_address_short();
        assert_eq!(&address_hash, "1HELLo...2Ri9d");
    }

    // #[test]
    // fn test_deserialization() {
    //     let result = serde_json::from_str("\"1HELLoE3sFD9569CLCbHEAVqvqV7U2Ri9d\"");
    //     assert_eq!(result.is_ok(), true, "Encountered error: {:?}", result);
    //     let address: Address = result.unwrap();
    //     assert_eq!(
    //         address,
    //         Address {
    //             address: String::from(ADDR)
    //         }
    //     );
    // }
}
