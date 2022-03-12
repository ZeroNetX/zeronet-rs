use super::{
    error::Error,
    models::{AuthPair, Cert, SiteData},
};
use log::*;
use num_bigint::BigUint;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

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
        let index = if let Some(index) = index {
            index
        } else {
            thread_rng().gen_range(0..29639936)
        };
        let privkey = zeronet_cryptography::hd_privkey(seed, index);
        let wif_privkey = zeronet_cryptography::privkey_to_wif(privkey);
        let address = zeronet_cryptography::privkey_to_pubkey(&wif_privkey).unwrap();

        (wif_privkey, address, index)
    }

    /// Creates a new user from a seed
    fn from_seed(master_seed: String) -> User {
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

    fn generate_auth_address(&mut self, address: &str) -> AuthPair {
        let start_time = SystemTime::now();
        let address_id = User::get_address_auth_index(address);
        let (auth_privkey, auth_pubkey, _) =
            self.get_site_keypair_from_seed(&self.master_seed, Some(address_id));
        let auth_pair = AuthPair::new(auth_pubkey, auth_privkey);

        let site_data = SiteData::new(address.to_string())
            .with_auth_pair(auth_pair.clone())
            .with_index(address_id);
        self.sites.insert(address.to_string(), site_data);

        // #[cfg(not(test))]
        // self.save();

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
            // #[cfg(not(test))]
            // self.save();

            debug!("Deleted site: {}", address);
        }
    }

    fn set_site_settings(&mut self, address: &str, settings: serde_json::Value) -> SiteData {
        #[allow(clippy::let_and_return)]
        let site_data = self.get_site_data(address, true).set_settings(settings);
        // #[cfg(not(test))]
        // self.save();
        site_data
    }

    /// Get data for a new, unique site
    ///
    /// Return: [site_address, bip32_index, {"auth_address": "1AddR", "auth_privatekey": "xxx", "privatekey": "xxx"}]
    pub fn get_new_site_data(&mut self) -> SiteData {
        //TODO: Add option to get a new site without master seed
        let (site_privkey, site_address, bip32_idx) =
            self.get_site_keypair_from_seed(&self.master_seed, None);

        if let Some(_site_address) = self.sites.get(&site_address.to_string()) {
            // TODO: do the whole process again instead of warning
            info!("Random error: site exist!");
        }

        let site_data = self
            .get_site_data(&site_address, true)
            .with_index(bip32_idx)
            .with_privkey(site_privkey);

        self.sites
            .insert(site_address.to_string(), site_data.clone());

        // #[cfg(not(test))]
        // self.save();
        site_data
    }
    /// Get BIP32 address from site address
    ///
    /// Return: BIP32 auth address
    fn get_auth_address(&self, addr: &str) -> Option<AuthPair> {
        if let Some(site_data) = self.sites.get(addr) {
            return site_data.get_auth_pair();
        }
        None
    }

    fn get_auth_privkey(&mut self, address: &str, create: bool) -> Option<String> {
        if let Some(cert) = self.get_cert(address) {
            let auth_pair = cert.get_auth_pair();
            Some(auth_pair.get_auth_privkey().to_string())
        } else {
            let site_data = self.get_site_data(address, create);
            // if let Some(auth_pair) = site_data.get_auth_pair() {
            //     Some(auth_pair.get_auth_privkey().to_string())
            // } else {
            //     None
            // }
            site_data
                .get_auth_pair()
                .map(|pair| pair.get_auth_privkey().to_string())
        }
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
            if let Some(auth_pair) = site_data.get_auth_pair() {
                if auth_pair.auth_address == auth_address {
                    return Some(auth_pair);
                }
                return None;
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
            // #[cfg(not(test))]
            // self.save();
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

        if let Some(provider) = provider {
            site_data.add_cert_provider(provider.to_string());
        } else if site_data.get_cert_provider().is_none() {
            site_data.delete_cert_provider();
        }

        // #[cfg(not(test))]
        // self.save();

        site_data
    }

    /// Get cert for the site address
    ///
    /// Return: { "auth_address": "1AddR", "auth_privatekey": "xxx", "auth_type": "web", "auth_user_name": "nofish", "cert_sign": "xxx"} or None
    fn get_cert(&self, provider: &str) -> Option<&Cert> {
        self.certs.get(provider)
    }

    /// Get cert user name for the site address
    ///
    /// Return user@certprovider.bit or None
    fn get_cert_user_id(&self, addr: &str) -> Option<String> {
        if let Some(cert) = &self.get_cert(addr) {
            // return Some(cert.auth_user_name.clone());
            return Some(format!("{}@{}", cert.auth_user_name, cert.auth_type));
        }
        None
    }

    pub fn get_address_auth_index(address: &str) -> u32 {
        let bytes = address.bytes();
        let hexs: Vec<String> = bytes.into_iter().map(|x| format!("{:02X}", x)).collect();
        let hex = &hexs.join("");

        let auth_index = BigUint::parse_bytes(hex.as_bytes(), 16).unwrap();

        // We don't need the whole index
        (auth_index % BigUint::from(100000000u32)).to_u32_digits()[0]
    }

    // pub fn save(&self) {
    //     let start_time = SystemTime::now();
    //     let file_path = ENV.data_path.join("users.json");
    //     let save_user = || -> Result<bool, Error> {
    //         let file = File::open(&file_path)?;
    //         let reader = BufReader::new(file);
    //         let mut users: HashMap<String, serde_json::Value> = serde_json::from_reader(reader)?;
    //         if users.contains_key(&self.master_address) == false {
    //             users.insert(self.master_address.clone(), json!({})); // Create if not exist
    //         }
    //         let user = users.get_mut(&self.master_address).unwrap();
    //         user["master_seed"] = json!(self.master_seed);
    //         user["sites"] = json!(self.sites);
    //         user["certs"] = json!(self.certs);
    //         user["settings"] = json!(self.settings);
    //         let users_file_content_new = serde_json::to_string_pretty(&json!(users))?;
    //         let users_file_bytes = fs::read(&file_path)?;
    //         let result = atomic_write(
    //             &file_path,
    //             users_file_content_new.as_bytes(),
    //             &users_file_bytes,
    //             true,
    //         );
    //         result
    //     };
    //     if let Err(err_msg) = save_user() {
    //         error!("Couldn't save user: {:?}", err_msg);
    //     } else {
    //         debug!(
    //             "Saved in {}s",
    //             SystemTime::now()
    //                 .duration_since(start_time)
    //                 .unwrap()
    //                 .as_secs_f32()
    //         );
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use crate::core::models::AuthPair;

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
        let site_data = user.get_new_site_data();
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

        assert_eq!(true, result);
    }

    #[test]
    fn test_add_cert_auth_not_exist() {
        let mut user = User::from_seed(SEED.to_string());

        let result = user.add_cert(AUTH_ADDR, CERT_DOMAIN, CERT_TYPE, CERT_USERNAME, CERT_SIGN);

        assert_eq!(false, result);
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
