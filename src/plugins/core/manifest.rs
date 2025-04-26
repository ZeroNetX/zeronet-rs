use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use zeronet_cryptography::error::CryptError;
use zerucontent::sort::sort_json;

use crate::{core::error::Error, plugins::core::plugin::Plugin};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PluginManifest {
    #[serde(flatten)]
    pub plugin: Plugin,
    pub signs: HashMap<String, String>,
    pub plugin_signature: String,
}

impl PluginManifest {
    pub async fn load(name: &str) -> Result<Self, serde_json::Error> {
        let manifest_path = PathBuf::from(format!("plugins/{name}/manifest.json"));
        let manifest_str = tokio::fs::read_to_string(&manifest_path).await.unwrap();
        serde_json::from_str(&manifest_str)
    }

    pub async fn sign_plugin(&mut self, private_key: &str) -> Result<Self, Error> {
        self.sign(private_key)?;
        let plugin_file = PathBuf::from(format!(
            "plugins/{name}/{name}.wasm",
            name = self.plugin.name
        ));
        let bytes = tokio::fs::read(plugin_file).await.unwrap();
        let plugin_signature = zeronet_cryptography::sign(bytes, private_key).unwrap();
        let manifest = PluginManifest {
            plugin_signature,
            ..self.clone()
        };
        Ok(manifest)
    }

    pub async fn verify_plugin(&self) -> Result<bool, Error> {
        self.verify()?;
        let plugin_file = PathBuf::from(format!(
            "plugins/{name}/{name}.wasm",
            name = self.plugin.name
        ));
        let bytes = tokio::fs::read(plugin_file).await.unwrap();
        let signature = &self.plugin_signature;
        let mut verified = false;
        for key in self.signs.keys() {
            if verified {
                continue;
            }
            verified = zeronet_cryptography::verify(bytes.clone(), key, signature).is_ok();
        }
        Ok(verified)
    }

    pub fn sign(&mut self, private_key: &str) -> Result<Self, Error> {
        let mut data = sort_json(serde_json::to_value(&self)?)?;
        let data = data.as_object_mut().unwrap();
        for key in ["path", "plugin_signature", "signs"] {
            data.remove(key);
        }
        let data = serde_json::to_string(&data)?;
        let signature = zeronet_cryptography::sign(data, private_key)?;
        let public_key = zeronet_cryptography::privkey_to_pubkey(private_key)?;
        let mut signs = self.signs.clone();
        signs.insert(public_key, signature);
        let signed_manifest = PluginManifest {
            signs,
            ..self.clone()
        };
        Ok(signed_manifest)
    }

    pub fn verify(&self) -> Result<bool, Error> {
        let mut data = sort_json(serde_json::to_value(self)?)?;
        let data = data.as_object_mut().unwrap();
        for key in ["path", "plugin_signature", "signs"] {
            data.remove(key);
        }
        let data = serde_json::to_string(&data)?;
        let mut valid = false;
        for (key, sign) in &self.signs {
            if valid {
                continue;
            }
            let res = zeronet_cryptography::verify(data.clone(), key, sign);
            match res {
                Ok(_) => valid = true,
                Err(e) => match e {
                    CryptError::AddressMismatch(_) => {} //valid = valid || false
                    _ => return Err(Error::CryptError(e.to_string())),
                },
            }
        }
        Ok(valid)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::PluginManifest;

    const PRIV_KEY: &str = "5KWn89CnWBSdq1Psx64R8nthtw8mv2sxZxcThiyEs9SyJFMLyGD";
    const PUB_KEY: &str = "1K5Co1W2S6QRjQLyPJofk4my9PimYVRKXA";
    const SIGN: &str =
        "HEt7+o54kOxxmkl0p0jo5WQ26P02fcdzxbllzot2s4TJf9vcdgcldzJd/mXMhBeC22rRbgyA79wnEn44a8zW/LM=";

    const CORRUPT_PUB_KEY: &str = "1K5Co1W2S6QRjQLyPJoFk4my9PimYVRKXA";
    const CORRUPT_SIGN: &str =
        "Gxmix0o/F7y79GMHb7yIGjm3kinEhY/1LjVivXBCihY2BAxoLv+VVZbQGxc7npWmE+CE0K0NAhwEfKYcHZOUs90=";
    const WRONG_SIGN: &str =
        "HMnH8SJM26o7ClonfMHPxZ5d2kFWE0b25M2nElnPq/JvEG7kHXYxPt+C9d1oEr4RVmxe1Deurm8I2fk1HHzXDIk=";

    #[test]
    fn sign() {
        let mut manifest = PluginManifest::default();
        manifest.sign(PRIV_KEY).unwrap();
        let res = manifest.verify();
        assert!(res.is_ok());
    }

    #[test]
    fn verify() {
        let mut signs = HashMap::new();
        signs.insert(PUB_KEY.into(), SIGN.into());
        let manifest = PluginManifest {
            signs,
            ..Default::default()
        };
        assert!(manifest.verify().is_ok());
        assert!(manifest.verify().unwrap());
    }

    #[test]
    fn verify_with_wrong_key() {
        let mut signs = HashMap::new();
        signs.insert(CORRUPT_PUB_KEY.into(), SIGN.into());
        let manifest = PluginManifest {
            signs,
            ..Default::default()
        };
        assert!(manifest.verify().is_ok());
        assert!(!manifest.verify().unwrap());
    }

    #[test]
    fn verify_with_wrong_sign() {
        let mut signs = HashMap::new();
        signs.insert(PUB_KEY.into(), WRONG_SIGN.into());
        let manifest = PluginManifest {
            signs,
            ..Default::default()
        };
        assert!(manifest.verify().is_ok());
        assert!(!manifest.verify().unwrap());
    }

    #[test]
    fn verify_with_corrupt_sign() {
        let mut signs = HashMap::new();
        signs.insert(PUB_KEY.into(), CORRUPT_SIGN.into());
        let manifest = PluginManifest {
            signs,
            ..Default::default()
        };
        assert!(manifest.verify().is_ok());
        assert!(!manifest.verify().unwrap());
    }

    #[test]
    fn verify_should_fail() {
        let mut signs = HashMap::new();
        signs.insert(CORRUPT_PUB_KEY.into(), CORRUPT_SIGN.into());
        let manifest = PluginManifest {
            signs,
            ..Default::default()
        };
        assert!(manifest.verify().is_ok());
        assert!(!manifest.verify().unwrap());
    }
}
