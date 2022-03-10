use clap::{Arg, ArgMatches, Command};
use lazy_static::lazy_static;
use std::{env::current_dir, fs, path::PathBuf, str::FromStr};

use crate::{core::error::Error, utils::gen_peer_id};

lazy_static! {
    pub static ref CURRENT_DIR: PathBuf = current_dir().unwrap();
    pub static ref DEF_DATA_DIR: String = CURRENT_DIR.join("data").to_str().unwrap().to_string();
    pub static ref MATCHES: ArgMatches = get_matches();
    pub static ref SUB_CMDS: Vec<String> =
        vec![
            "siteDownload".into(),
            "siteFindPeers".into(),
            // "siteNeedFile".into(),
            "siteVerify".into(),
            "sitePeerExchange".into(),
            "siteFetchChanges".into(),
            "dbRebuild".into(),
            // "dbQuery".into()
        ];
    pub static ref ENV: Environment = {
        if let Ok(env) = get_env(&*MATCHES) {
            return env;
        };
        panic!("Could not get environment variables");
    };
    pub static ref VERSION: String = String::from("0.8.0");
    pub static ref REV: usize = 4800;
}

const TRACKERS: &[&str] = &[
    "udp://abufinzio.monocul.us:6969/announce",
    "udp://tracker.0x.tf:6969/announce",
    "udp://tracker.zerobytes.xyz:1337/announce",
    "udp://vibe.sleepyinternetfun.xyz:1738/announce",
    "udp://www.torrent.eu.org:451/announce",
];

#[derive(Debug, Clone)]
pub struct Environment {
    pub version: String,
    pub rev: usize,
    pub peer_id: String,
    pub data_path: PathBuf,
    // pub broadcast_port: usize,
    // pub ui_ip: String,
    // pub ui_port: usize,
    pub trackers: Vec<String>,
    // pub homepage: String,
    // pub lang: String,
    // pub dist: String,
}

fn get_matches() -> ArgMatches {
    let sub_commands = (&*SUB_CMDS)
        .iter()
        .map(|cmd| Command::new(cmd))
        .collect::<Vec<_>>();

    Command::new("zeronet")
        .version((*VERSION).as_str())
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
                .default_value(&DEF_DATA_DIR)
                .help("Path of data directory"),
            // Should be removed
            // Arg::new("CONSOLE_LOG_LEVEL")
            //     .long("console_log_level")
            //     .help("Level of logging to file"),
            // Arg::new("LOG_DIR")
            //     .long("log_dir")
            //     .default_value("./log")
            //     .help("Path of logging directory"),
            // Arg::new("LOG_LEVEL")
            //     .long("log_level")
            //     .help("Level of loggin to file"),
            // Arg::new("LOG_ROTATE")
            //     .long("log_rotate")
            //     .default_value("daily")
            //     .possible_values(&["hourly", "daily", "weekly", "off"])
            //     .help("Log rotate interval"),
            // Arg::new("LOG_ROTATE_BACKUP_COUNT")
            //     .long("log_rotate_backup_count")
            //     .default_value("5")
            //     .help("Log rotate backup count"),
            // Arg::new("LANGUAGE")
            //     .short('l')
            //     .long("language")
            //     .default_value("en")
            //     .help("Web interface language"),
            // Arg::new("UI_IP")
            //     .long("ui_ip")
            //     .default_value("127.0.0.1")
            //     .help("Web interface bind address"),
            // Arg::new("UI_PORT")
            //     .long("ui_port")
            //     .default_value("43110")
            //     .help("Web interface bind port"),
            // Arg::new("UI_RESTRICT")
            //     .long("ui_restrict")
            //     .help("Restrict web access"),
            // Arg::new("UI_HOST")
            //     .long("ui_host")
            //     .help("Allow access using this hosts"),
            // Arg::new("UI_TRANS_PROXY")
            //     .long("ui_trans_proxy")
            //     .help("Allow access using a transparent proxy"),
            // Arg::new("OPEN_BROWSER")
            //     .long("open_browser")
            //     .help("Open homepage in web browser automatically"),
            // Arg::new("HOMEPAGE")
            //     .long("homepage")
            //     .default_value("/1HELLoE3sFD9569CLCbHEAVqvqV7U2Ri9d")
            //     .help("Web interface Homepage"),
            // // UPDATE SITE?
            // Arg::new("DIST_TYPE")
            //     .long("dist_type")
            //     .default_value("source")
            //     .help("Type of installed distribution"),
            // Arg::new("SIZE_LIMIT")
            //     .long("size_limit")
            //     .default_value("10")
            //     .help("Default site size limit in MB"),
            // Arg::new("FILE_SIZE_LIMIT")
            //     .long("file_size_limit")
            //     .default_value("10")
            //     .help("Maximum per file size limit"),
            // Arg::new("CONNECTED_LIMIT")
            //     .long("connected_limit")
            //     .default_value("8")
            //     .help("Max connected peer per site"),
            // Arg::new("GLOBAL_CONNECTED_LIMIT")
            //     .long("global_connected_limit")
            //     .default_value("512")
            //     .help("Max connections"),
            // Arg::new("FILESERVER_IP")
            //     .long("fileserver_ip")
            //     .default_value("*")
            //     .help("Fileserver bind address"),
            // Arg::new("FILESERVER_PORT_RANGE")
            //     .long("fileserver_port_range")
            //     .default_value("10000-40000")
            //     .help("Fileserver randomization range"),
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
        ])
        .subcommands(sub_commands)
        .get_matches()
}

pub fn get_env(matches: &ArgMatches) -> Result<Environment, Error> {
    let data_path_str = matches.value_of("DATA_DIR").unwrap();
    let data_path = PathBuf::from_str(data_path_str).unwrap();
    let data_path = if data_path.exists() && data_path.is_dir() {
        data_path
    } else {
        fs::create_dir_all(data_path_str).unwrap();
        PathBuf::from_str(data_path_str).unwrap()
    };
    // let ui_ip = matches.value_of("UI_IP").unwrap();
    // let ui_port: usize = matches.value_of("UI_PORT").unwrap().parse()?;
    // let broadcast_port: usize = matches.value_of("BROADCAST_PORT").unwrap().parse()?;
    let env = Environment {
        version: VERSION.clone(),
        rev: *REV,
        peer_id: gen_peer_id(),
        data_path,
        // broadcast_port,
        // ui_ip: String::from(ui_ip),
        // ui_port,
        trackers: TRACKERS.iter().map(|s| String::from(*s)).collect(),
        // homepage: String::from(matches.value_of("HOMEPAGE").unwrap()),
        // lang: String::from(matches.value_of("LANGUAGE").unwrap()),
        // dist: String::from(matches.value_of("DIST_TYPE").unwrap()),
    };
    Ok(env)
}
