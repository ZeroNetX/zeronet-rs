use std::collections::HashMap;

use actix::{Actor, Context, Handler, Message};
use serde_json::Value;

use crate::{
    controllers::users::UserController,
    core::{
        address::Address,
        user::{models::SiteData, User},
    },
};

impl Actor for UserController {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "Option<User>")]
pub struct UserRequest {
    pub address: String,
}

impl Handler<UserRequest> for UserController {
    type Result = Option<User>;

    fn handle(&mut self, msg: UserRequest, _: &mut Self::Context) -> Self::Result {
        match msg.address.as_str() {
            "current" => Some(self.current()),
            address => self.get_user(address),
        }
    }
}

#[derive(Message, Default)]
#[rtype(result = "Option<HashMap<String, Value>>")]
pub struct UserSettings {
    pub set: bool,
    pub global: bool,
    pub user_addr: String,
    pub site_addr: String,
    pub settings: Option<HashMap<Address, HashMap<String, serde_json::Value>>>,
}

impl Handler<UserSettings> for UserController {
    type Result = Option<HashMap<String, Value>>;

    fn handle(&mut self, msg: UserSettings, _ctx: &mut Self::Context) -> Self::Result {
        let user = match msg.user_addr.as_str() {
            "current" => {
                let user_addr = self.current().master_address;
                self.get_user_mut(&user_addr)
            }
            _ => self.get_user_mut(&msg.user_addr),
        };
        if let Some(user) = user {
            if msg.global {
                let setting = msg.settings.unwrap();
                user.settings = setting.values().next().unwrap().clone();
                None
            } else if msg.set {
                let site_addr = msg.settings.clone().unwrap().keys().next().unwrap().clone();
                user.settings = msg.settings.unwrap().get(&site_addr).unwrap().clone();
                None
            } else {
                Some(user.settings.clone())
            }
        } else {
            None
        }
    }
}

#[derive(Message)]
#[rtype(result = "Option<SiteData>")]
pub struct UserSiteData {
    pub user_addr: String,
    pub site_addr: String,
}

impl Handler<UserSiteData> for UserController {
    type Result = Option<SiteData>;

    fn handle(&mut self, msg: UserSiteData, _: &mut Self::Context) -> Self::Result {
        let user = match msg.user_addr.as_str() {
            "current" => Some(self.current_mut()),
            _ => self.get_user_mut(&msg.user_addr),
        };
        if let Some(user) = user {
            Some(user.get_site_data(&msg.site_addr, true))
        } else {
            None
        }
    }
}
