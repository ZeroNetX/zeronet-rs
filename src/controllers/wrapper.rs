use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
};

use actix_files::NamedFile;
use actix_web::{HttpRequest, HttpResponse, Responder, Result};
use log::*;
use uuid::Uuid;
use zerucontent::Content;

use crate::{
    controllers::handlers::{
        sites::{AddWrapperKey, Lookup, SiteContent},
        users::UserSettings,
    },
    core::{address::Address, error::Error},
    environment::ENV,
};

use super::server::ZeroServer;

struct WrapperData {
    file_url: String,
    file_inner_path: String,
    address: String,
    title: String,
    body_style: String,
    meta_tags: String,
    query_string: String,
    wrapper_key: String,
    ajax_key: String,
    wrapper_nonce: String,
    postmessage_nonce_security: String,
    permissions: String,
    show_loadingscreen: String,
    sandbox_permissions: String,
    rev: String,
    lang: String,
    homepage: String,
    themeclass: String,
    script_nonce: String,
}

pub async fn serve_wrapper(
    req: HttpRequest,
    data: actix_web::web::Data<ZeroServer>,
) -> HttpResponse {
    let nonce = Uuid::new_v4().simple().to_string();
    {
        let mut nonces = data.wrapper_nonces.lock().unwrap();
        nonces.insert(nonce.clone());
        trace!("Valid nonces ({}): {:?}", nonces.len(), nonces);
    }

    let address_string = req.match_info().query("address");
    let address = match Address::from_str(address_string) {
        Ok(a) => a,
        Err(_) => {
            return HttpResponse::Ok()
                .body(format!("{} is a malformed ZeroNet address", address_string));
        }
    };
    let inner_path = req.match_info().query("inner_path");
    info!(
        "Serving wrapper for zero://{}/{}",
        address.get_address_short(),
        inner_path
    );

    let result = data
        .site_controller
        .send(AddWrapperKey::new(address.clone(), nonce.clone()));

    if result.await.is_err() {
        error!("Error sending wrapper key to site manager");
    }
    let site_controller = data.site_controller.clone();
    let query = match site_controller.send(Lookup::Address(address.clone())).await {
        Ok(v) => v,
        Err(err) => {
            error!("{:?}", err);
            return HttpResponse::BadRequest().finish();
        }
    };
    let (_, site) = query.expect("MailBox Closed");
    let show_loadingscreen;
    let content = site.send(SiteContent(None)).await;
    let title;
    let content = if let Ok(Ok(content)) = content {
        show_loadingscreen = String::from("false");
        title = content.title.to_string();
        content
    } else {
        show_loadingscreen = String::from("true");
        title = format!("Loading {}...", address.address);
        Content::default()
    };
    let mut meta_tags = String::new();
    if !content.viewport.is_empty() {
        let mut meta = String::new();
        html_escape::encode_text_to_string(content.viewport, &mut meta);
        meta_tags.push_str(&format!(
            "<meta name=\"viewport\" id=\"viewport\" content=\"{}\">",
            meta
        ));
    }
    if !content.favicon.is_empty() {
        let mut meta = String::new();
        meta.push_str(&format!("/{}/", address_string));
        html_escape::encode_text_to_string(content.favicon, &mut meta);
        meta_tags.push_str(&format!("<link rel=\"icon\" href=\"{}\">", meta));
    }
    let user_settings = data
        .user_controller
        .send(UserSettings {
            user_addr: String::from("current"),
            site_addr: address_string.into(),
            ..Default::default()
        })
        .await
        .unwrap()
        .unwrap(); //TODO: handle error
    let theme = if let Some(theme) = user_settings.get("theme") {
        match theme.as_str() {
            Some("dark") => "dark",
            _ => "light",
        }
    } else {
        "light"
    };
    let themeclass = format!("theme-{}", theme);

    let mut body_style = String::new();
    if !content.background_color.is_empty() {
        let mut meta = String::new();
        let theme_str = match theme {
            "dark" => content.background_color_dark,
            _ => content.background_color,
        };
        html_escape::encode_text_to_string(theme_str, &mut meta);
        body_style.push_str(&format!("background-color: {};", meta));
    }

    let postmessage_nonce_security = format!("{}", content.postmessage_nonce_security);

    let path = PathBuf::from("./ui/templates/wrapper.html");

    let sandbox_permissions = "".into();

    let string = match render(
        &path,
        WrapperData {
            file_url: format!("\\/{}\\/{}", address, inner_path),
            file_inner_path: String::from(inner_path),
            address: address.to_string(),
            title,
            body_style,
            meta_tags,
            query_string: format!("\\?wrapper_nonce\\={}", nonce.clone()),
            wrapper_key: nonce.clone(),
            ajax_key: String::from("ajax_key"), //TODO!: Need to Replace with real value
            wrapper_nonce: nonce.clone(),
            postmessage_nonce_security,
            permissions: String::from("[]"), //TODO!: Need to Replace with permissions from site settings
            show_loadingscreen,              //TODO! Handle this when websockets are implemented
            sandbox_permissions,
            rev: format!("{}", ENV.rev),
            lang: ENV.lang.to_string(),
            homepage: String::from(&*ENV.homepage),
            themeclass,
            script_nonce: String::from("script_nonce"), //TODO!: Need to Replace with real value
        },
    ) {
        Ok(s) => s,
        Err(_) => String::new(),
    };
    HttpResponse::Ok()
        .content_type("html")
        .append_header(("X-Hdr", "sample")) //TODO!: Use Header value from ZeroNet impl
        .body(string)
}

