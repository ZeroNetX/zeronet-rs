use super::{address::Address as Addr, error::Error, models::SiteSettings, peer::Peer};
use chrono::Utc;
use log::error;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    str::FromStr,
};
use zerucontent::Content;

pub struct Site {
    address: Addr,
    pub peers: HashMap<String, Peer>,
    pub settings: SiteSettings,
    pub data_path: PathBuf,
    content: Option<Content>,
}

impl Site {
    pub fn new(address: &str, data_path: PathBuf) -> Result<Self, Error> {
        let mut settings = SiteSettings::default();
        Ok(Self {
            address: Addr::from_str(address)?,
            peers: HashMap::new(),
            data_path,
            settings,
            content: None,
        })
    }

    pub fn address(&self) -> String {
        self.address.address.clone()
    }

    fn content_exists(&self) -> bool {
        self.content.is_some()
    }

    pub fn content(&self) -> Option<Content> {
        self.content.clone()
    }

    pub fn modify_content(&mut self, content: Content) {
        self.content = Some(content);
    }

    pub async fn verify_content(&self) -> bool {
        if self.content.is_none() {
            false
        } else {
            let content = self.content.clone().unwrap();
            let verified = content.verify((&self.address()).clone());
            if !verified {
                error!("Content verification failed for {}", self.address());
            }
            //TODO! Return the result of the verification
            true
        }
    }

    // pub async fn download_content(&self, inner_path: String) -> bool {
    //     println!(
    //         "Downloading Site : {}'s Content : {}",
    //         &self.address, &inner_path,
    //     );
    //     //36744 11917
    //     let address = PeerAddr::parse("127.0.0.1:11917".to_string()).unwrap();
    //     let mut connection = ZeroConnection::from_address(address).unwrap();
    //     let body = templates::GetFile {
    //         site: self.address.to_string(),
    //         inner_path: "content.json".to_string(),
    //         location: 0,
    //         file_size: Default::default(),
    //     };
    //     // let message = ZeroMessage::request("getFile", connection.next_req_id, body);
    //     // let result = connection.connection.request(message).await.unwrap();
    //     // // match result {
    //     // //     ZeroMessage::Request(req) => req.body(),
    //     // //     ZeroMessage::Response(res) => res.body(),
    //     // // };
    //     // let bod = result.body::<templates::GetFileResponse>().unwrap();
    //     // // let bod = result;
    //     // let s = String::from_utf8_lossy(&bod.body);
    //     // println!("{}", s);
    //     // let path = "data/".to_string() + &self.address.address;
    //     // let exists = Path::new(&path).exists();
    //     // if exists {
    //     // } else {
    //     //     fs::create_dir_all(&path).unwrap();
    //     // }
    //     // fs::write(path.to_owned() + &"/content.json".to_string(), &bod.body).unwrap();
    //     true
    // }
}
