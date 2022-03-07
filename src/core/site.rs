use super::{address::Address as Addr, error::Error, models::SiteSettings, peer::Peer};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    str::FromStr,
};

pub struct Site {
    pub address: Addr,
    peers: HashMap<String, Peer>,
    pub settings: SiteSettings,
    data_path: PathBuf,
}

pub trait SiteIO {
    fn load_settings(address: &str) -> Result<SiteSettings, Error>;
    fn save_settings(&self) -> Result<(), Error>;
}

impl Site {
    /// Create a new site with def settings
    pub fn new(address: &str, data_path: PathBuf) -> Result<Self, Error> {
        let mut settings = SiteSettings::default();
        Ok(Self {
            address: Addr::from_str(address)?,
            peers: HashMap::new(),
            data_path,
            settings,
        })
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
