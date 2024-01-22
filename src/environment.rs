use std::{collections::HashMap, env::current_dir, fs, path::PathBuf, str::FromStr};

use clap::{Arg, ArgMatches, Command};
use lazy_static::lazy_static;
use mut_static::MutStatic;
use rand::Rng;
use serde_json::json;

use crate::{
    core::{error::Error, site::models::SiteStorage, user::User},
    io::utils::{load_sites_file, load_trackers, load_users_file},
    plugins::{
        self,
        core::plugin::Plugin,
        path_provider::{self, PathProviderPlugin},
    },
    utils::gen_peer_id,
};

lazy_static! {
    pub static ref PLUGINS: Vec<Plugin> = plugins::utils::load_plugins();
    pub static ref PATH_PROVIDER_PLUGINS: MutStatic<Vec<PathProviderPlugin>> = path_provider::load_plugins();
    pub static ref DEF_ASSETS_PATH: PathBuf = PathBuf::from("assets/");
    pub static ref DEF_PEERS_FILE_PATH: PathBuf = DEF_ASSETS_PATH.join("peers.txt");
    pub static ref DEF_TRACKERS_FILE_PATH: PathBuf = DEF_ASSETS_PATH.join("trackers.txt");
    pub static ref DEF_MEDIA_PATH: PathBuf = DEF_ASSETS_PATH.join("media/");
    pub static ref DEF_TEMPLATES_PATH: PathBuf = DEF_ASSETS_PATH.join("templates/");
    pub static ref CURRENT_DIR: PathBuf = current_dir().unwrap();
    pub static ref DEF_DATA_DIR: String = CURRENT_DIR.join("data").to_str().unwrap().to_string();
    pub static ref DEF_LOG_DIR: String = CURRENT_DIR.join("log").to_str().unwrap().to_string();
    pub static ref USER_STORAGE: HashMap<String, User> = load_users_file();
    pub static ref SITE_STORAGE: HashMap<String, SiteStorage> = load_sites_file();
    pub static ref SITE_PERMISSIONS_DETAILS: HashMap<String, String> = {
        let mut map = HashMap::new();
        map.insert("ADMIN".into(), "Modify your client's configuration and access all site".into());
        map.insert("NOSANDBOX".into(), "Modify your client's configuration and access all site".into());
        map.insert("PushNotification".into(), "Send notifications".into());
        map
    };
    pub static ref MATCHES: ArgMatches = get_matches();
    pub static ref SUB_CMDS: Vec<String> =
        vec![
            "siteCreate".into(),
            "siteNeedFile".into(),
            "siteDownload".into(),
            "siteSign".into(),
            //"sitePublish".into(),
            "siteVerify".into(),
            "siteFileEdit".into(),
            "siteUpdate".into(),
            // "siteCmd".into(),
            "dbRebuild".into(),
            "dbQuery".into(),
            "peerPing".into(),
            // "peerGetFile".into()
            // "peerCmd".into()
            "cryptKeyPair".into(),
            "cryptSign".into(),
            "cryptVerify".into(),
            // "cryptGetPrivateKey".into()
            "getConfig".into(),
            "siteFindPeers".into(),
            "sitePeerExchange".into(),
            "siteFetchChanges".into(),

            "pluginCreate".into(),
            "pluginSign".into(),
            "pluginVerify".into(),
        ];
    pub static ref ENV: Environment = {
        if let Ok(env) = get_env(&MATCHES) {
            return env;
        };
        panic!("Could not get environment variables");
    };
    pub static ref SITE_PEERS_NEED: usize = ENV.site_peers_need;
    pub static ref TRACKERS: Vec<String> = load_trackers();
    pub static ref VERSION: String = String::from("0.8.0");
    pub static ref REV: usize = 4800;
    pub static ref VERSION_WITH_REV: String = format!("{} r{}", &*VERSION, &*REV);
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub version: String,
    pub rev: usize,
    pub peer_id: String,
    pub data_path: PathBuf,
    pub log_path: PathBuf,
    pub fileserver_ip: String,
    pub fileserver_port: u16,
    pub ui_ip: String,
    pub ui_port: u16,
    pub ui_restrict: bool,
    pub ui_host: String,
    pub ui_trans_proxy: bool,
    // pub broadcast_port: usize,
    pub trackers: Vec<String>,
    pub homepage: String,
    pub lang: String,
    pub dist: String,
    pub use_block_storage: bool,
    pub access_key: String,
    pub size_limit: usize,
    pub file_size_limit: usize,
    pub site_peers_need: usize,
    #[cfg(debug_assertions)]
    pub debug: bool,
}

