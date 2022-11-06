use std::{fs::File, io::Read, path::Path, str::FromStr};

use actix_web::{
    body::BoxBody, http::ConnectionType::KeepAlive, HttpRequest, HttpResponse, Responder, Result,
};
use log::*;
use mime_guess::MimeGuess;
use regex::Regex;
use tokio::fs;
use zerucontent::{Content, Number};

use crate::{
    core::address::Address,
    environment::{DEF_TEMPLATES_PATH, ENV},
    plugins::site_server::{
        common::get_nonce,
        file::serve_file,
        handlers::{
            sites::{GetSiteKeys, Lookup, SiteContent},
            users::UserSettings,
        },
        media::parse_media_path,
        server::build_header,
    },
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
    has_wrapper_nonce: bool,
) -> HttpResponse {
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
        .send(GetSiteKeys {
            address: address.clone(),
        })
        .await;

    if result.is_err() {
        error!("Error getting wrapper key to site manager");
    }
    let (nonce, ajax_key) = result.unwrap().unwrap();
    {
        let mut nonces = data.wrapper_nonces.lock().unwrap();
        nonces.insert(nonce.clone());
        trace!("Valid nonces ({}): {:?}", nonces.len(), nonces);
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
    let content = site.send(SiteContent(None)).await;
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
    let title;
    let show_loadingscreen;
    let content = if let Ok(Ok(content)) = content {
        show_loadingscreen = String::from("false");
        title = content.title.to_string();
        content
    } else {
        show_loadingscreen = String::from("true");
        title = format!("Loading {}...", address.address);
        Content::default()
    };
    if has_wrapper_nonce {
        let inner_path = &format!("/media/{}/{}", address.address, inner_path);
        let (address, inner_path) = parse_media_path(inner_path).unwrap();
        let file_path = &ENV.data_path.join(address).join(&inner_path);
        match serve_file(&req, file_path, None, Some(true), None, Some(false), None).await {
            Ok((res, headers)) => {
                let content_type = res.content_type().clone();
                let type_ = content_type.type_();
                let subtype = content_type.subtype();
                let file_path = res.path().to_owned();
                let response = res.respond_to(&req);
                let is_html = type_ == "text" && subtype == "html";
                let mut response = if is_html {
                    let mut string = fs::read_to_string(file_path).await.unwrap();
                    string = string.replace("{themeclass}", &themeclass);
                    let modified = match content.modified {
                        Number::Float(float) => float.to_string(),
                        Number::Integer(integer) => integer.to_string(),
                    };
                    string = string.replace("{site_modified}", &modified);
                    string = string.replace("{lang}", &ENV.lang);
                    response.set_body(BoxBody::new(string))
                } else {
                    response
                };
                if let Some(headers_) = headers {
                    let headers = response.headers_mut();
                    headers.clear();
                    for (key, value) in headers_.into_iter() {
                        headers.append(key, value);
                    }
                }
                response.head_mut().set_connection_type(KeepAlive);
                return response;
            }
            Err(err) => {
                error!("Serve Site:: Bad request {:?}", err);
                return HttpResponse::BadRequest().finish();
            }
        }
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

    let mut path = (*DEF_TEMPLATES_PATH).to_owned();
    path.push("wrapper.html");

    let sandbox_permissions = "".into();
    let script_nonce = get_nonce(true, 64);
    let mut query_string = req.query_string().to_owned();
    if query_string.is_empty() {
        query_string = format!("\\?wrapper_nonce\\={}", nonce.clone());
    } else {
        query_string = format!("\\?{}&wrapper_nonce\\={}", query_string, nonce.clone(),);
    }
    let string = match render(
        &path,
        WrapperData {
            file_url: format!("\\/{}\\/{}", address, inner_path),
            file_inner_path: String::from(inner_path),
            address: address.to_string(),
            title,
            body_style,
            meta_tags,
            query_string,
            wrapper_key: nonce.clone(),
            ajax_key,
            wrapper_nonce: nonce.clone(),
            postmessage_nonce_security,
            permissions: String::from("[]"), //TODO!: Need to Replace with permissions from site settings
            show_loadingscreen,              //TODO! Handle this when websockets are implemented
            sandbox_permissions,
            rev: format!("{}", ENV.rev),
            lang: ENV.lang.to_string(),
            homepage: String::from(&*ENV.homepage),
            themeclass,
            script_nonce: script_nonce.clone(),
        },
    ) {
        Ok(s) => s,
        Err(_) => String::new(),
    };
    let mut res = HttpResponse::Ok();
    for (key, value) in build_header!(200, None, &script_nonce).iter() {
        res.append_header((key.as_str(), value.to_str().unwrap()));
    }
    res.keep_alive().body(string)
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

pub fn is_wrapper_necessary(path: &str) -> bool {
    let regex = Regex::new("/(?P<address>[A-Za-z0-9\\._-]+)(?P<inner_path>/.*|$)").unwrap();
    let result = regex.captures(path);
    if result.is_none() {
        return true;
    }
    let inner_path = result.unwrap().name("inner_path");

    if inner_path.is_none()
        || inner_path.unwrap().as_str().ends_with('/')
        || inner_path.unwrap().as_str().ends_with("html")
    {
        true
    } else {
        let mime = MimeGuess::from_path(path);
        mime.iter().any(|type_| type_.type_() == mime::HTML)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDR: &str = "1HelloAddr";
    const ADDR1: &str = "1HelloAddr.bit";
    const ADDR2: &str = "1Hello_Addr.bit";
    const ADDR3: &str = "1Hello-Addr.bit";
    const INNER_PATH: &str = "index.html";
    const INNER_PATH1: &str = "index.xhtml";
    const INNER_PATH2: &str = "index.js";
    const INNER_PATH3: &str = "index.css";

    #[test]
    fn test_is_wrapper_necessary() {
        let prepare_path = |addr: &str, inner_path: &str| format!("/{addr}/{inner_path}");

        let test = is_wrapper_necessary(&prepare_path(ADDR, INNER_PATH));
        assert!(test);

        let test = is_wrapper_necessary(&prepare_path(ADDR3, ""));
        assert!(test);

        let test = is_wrapper_necessary(&prepare_path(ADDR1, INNER_PATH1));
        assert!(test);

        let test = is_wrapper_necessary(&prepare_path(ADDR2, INNER_PATH2));
        assert!(!test);

        let test = is_wrapper_necessary(&prepare_path(ADDR3, INNER_PATH3));
        assert!(!test);
    }
}
