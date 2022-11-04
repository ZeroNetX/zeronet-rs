use crate::{core::site::Site, plugins::site_server::common::get_nonce};

pub mod sites;
pub mod users;

impl Site {
    pub fn get_wrapper_key(&mut self) -> String {
        if self.storage.keys.wrapper_key.is_empty() {
            let key = get_nonce(false, 64);
            self.storage.keys.wrapper_key = key.clone();
            key
        } else {
            self.storage.keys.wrapper_key.clone()
        }
    }

    pub fn get_ajax_key(&mut self) -> String {
        if self.storage.keys.ajax_key.is_empty() {
            let key = get_nonce(false, 64);
            self.storage.keys.ajax_key = key.clone();
            key
        } else {
            self.storage.keys.ajax_key.clone()
        }
    }
}