fn get_matches() -> ArgMatches {
    let sub_commands = (*SUB_CMDS)
        .iter()
        .map(|cmd| {
            let app = Command::new(cmd.as_str());
            if cmd.starts_with("peer") {
                app.arg(Arg::new("peer").short('p').required(false).num_args(1))
            } else if cmd.starts_with("plugin") {
                app.arg(Arg::new("name").short('n').required(false).num_args(1))
            } else if cmd.starts_with("cryptSign") || cmd.starts_with("cryptVerify") {
                app.arg(Arg::new("data").short('d').required(true).num_args(1))
            } else {
                app.arg(Arg::new("site").short('s').required(false).num_args(1))
            }
        })
        .collect::<Vec<_>>();
    Command::new("ZeroNetX")
        .version(VERSION.as_str())
        .author("PramUkesh <pramukesh@zeroid.bit>")
        .about("ZeroNet Protocol Implementation in Rust.")
        .args(&[
            //     // Should probably be removed in favor of environment flags
            //     Arg::new("VERBOSE")
            //         .short('v')
            //         .long("verbose")
            //         .help("More detailed logging"),
            //     // Should probably be replaced with arguments dealing particularly with coffeescript compilation and other debug features
            //     Arg::new("DEBUG").long("debug").help("Debug mode"),
            //     // Should probably be removed in favor of environment flags
            //     Arg::new("SILENT")
            //         .long("silent")
            //         .help("Only log errors to terminal"),
            //     // Look up what this does:
            //     Arg::new("DEBUG_SOCKET")
            //         .long("debug_socket")
            //         .help("Debug socket connections"),
            //     Arg::new("MERGE_MEDIA")
            //         .long("merge_media")
            //         .help("Merge all.js and all.css"),
            //     Arg::new("BATCH")
            //         .long("batch")
            //         .help("Batch mode (No interactive input for commands)"),
            //     Arg::new("CONFIG_FILE")
            //         .long("config_file")
            //         .default_value("./zeronet.conf")
            //         .help("Path of config file"),
            Arg::new("DATA_DIR")
                .long("data_dir")
                .default_value(&**DEF_DATA_DIR)
                .help("Path of data directory"),
            Arg::new("LOG_DIR")
                .long("log_dir")
                .default_value(&**DEF_LOG_DIR)
                .help("Path of logging directory"),
            Arg::new("CONSOLE_LOG_LEVEL")
                .long("console_log_level")
                .default_value("debug")
                .help("Level of logging to file"),
            // Arg::new("LOG_LEVEL")
            //     .long("log_level")
            //     .help("Level of logging to file"),
            // Arg::new("LOG_ROTATE")
            //     .long("log_rotate")
            //     .default_value("daily")
            //     .possible_values(&["hourly", "daily", "weekly", "off"])
            //     .help("Log rotate interval"),
            // Arg::new("LOG_ROTATE_BACKUP_COUNT")
            //     .long("log_rotate_backup_count")
            //     .default_value("5")
            //     .help("Log rotate backup count"),
            Arg::new("LANGUAGE")
                .short('l')
                .long("language")
                .default_value("en")
                .help("Web interface language"),
            Arg::new("UI_IP")
                .long("ui_ip")
                .default_value("127.0.0.1")
                .help("Web interface bind address"),
            Arg::new("UI_PORT")
                .long("ui_port")
                .default_value("42110")
                .help("Web interface bind port"),
            Arg::new("UI_RESTRICT")
                .long("ui_restrict")
                .help("Restrict web access"),
            Arg::new("UI_HOST")
                .long("ui_host")
                .help("Allow access using this hosts"),
            Arg::new("UI_TRANS_PROXY")
                .long("ui_trans_proxy")
                .help("Allow access using a transparent proxy"),
            // Arg::new("OPEN_BROWSER")
            //     .long("open_browser")
            //     .help("Open homepage in web browser automatically"),
            Arg::new("HOMEPAGE")
                .long("homepage")
                .default_value("1HELLoE3sFD9569CLCbHEAVqvqV7U2Ri9d")
                .help("Web interface Homepage"),
            Arg::new("ACCESS_KEY")
                .long("access_key")
                .default_value("")
                .help("Access key for Various API calls"),
            Arg::new("DIST_TYPE")
                .long("dist_type")
                .default_value("DecentNet")
                .help("Type of installed distribution"),
            Arg::new("SIZE_LIMIT")
                .long("size_limit")
                .default_value("10")
                .help("Default site size limit in MB"),
            Arg::new("FILE_SIZE_LIMIT")
                .long("file_size_limit")
                .default_value("10")
                .help("Maximum per file size limit"),
            // Arg::new("CONNECTED_LIMIT")
            //     .long("connected_limit")
            //     .default_value("8")
            //     .help("Max connected peer per site"),
            // Arg::new("GLOBAL_CONNECTED_LIMIT")
            //     .long("global_connected_limit")
            //     .default_value("512")
            //     .help("Max connections"),
            Arg::new("FILESERVER_IP")
                .long("fileserver_ip")
                .default_value("*")
                .help("Fileserver bind address"),
            Arg::new("FILESERVER_PORT")
                .long("fileserver_port")
                .default_value("10000-40000")
                .help("Fileserver randomization range 10000-40000"),
            Arg::new("SITE_PEERS_NEED")
                .long("site_peers_need")
                .default_value("1")
                .help("Minimum Peers need for Site communication"),
            // Arg::new("FILESERVER_IP_TYPE")
            //     .long("fileserver_ip_type")
            //     .default_value("dual")
            //     .possible_values(&["ipv4", "ipv6", "dual"])
            //     .help("Fileserver ip type"),
            // Arg::new("IP_LOCAL")
            //     .long("ip_local")
            //     .default_value("['127.0.0.1', '::1']")
            //     .help("My local ips"),
            // Arg::new("IP_EXTERNAL")
            //     .long("ip_external")
            //     .default_value("[]")
            //     .help("Set reported external ip"),
            // Arg::new("TOR_HS_PORT")
            //     .long("tor_hs_port")
            //     .default_value("15441")
            //     .help("Hidden service port in Tor always mode"),
            // Arg::new("BROADCAST_PORT")
            //     .long("broadcast_port")
            //     .default_value("1544")
            //     .help("Port to broadcast local discovery messages"),
            Arg::new("USE_BLOCK_STORAGE")
                .long("use_block_storage")
                .short('b')
                .help("Use Block Storage for Files instead of Normal Site Storage"),
        ])
        .subcommands(sub_commands)
        .get_matches()
}

