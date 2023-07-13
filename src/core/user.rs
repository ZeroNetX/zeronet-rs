use super::error::Error;
#[cfg(feature = "userio")]
use super::io::UserIO;
use futures::executor::block_on;
use log::*;
use models::*;
use num_bigint::BigUint;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

pub mod models {
    use serde::{Deserialize, Serialize};
    use std::collections::{BTreeMap, HashMap};

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
    pub struct AuthPair {
        pub auth_address: String,
        #[serde(rename = "auth_privatekey")]
        auth_privkey: String,
    }

    impl AuthPair {
        pub fn new(auth_address: String, auth_privkey: String) -> Self {
            AuthPair {
                auth_address,
                auth_privkey,
            }
        }

        pub fn get_auth_privkey(&self) -> &str {
            &self.auth_privkey
        }
    }

    #[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
    pub struct Cert {
        auth_pair: AuthPair,
        pub auth_type: String,
        pub auth_user_name: String,
        pub cert_sign: String,
    }

    impl Cert {
        pub fn new(
            auth_pair: AuthPair,
            auth_type: String,
            auth_user_name: String,
            cert_sign: String,
        ) -> Self {
            Cert {
                auth_pair,
                auth_type,
                auth_user_name,
                cert_sign,
            }
        }

        pub fn get_auth_pair(&self) -> AuthPair {
            self.auth_pair.clone()
        }

