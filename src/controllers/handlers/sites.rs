use actix::{Actor, Addr, Context, Handler, Message};
use futures::executor::block_on;
use log::info;
use serde::{Deserialize, Serialize};
use zerucontent::Content;

use crate::{
    controllers::sites::SitesController,
    core::{address::Address, error::Error, site::Site},
};

impl Actor for Site {
    type Context = Context<Self>;
}

impl Actor for SitesController {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "Result<(Address, Addr<Site>), Error>")]
pub enum Lookup {
    Address(Address),
    Key(String),
}

impl Handler<Lookup> for SitesController {
    type Result = Result<(Address, Addr<Site>), Error>;

    fn handle(&mut self, msg: Lookup, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            Lookup::Address(address) => self.get(address),
            Lookup::Key(s) => self.get_by_key(s),
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct AddWrapperKey {
    address: Address,
    wrapper_key: String,
}

impl AddWrapperKey {
    pub fn new(address: Address, wrapper_key: String) -> AddWrapperKey {
        AddWrapperKey {
            address,
            wrapper_key,
        }
    }
}

impl Handler<AddWrapperKey> for SitesController {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: AddWrapperKey, _ctx: &mut Context<Self>) -> Self::Result {
        // let site = self.get_site(&msg.address.address).unwrap();
        self.nonce
            .insert(msg.wrapper_key.clone(), msg.address.clone());
        let (_, addr) = self.get(msg.address.clone()).unwrap();
        self.sites_addr.insert(msg.address.clone(), addr);
        // let res = block_on(site.send(AddWrapperKey {
        //     address: msg.address,
        //     wrapper_key: msg.wrapper_key,
        // }))??;
        //TODO!: AddWrapperKey to sites.json file
        info!(
            "Added wrapper key {} for {}",
            msg.wrapper_key,
            msg.address.get_address_short()
        );
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "Result<String, Error>")]
pub struct GetWrapperKey {
    pub address: Address,
}

impl Handler<GetWrapperKey> for SitesController {
    type Result = Result<String, Error>;

    fn handle(&mut self, msg: GetWrapperKey, _ctx: &mut Context<Self>) -> Self::Result {
        let nonces = self.nonce.to_owned();
        let s = nonces.iter().find(|(_, a)| {
            let a = a.clone();
            if &msg.address == a {
                return true;
            }
            false
        });
        match s {
            Some((k, _)) => Ok(k.to_owned()),
            None => Err(Error::WrapperKeyNotFound),
        }
    }
}

/// Message struct used to request a file from a site
/// ```
/// match result {
/// 	Ok(true) => println!("File has been downloaded."),
/// 	Ok(false) => println!("File has been queued for download."),
/// 	Err(_) => println!("An error occured!"),
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Default, Clone, Message)]
#[rtype(result = "Result<bool, Error>")]
pub struct FileGetRequest {
    #[serde(default)]
    pub inner_path: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub timeout: f64,
}

impl Handler<FileGetRequest> for Site {
    type Result = Result<bool, Error>;

    fn handle(&mut self, msg: FileGetRequest, _ctx: &mut Context<Self>) -> Self::Result {
        let res = block_on(self.need_file(msg.inner_path, None, None));
        res
    }
}

#[derive(Message)]
#[rtype(result = "Result<Content, Error>")]
pub struct SiteContent(pub Option<String>);

impl Handler<SiteContent> for Site {
    type Result = Result<Content, Error>;

    fn handle(&mut self, msg: SiteContent, _ctx: &mut Context<Self>) -> Self::Result {
        match msg.0 {
            Some(inner_path) => Ok(self.content(Some(&inner_path)).unwrap()),
            None => Ok(self.content(None).unwrap()),
        }
    }
}
