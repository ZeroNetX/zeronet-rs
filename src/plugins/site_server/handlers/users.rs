use std::collections::HashMap;

use actix::{Actor, Context, Handler, Message};
use futures::executor::block_on;
use serde::Deserialize;
use serde_json::Value;

use crate::{
    controllers::users::UserController,
    core::{
        address::Address,
        error::Error,
        io::UserIO,
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

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct UserSetSiteCertRequest {
    pub user_addr: String,
    pub site_addr: String,
    pub provider: String,
}

impl Handler<UserSetSiteCertRequest> for UserController {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: UserSetSiteCertRequest, _: &mut Self::Context) -> Self::Result {
        let user = match msg.user_addr.as_str() {
            "current" => Some(self.current_mut()),
            address => self.get_user_mut(address),
        };
        if let Some(user) = user {
            user.set_cert(&msg.site_addr, Some(&msg.provider));
            Ok(())
        } else {
            Err(Error::UserNotFound)
        }
    }
}

#[derive(Message, Deserialize)]
#[rtype(result = "Result<bool, Error>")]
pub struct UserCertAddRequest {
    #[serde(default)]
    pub user_addr: String,
    pub domain: String,
    pub auth_type: String,
    pub auth_user_name: String,
    pub cert: String,
}

impl Handler<UserCertAddRequest> for UserController {
    type Result = Result<bool, Error>;

    fn handle(&mut self, msg: UserCertAddRequest, _: &mut Self::Context) -> Self::Result {
        let user = match msg.user_addr.as_str() {
            "current" => Some(self.current_mut()),
            address => self.get_user_mut(address),
        };
        if let Some(user) = user {
            user.add_cert(
                &msg.domain,
                &msg.auth_type,
                &msg.auth_user_name,
                &msg.cert,
                &msg.cert,
            )
        } else {
            Err(Error::UserNotFound)
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
                let _ = block_on(user.save());
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
#[rtype(result = "Option<HashMap<String, SiteData>>")]
pub struct UserSiteData {
    pub user_addr: String,
    pub site_addr: String,
}

impl Handler<UserSiteData> for UserController {
    type Result = Option<HashMap<String, SiteData>>;

    fn handle(&mut self, msg: UserSiteData, _: &mut Self::Context) -> Self::Result {
        let user = match msg.user_addr.as_str() {
            "current" => Some(self.current_mut()),
            _ => self.get_user_mut(&msg.user_addr),
        };
        if let Some(user) = user {
            match msg.site_addr.as_str() {
                "all" => {
                    let map = user
                        .sites
                        .clone()
                        .into_iter()
                        .map(|(addr, site_data)| (addr, site_data))
                        .collect::<HashMap<String, SiteData>>();
                    Some(map)
                }
                addr => {
                    let mut map = HashMap::<String, SiteData>::with_capacity(1);
                    map.insert(addr.into(), user.get_site_data(addr, true));
                    Some(map)
                }
            }
        } else {
            None
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct UserSiteDataDeleteRequest {
    pub user_addr: String,
    pub site_addr: String,
}

impl Handler<UserSiteDataDeleteRequest> for UserController {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: UserSiteDataDeleteRequest, _: &mut Self::Context) -> Self::Result {
        let user = match msg.user_addr.as_str() {
            "current" => Some(self.current_mut()),
            _ => self.get_user_mut(&msg.user_addr),
        };
        if let Some(user) = user {
            user.delete_site_data(&msg.site_addr);
            Ok(())
        } else {
            Err(Error::UserNotFound)
        }
    }
}
