use std::collections::HashMap;

use actix::{Actor, Context, Handler, Message};

use crate::{
    controllers::users::UserController,
    core::{address::Address, user::User},
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
        self.get_user(&msg.address)
    }
}

#[derive(Message, Default)]
#[rtype(result = "Option<User>")]
pub struct UserSettings {
    pub set: bool,
    pub address: String,
    pub settings: Option<HashMap<Address, HashMap<String, serde_json::Value>>>,
}

impl Handler<UserSettings> for UserController {
    type Result = Option<User>;

    fn handle(&mut self, msg: UserSettings, _ctx: &mut Self::Context) -> Self::Result {
        if msg.set {
            if let Some(user) = self.get_users_mut().get_mut(&msg.address) {
                let site_addr = msg.settings.clone().unwrap().keys().next().unwrap().clone();
                user.settings = msg.settings.unwrap().get(&site_addr).unwrap().clone();
                Some(user.clone())
            } else {
                None
            }
        } else {
            let addr = match (&msg.address).len() {
                0 => self.current().master_address.clone(),
                _ => msg.address,
            };
            if let Some(user) = self.get_user(&addr) {
                Some(user)
            } else {
                None
            }
        }
    }
}