fn render(file_path: &Path, data: WrapperData) -> Result<String, ()> {
    let mut file = match File::open(file_path) {
        Ok(f) => f,
        Err(error) => {
            error!("Failed to Get Wrapper Template file: {:?}", error);
            return Result::Err(());
        }
    };
    let mut string = String::new();
    match file.read_to_string(&mut string) {
        Ok(_) => {}
        Err(_) => return Err(()),
    };
    let enable_web_socket = true; //TODO!: Add env var to control this
    if enable_web_socket {
        string = string.replace("\n{websocket_scripts}", concat!(
            "\n<script type=\"text/javascript\" src=\"/uimedia/all.js?rev={rev}&lang={lang}\" nonce=\"{script_nonce}\"></script>",
            "\n<script nonce=\"{script_nonce}\">setTimeout(window.wrapper.onWrapperLoad, 1)</script>"));
    } else {
        string = string.replace("\n{websocket_scripts}", "");
    }
    let server_url = format!("{}:{}", &*ENV.ui_ip, &ENV.ui_port);

    string = string.replace("{title}", &data.title);
    string = string.replace("{rev}", &data.rev);
    string = string.replace("{meta_tags}", &data.meta_tags);

    string = string.replace("{body_style}", &data.body_style);
    string = string.replace("{themeclass}", &data.themeclass);

    string = string.replace("{script_nonce}", &data.script_nonce);

    string = string.replace("{homepage}", &data.homepage);

    string = string.replace("{sandbox_permissions}", &data.sandbox_permissions);

    string = string.replace("{file_url}", &data.file_url);
    string = string.replace("{query_string}", &data.query_string);
    string = string.replace("{address}", &data.address);
    string = string.replace("{wrapper_nonce}", &data.wrapper_nonce);
    string = string.replace("{wrapper_key}", &data.wrapper_key);
    string = string.replace("{ajax_key}", &data.ajax_key);
    string = string.replace(
        "{postmessage_nonce_security}",
        &data.postmessage_nonce_security,
    );
    string = string.replace("{file_inner_path}", &data.file_inner_path);
    string = string.replace("{permissions}", &data.permissions);
    string = string.replace("{show_loadingscreen}", &data.show_loadingscreen);
    string = string.replace("{server_url}", server_url.as_str());

    // string = string.replace("{inner_path}", &data.inner_path);
    string = string.replace("{lang}", &data.lang);
    Ok(string)
}

pub async fn serve_uimedia(req: HttpRequest) -> HttpResponse {
    let inner_path = req.match_info().query("inner_path");

    match serve_uimedia_file(inner_path) {
        Ok(f) => f.respond_to(&req),
        Err(_) => HttpResponse::BadRequest().finish(),
    }
}

fn serve_uimedia_file(inner_path: &str) -> Result<NamedFile, Error> {
    trace!("Serving uimedia file: {:?}", inner_path);
    let mut file_path = PathBuf::from("./ui/media");

    //TODO!: InFallible Handling of files
    match inner_path {
        "favicon.ico" | "apple-touch-icon.png" => file_path.push(&Path::new("img")),
        _ => {}
    }
    file_path.push(&Path::new(inner_path));

    if !file_path.is_file() {
        return Err(Error::FileNotFound(file_path.to_str().unwrap().to_string()));
    }
    let f = NamedFile::open(file_path)?;

    Ok(f)
}