        pub fn get_cert_sign(&self) -> &str {
            &self.cert_sign
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct SiteData {
        #[serde(skip_serializing)]
        pub address: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub index: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cert_provider: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(flatten)]
        auth_pair: Option<AuthPair>,
        #[serde(skip_serializing_if = "Option::is_none")]
        privatekey: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        settings: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "HashMap::is_empty")]
        plugin_data: HashMap<String, serde_json::Value>,
    }

    impl SiteData {
        pub fn new(address: String) -> Self {
            SiteData {
                address,
                index: None,
                cert_provider: None,
                auth_pair: None,
                privatekey: None,
                settings: None,
                plugin_data: HashMap::new(),
            }
        }

        pub fn create(address: String, index: u32, auth_pair: AuthPair, privkey: String) -> Self {
            SiteData {
                address,
                index: Some(index),
                cert_provider: None,
                auth_pair: Some(auth_pair),
                privatekey: Some(privkey),
                settings: None,
                plugin_data: HashMap::new(),
            }
        }

        pub fn with_index(&mut self, index: u32) -> Self {
            self.index = Some(index);
            self.to_owned()
        }

        pub fn get_index(&self) -> Option<u32> {
            self.index
        }

        pub fn get_cert_provider(&self) -> Option<String> {
            self.cert_provider.clone()
        }

        pub fn add_cert_provider(&mut self, cert_provider: String) {
            self.cert_provider = Some(cert_provider);
        }

        pub fn delete_cert_provider(&mut self) {
            self.cert_provider = None;
        }

        pub fn with_auth_pair(&mut self, auth_pair: AuthPair) -> Self {
            self.auth_pair = Some(auth_pair);
            self.to_owned()
        }

        pub fn get_auth_pair(&self) -> Option<AuthPair> {
            self.auth_pair.clone()
        }

        pub fn with_privatekey(&mut self, priv_key: String) -> Self {
            self.privatekey = Some(priv_key);
            self.to_owned()
        }

        pub fn get_privkey(&self) -> Option<String> {
            self.privatekey.clone()
        }

        pub fn get_settings(&self) -> Option<serde_json::Value> {
            self.settings.clone()
        }

        pub fn set_settings(&mut self, settings: serde_json::Value) -> Self {
            self.settings = Some(settings);
            self.to_owned()
        }

        pub fn get_plugin_data(&self) -> &HashMap<String, serde_json::Value> {
            &self.plugin_data
        }

        pub fn get_plugin_data_mut(&mut self) -> &mut HashMap<String, serde_json::Value> {
            &mut self.plugin_data
        }

        pub fn add_plugin_data(&mut self, key: String, value: serde_json::Value) {
            self.plugin_data.insert(key, value);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    #[serde(skip_serializing, skip_deserializing)]
    pub master_address: String,
    master_seed: String,
    pub sites: HashMap<String, SiteData>,
    pub certs: HashMap<String, Cert>,
    pub settings: HashMap<String, serde_json::Value>,
}

impl Default for User {
    fn default() -> Self {
        Self::new()
    }
}

impl User {
    /// Creates a new user with a new seed and address pair
    pub fn new() -> User {
        let (master_seed, master_address) = zeronet_cryptography::create();
        User {
            master_seed: master_seed.to_string(),
            master_address,
            sites: HashMap::new(),
            certs: HashMap::new(),
            settings: HashMap::new(),
        }
    }

    fn get_site_keypair_from_seed(&self, seed: &str, index: Option<u32>) -> (String, String, u32) {
        let index = index.unwrap_or(thread_rng().gen_range(0..29639936));
        let privkey = zeronet_cryptography::hd_privkey(seed, index);
        let wif_privkey = zeronet_cryptography::privkey_to_wif(privkey);
        let address = zeronet_cryptography::privkey_to_pubkey(&wif_privkey).unwrap();

        (wif_privkey, address, index)
    }

    fn generate_site_keypair(&self) -> (String, String, u32) {
        let (privkey, address) = zeronet_cryptography::create();
        let wif_privkey = zeronet_cryptography::privkey_to_wif(privkey);
        let index = Self::get_address_auth_index(&address);

        (wif_privkey, address, index)
    }

    /// Creates a new user from a seed
    pub fn from_seed(master_seed: String) -> User {
        let privkey = zeronet_cryptography::seed_to_privkey(&master_seed).unwrap();
        let wif_privkey = zeronet_cryptography::privkey_to_wif(privkey);
        let master_address = zeronet_cryptography::privkey_to_pubkey(&wif_privkey).unwrap();
        User {
            master_seed,
            master_address,
            sites: HashMap::new(),
            certs: HashMap::new(),
            settings: HashMap::new(),
        }
    }

    pub fn get_master_seed(&self) -> String {
        //TODO: Check for permissions
        self.master_seed.clone()
    }

    pub fn get_address_auth_index(address: &str) -> u32 {
        let bytes = address.bytes();
        let hexs: Vec<String> = bytes.into_iter().map(|x| format!("{:02X}", x)).collect();
        let hex = &hexs.join("");

        let auth_index = BigUint::parse_bytes(hex.as_bytes(), 16).unwrap();

        // We don't need the whole index
        (auth_index % BigUint::from(100000000u32)).to_u32_digits()[0]
    }

    fn generate_auth_address(&mut self, address: &str) -> AuthPair {
        let start_time = SystemTime::now();
        let address_id = User::get_address_auth_index(address);
        let (auth_privkey, auth_pubkey, _) =
            self.get_site_keypair_from_seed(&self.master_seed, Some(address_id));
        let auth_pair = AuthPair::new(auth_pubkey, auth_privkey);

        let site_data = SiteData::new(address.to_string()).with_auth_pair(auth_pair.clone());
        self.sites.insert(address.to_string(), site_data);

        #[cfg(feature = "userio")]
        #[cfg(not(test))]
        block_on(self.save());

        debug!(
            "Added new site: {} in {}s",
            address,
            SystemTime::now()
                .duration_since(start_time)
                .unwrap()
                .as_secs_f32()
        );

        auth_pair
    }

    /// Get user site data
    ///
    /// Return: {"auth_address": "1AddR", "auth_privatekey": "xxx"}
    pub fn get_site_data(&mut self, address: &str, create: bool) -> SiteData {
        if let Some(site_auth_data) = self.sites.get(address) {
            site_auth_data.to_owned()
        } else {
            if create {
                self.generate_auth_address(address);
                return self.sites.get(address).unwrap().to_owned();
            }
            SiteData::new(address.to_string()) // empty
        }
    }

    fn delete_site_data(&mut self, address: &str) {
        if self.sites.remove(address).is_some() {
            #[cfg(feature = "userio")]
            #[cfg(not(test))]
            block_on(self.save());

            debug!("Deleted site: {}", address);
        }
    }

    fn set_site_settings(&mut self, address: &str, settings: serde_json::Value) -> SiteData {
        #[allow(clippy::let_and_return)]
        let site_data = self.get_site_data(address, true).set_settings(settings);

        #[cfg(feature = "userio")]
        #[cfg(not(test))]
        block_on(self.save());

        site_data
    }

    /// Get data for a new, unique site
    ///
    /// Return: [site_address, bip32_index, {"auth_address": "1AddR", "auth_privatekey": "xxx", "privatekey": "xxx"}]
    pub fn get_new_site_data(&mut self, with_seed: bool) -> SiteData {
        let (site_privkey, site_address, bip32_idx) = loop {
            if with_seed {
                let keypair = self.get_site_keypair_from_seed(&self.master_seed, None);
                if self.sites.get(&keypair.1).is_none() {
                    break keypair;
                }
            } else {
                let keypair = self.generate_site_keypair();
                if self.sites.get(&keypair.1).is_none() {
                    break keypair;
                }
            }

            info!("Info: Site already exists, creating a new one");
        };

        let site_data = self
            .get_site_data(&site_address, true)
            .with_index(bip32_idx)
            .with_privatekey(site_privkey);

        self.sites
            .insert(site_address.to_string(), site_data.clone());

        #[cfg(feature = "userio")]
        #[cfg(not(test))]
        block_on(self.save());
        site_data
    }

    fn get_auth_pair(&mut self, address: &str, create: bool) -> Option<AuthPair> {
        if let Some(cert) = self.get_cert(address) {
            Some(cert.get_auth_pair())
        } else {
            let site_data = self.get_site_data(address, create);
            site_data.get_auth_pair()
        }
    }

    /// Get BIP32 address from site address
    ///
    /// Return: BIP32 auth address
    fn get_auth_address(&mut self, address: &str, create: bool) -> Option<String> {
        let auth_pair = self.get_auth_pair(address, create)?;
        Some(auth_pair.auth_address)
    }

    fn get_auth_privkey(&mut self, address: &str, create: bool) -> Option<String> {
        let auth_pair = self.get_auth_pair(address, create)?;
        Some(auth_pair.get_auth_privkey().to_owned())
    }

    /// Add cert for the user
    fn add_cert(
        &mut self,
        auth_address: &str,
        domain: &str,
        auth_type: &str,
        auth_username: &str,
        cert_sign: &str,
    ) -> bool {
        let auth_pair: Option<AuthPair> = self.sites.values().find_map(|site_data| {
            let auth_pair = site_data.get_auth_pair()?;
            if auth_pair.auth_address == auth_address {
                return Some(auth_pair);
            }
            None
        });

        if auth_pair.is_none() {
            return false;
        }

        let cert_node = Cert::new(
            auth_pair.unwrap(),
            auth_type.to_string(),
            auth_username.to_string(),
            cert_sign.to_string(),
        );

        let cert = self.certs.get(domain);

        if cert.is_some() && (cert != Some(&cert_node)) {
            false
        // } else if cert == Some(&cert_node) {
        //     false
        } else {
            self.certs.insert(domain.to_string(), cert_node);

            #[cfg(feature = "userio")]
            #[cfg(not(test))]
            block_on(self.save());

            true
        }
    }

    /// Remove cert from user
    fn delete_cert(&mut self, domain: &str) {
        self.certs.remove(domain);
    }

    /// Set active cert for a site
    fn set_cert(&mut self, address: &str, provider: Option<&str>) -> SiteData {
        let mut site_data = self.get_site_data(address, true);

        if let Some(domain) = site_data.get_cert_provider() {
            if self.certs.get(&domain).is_some() {
                warn!("Warning: Cert already exists");
            }
        }

        if let Some(provider) = provider {
            site_data.add_cert_provider(provider.to_string());
        } else if site_data.get_cert_provider().is_none() {
            site_data.delete_cert_provider();
        }

        #[cfg(feature = "userio")]
        #[cfg(not(test))]
        block_on(self.save());

        site_data
    }

    /// Get cert for the site address
    ///
    /// Return: { "auth_address": "1AddR", "auth_privatekey": "xxx", "auth_type": "web", "auth_user_name": "nofish", "cert_sign": "xxx"} or None
    fn get_cert(&self, address: &str) -> Option<&Cert> {
        let site_data = self.sites.get(address)?;
        let cert = site_data.get_cert_provider()?;
        self.certs.get(&cert)
    }

    /// Get cert user name for the site address
    ///
    /// Return user@certprovider.bit or None
    fn get_cert_user_id(&mut self, address: &str) -> Option<String> {
        let site_data = self.get_site_data(address, false);
        let cert = &self.get_cert(address)?;
        return Some(format!(
            "{}@{}",
            cert.auth_user_name,
            site_data.get_cert_provider().unwrap()
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::models::AuthPair;
    use super::User;

    const EXAMPLE_SITE: &str = "1HELLoE3sFD9569CLCbHEAVqvqV7U2Ri9d";
    const SEED: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const CHILD_INDEX: u32 = 45168996;
    const AUTH_PRIVKEY: &str = "5J3HUZpcNuEMmFMec9haxPJ58GiEHruqYDLtMGtFAumaLMr5dCV";
    const AUTH_ADDR: &str = "1M6UT3GYmPhMYShDKYsLaFehZ5pmc83Mso";
    const CERT_DOMAIN: &str = "zeroxid.bit";
    const CERT_TYPE: &str = "web";
    const CERT_SIGN: &str =
        "HGSx3lZ/Z+SF2n94H/x1raw2ATnMGl/8cXeDiG+HfparYuok261XMGLdSAYs6CzfC2Ppg0LrqhnWve9iGB4xAew=";
    const CERT_USERNAME: &str = "zeronetx";

    #[test]
    fn get_address_auth_index() {
        let index = User::get_address_auth_index(EXAMPLE_SITE);

        assert_eq!(index, CHILD_INDEX);
    }

    #[test]
    fn test_generate_auth_address() {
        let mut user = User::from_seed(SEED.to_string());
        let auth_address = user.generate_auth_address(EXAMPLE_SITE);
        let expected_result = AuthPair::new(AUTH_ADDR.to_string(), AUTH_PRIVKEY.to_string());

        assert_eq!(expected_result, auth_address);
    }

    #[test]
    fn test_get_site_keypair_from_seed() {
        let user = User::from_seed(SEED.to_string());
        let (_, address, _) = user.get_site_keypair_from_seed(&user.master_seed, Some(CHILD_INDEX));
        assert_eq!(address, AUTH_ADDR);
    }

    #[test]
    fn test_get_new_site_data() {
        let mut user = User::from_seed(SEED.to_string());
        let site_data = user.get_new_site_data(true);
        let (_privkey, address, index) =
            user.get_site_keypair_from_seed(&user.master_seed, site_data.index);

        assert_eq!((site_data.address, site_data.index), (address, Some(index)));
    }

    #[test]
    fn test_get_site_data_exist_in_sites() {
        let mut user = User::from_seed(SEED.to_string());

        // adding a site to the sites
        user.generate_auth_address(EXAMPLE_SITE);

        // getting site data
        let site_data = user.get_site_data(EXAMPLE_SITE, true);
        let expected_result = AuthPair::new(AUTH_ADDR.to_string(), AUTH_PRIVKEY.to_string());

        assert_eq!(Some(expected_result), site_data.get_auth_pair());
    }

    #[test]
    fn test_get_site_data_create() {
        let mut user = User::from_seed(SEED.to_string());
        let site_data = user.get_site_data(EXAMPLE_SITE, true);
        let expected_result = AuthPair::new(AUTH_ADDR.to_string(), AUTH_PRIVKEY.to_string());

        assert_eq!(Some(expected_result), site_data.get_auth_pair());
    }

    #[test]
    fn test_get_site_data_not_create() {
        let mut user = User::from_seed(SEED.to_string());
        let site_data = user.get_site_data(EXAMPLE_SITE, false);
        let expected_result = None;

        assert_eq!(expected_result, site_data.get_auth_pair());
    }

    #[test]
    fn test_set_site_settings() {
        let mut user = User::from_seed(SEED.to_string());

        let settings = serde_json::Value::String("Some settings..".to_string());
        let site_data = user.set_site_settings(EXAMPLE_SITE, settings.clone());

        assert_eq!(Some(settings), site_data.get_settings());
    }

    #[test]
    fn test_add_cert_auth_exist() {
        let mut user = User::from_seed(SEED.to_string());

        user.get_site_data(EXAMPLE_SITE, true);
        let result = user.add_cert(AUTH_ADDR, CERT_DOMAIN, CERT_TYPE, CERT_USERNAME, CERT_SIGN);

        assert!(result);
    }

    #[test]
    fn test_add_cert_auth_not_exist() {
        let mut user = User::from_seed(SEED.to_string());

        let result = user.add_cert(AUTH_ADDR, CERT_DOMAIN, CERT_TYPE, CERT_USERNAME, CERT_SIGN);

        assert!(!result);
    }

    #[test]
    fn test_set_cert() {
        let mut user = User::from_seed(SEED.to_string());

        let result = user.set_cert(EXAMPLE_SITE, Some(CERT_DOMAIN));

        assert_eq!(Some(CERT_DOMAIN.to_string()), result.get_cert_provider());
    }

    #[test]
    fn test_get_auth_privkey_with_cert() {
        let mut user = User::from_seed(SEED.to_string());

        user.set_cert(EXAMPLE_SITE, Some(CERT_DOMAIN));

        assert_eq!(
            Some(AUTH_PRIVKEY.to_string()),
            user.get_auth_privkey(EXAMPLE_SITE, true)
        );
    }
    #[test]
    fn test_get_auth_privkey_without_cert() {
        let mut user = User::from_seed(SEED.to_string());

        assert_eq!(
            Some(AUTH_PRIVKEY.to_string()),
            user.get_auth_privkey(EXAMPLE_SITE, true)
        );
    }
    #[test]
    fn test_get_auth_privkey_without_pair() {
        let mut user = User::from_seed(SEED.to_string());

        assert_eq!(
            None,
            user.get_auth_privkey(EXAMPLE_SITE, false /* creates an empty site */)
        );
    }
}
