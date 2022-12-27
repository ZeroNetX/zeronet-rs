use actix::{Actor, Addr, Context, Handler, Message, ResponseActFuture};
use bitcoin::hashes::hex::ToHex;
use futures::{executor::block_on, future::join_all, FutureExt};
use log::info;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use zerucontent::Content;

use crate::{
    controllers::sites::SitesController,
    core::{
        address::Address,
        error::Error,
        site::{models::SiteInfo, Site},
    },
    environment::ENV,
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
#[rtype(result = "Result<(String, String), Error>")]
pub struct GetSiteKeys {
    pub address: Address,
}

impl Handler<GetSiteKeys> for SitesController {
    type Result = Result<(String, String), Error>;

    fn handle(&mut self, msg: GetSiteKeys, _ctx: &mut Context<Self>) -> Self::Result {
        let ajax_key = self.ajax_keys.iter().find(|(_, a)| &msg.address == *a);
        let nonce = self.nonce.iter().find(|(_, a)| &msg.address == *a);
        match (nonce, ajax_key) {
            (Some((n, _)), Some((a, _))) => Ok((n.to_owned(), a.to_owned())),
            _ => Err(Error::WrapperKeyNotFound),
        }
    }
}

/// Message struct used to request a file from a site
/// ```
/// match result {
///     Ok(true) => println!("File has been downloaded."),
///     Ok(false) => println!("File has been queued for download."),
///     Err(_) => println!("An error occurred!"),
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

#[derive(Serialize, Deserialize, Debug, Default, Clone, Message)]
#[rtype(result = "Option<Value>")]
pub struct FileRulesRequest {
    #[serde(default)]
    pub inner_path: String,
}

impl Handler<FileRulesRequest> for Site {
    type Result = Option<Value>;

    fn handle(&mut self, msg: FileRulesRequest, _ctx: &mut Context<Self>) -> Self::Result {
        self.get_file_rules(&msg.inner_path)
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
            None => match self.content(None) {
                Some(content) => Ok(content),
                None => Err(Error::MissingError),
            },
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<SiteInfo, Error>")]
pub struct SiteInfoRequest();

impl Handler<SiteInfoRequest> for Site {
    type Result = Result<SiteInfo, Error>;

    fn handle(&mut self, _: SiteInfoRequest, _ctx: &mut Context<Self>) -> Self::Result {
        // TODO: replace default values
        if !self.content_exists() {
            let _ = self.load_content();
        }
        let mut content = self.content(None).unwrap_or_default().raw();
        if let Value::Object(map) = &mut content {
            for key in &["sign", "signs", "signers_sign"] {
                if map.contains_key(*key) {
                    map.remove(*key);
                }
            }
            for key in &["files", "files_optional", "includes"] {
                if map.contains_key(*key) {
                    map[*key] = match &map[*key] {
                        Value::Object(content) => Value::Number(Number::from(content.len())),
                        _ => Value::Number(Number::from(0)),
                    };
                }
            }
        }
        let size_limit = self.storage.settings.size_limit;
        let size_limit = if size_limit == 0 {
            ENV.size_limit
        } else {
            size_limit
        };
        let next_size_limit = size_limit; //TODO!: Need to modify next_size_limit
        let mut storage = self.storage.clone();
        storage.settings.size_limit = size_limit;

        Ok(SiteInfo {
            auth_address: String::from(""),
            address_hash: self.addr().get_address_hash().to_hex(),
            cert_user_id: None,
            address: self.address(),
            address_short: self.addr().get_address_short(),
            settings: storage.clone(),
            content_updated: 0f64,
            bad_files: storage.cache.bad_files.len(),
            size_limit,
            next_size_limit,
            peers: self.peers.len() + 1,
            started_task_num: 0,
            tasks: 0,
            workers: 0,
            content,
            privatekey: false,
        })
    }
}

#[derive(Message)]
#[rtype(result = "Result<Vec<SiteInfo>, Error>")]
pub struct SiteInfoListRequest {
    pub connecting: bool,
}

impl Handler<SiteInfoListRequest> for SitesController {
    type Result = ResponseActFuture<Self, Result<Vec<SiteInfo>, Error>>;

    fn handle(&mut self, _msg: SiteInfoListRequest, _ctx: &mut Context<Self>) -> Self::Result {
        let requests: Vec<_> = self
            .sites_addr
            .par_iter()
            .map(|(_, addr)| addr.send(SiteInfoRequest()))
            .collect();
        let request = join_all(requests).map(|r| {
            Ok(r.into_iter()
                .filter_map(|x| match x {
                    Ok(Ok(a)) => Some(a),
                    _ => None,
                })
                .collect())
        });
        let wrapped = actix::fut::wrap_future::<_, Self>(request);
        Box::pin(wrapped)
    }
}

#[derive(Message)]
#[rtype(result = "Result<Vec<Map<String, Value>>, Error>")]
pub struct DBQueryRequest {
    pub address: String,
    pub query: String,
}

impl Handler<DBQueryRequest> for SitesController {
    type Result = Result<Vec<Map<String, Value>>, Error>;

    fn handle(&mut self, msg: DBQueryRequest, _ctx: &mut Context<Self>) -> Self::Result {
        let conn = self.db_manager.get_db(&msg.address).unwrap();
        block_on(Self::db_query(conn, &msg.query))
    }
}
