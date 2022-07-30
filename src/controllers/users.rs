use std::collections::HashMap;

use actix::{Actor, Addr};
use log::info;

use crate::{
    core::{error::Error, user::User},
    environment::USER_STORAGE,
};

pub fn run() -> Result<Addr<UserController>, Error> {
    info!("Starting User Controller.");
    let users = (*USER_STORAGE).clone();
    let user_controller = UserController::new(users);
    let user_controller_addr = user_controller.start();
    Ok(user_controller_addr)
}

pub struct UserController {
    users: HashMap<String, User>,
}

impl UserController {
    pub fn new(users: HashMap<String, User>) -> Self {
        Self { users }
    }

    pub fn current(&self) -> User {
        self.users.values().next().unwrap().clone()
    }

    pub fn create_user(&mut self) -> User {
        let user = User::new();
        self.add_user(user.clone());
        user
    }

    pub fn add_user(&mut self, user: User) {
        self.users.insert(user.master_address.clone(), user);
    }

    pub fn get_user(&self, master_address: &str) -> Option<User> {
        self.users.get(master_address).cloned()
    }

    pub fn get_user_mut(&mut self, master_address: &str) -> Option<&mut User> {
        self.users.get_mut(master_address)
    }

    pub fn get_users(&self) -> &HashMap<String, User> {
        &self.users
    }

    pub fn get_users_mut(&mut self) -> &mut HashMap<String, User> {
        &mut self.users
    }

    pub fn remove_user(&mut self, master_address: &str) {
        self.users.remove(master_address);
    }
}