pub fn get_env(matches: &ArgMatches) -> Result<Environment, Error> {
    let data_path_str = matches.get_one::<String>("DATA_DIR").unwrap();
    let data_path = PathBuf::from_str(data_path_str).unwrap();
    let data_path = if data_path.is_dir() {
        data_path
    } else {
        fs::create_dir_all(data_path_str).unwrap();
        PathBuf::from_str(data_path_str).unwrap()
    };
    let log_path_str = matches.get_one::<String>("LOG_DIR").unwrap();
    let log_path = PathBuf::from_str(log_path_str).unwrap();
    let log_path = if log_path.is_dir() {
        log_path
    } else {
        fs::create_dir_all(log_path_str).unwrap();
        PathBuf::from_str(log_path_str).unwrap()
    };
    let fileserver_ip = if let Some(ip) = matches.get_one::<String>("FILESERVER_IP") {
        if ip == "*" {
            "127.0.0.1".into()
        } else {
            ip.into()
        }
    } else {
        unreachable!()
    };
    let fileserver_port = if let Some(port) = matches.get_one::<String>("FILESERVER_PORT") {
        if port.contains("10000-40000") {
            let mut rng = rand::thread_rng();
            rng.gen_range(10000..=40000)
        } else {
            port.parse::<u16>().unwrap()
        }
    } else {
        10000 + rand::random::<u16>() % 10000
    };
    let use_block_storage = matches.get_one::<bool>("USE_BLOCK_STORAGE").is_some();
    let ui_ip = matches.get_one::<String>("UI_IP").unwrap();
    let ui_port: u16 = matches.get_one::<String>("UI_PORT").unwrap().parse()?;
    let ui_host = matches
        .get_one::<String>("UI_HOST")
        .unwrap_or(&String::default())
        .to_owned();
    let ui_trans_proxy = matches.get_one::<bool>("UI_TRANS_PROXY").is_some();
    let ui_restrict = matches.get_one::<bool>("UI_RESTRICT").is_some();
    let log_level = matches.get_one::<String>("CONSOLE_LOG_LEVEL").unwrap();
    // let broadcast_port: usize = matches.value_of("BROADCAST_PORT").unwrap().parse()?;

    //TODO! Replace with file based logger with public release.
    std::env::set_var("DECENTNET_LOG", format!("zeronet={}", log_level));
    pretty_env_logger::init_custom_env("DECENTNET_LOG");

    let env = Environment {
        version: VERSION.clone(),
        rev: *REV,
        peer_id: gen_peer_id(),
        data_path,
        log_path,
        fileserver_ip,
        fileserver_port,
        ui_ip: String::from(ui_ip),
        ui_port,
        ui_host,
        ui_trans_proxy,
        ui_restrict,
        trackers: (*TRACKERS).iter().map(String::from).collect(),
        homepage: String::from(matches.get_one::<String>("HOMEPAGE").unwrap()),
        lang: String::from(matches.get_one::<String>("LANGUAGE").unwrap()),
        dist: String::from(matches.get_one::<String>("DIST_TYPE").unwrap()),
        use_block_storage,
        access_key: String::from(matches.get_one::<String>("ACCESS_KEY").unwrap()),
        size_limit: matches.get_one::<String>("SIZE_LIMIT").unwrap().parse()?,
        file_size_limit: matches
            .get_one::<String>("FILE_SIZE_LIMIT")
            .unwrap()
            .parse()?,
        site_peers_need: matches
            .get_one::<String>("SITE_PEERS_NEED")
            .unwrap()
            .parse()?,
        #[cfg(debug_assertions)]
        debug: true,
    };
    Ok(env)
}

pub fn client_info() -> serde_json::Value {
    let os = if cfg!(windows) {
        "windows"
    } else if cfg!(unix) {
        "unix"
    } else if cfg!(macos) {
        "macos"
    } else if cfg!(android) {
        "android"
    } else if cfg!(ios) {
        "ios"
    } else {
        "unrecognised"
    };
    json!({
        "platform": os,
        "fileserver_ip": *ENV.fileserver_ip,
        "fileserver_port": ENV.fileserver_port,
        "version": *VERSION,
        "rev": *REV,
        "language": *ENV.lang,
        "debug": false,
        "log_dir":*ENV.log_path,
        "data_dir": *ENV.data_path,
        "plugins" : [
            "Placeholder Data",
        ],
    })
}
