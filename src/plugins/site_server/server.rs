use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{Arc, Mutex},
};

use actix::Addr;
use actix_web::{
    body::BoxBody,
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    http::header::{self, HeaderMap},
    web::{get, Data, Query},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use log::*;
use mime::Mime;
use regex::Regex;

use crate::{
    controllers::{sites::SitesController, users::UserController},
    environment::ENV,
    header_name, header_value,
    plugins::{
        register_plugins,
        site_server::{common::*, error::*, media::*, wrapper::*},
        websocket,
    },
};

pub struct ZeroServer {
    pub user_controller: actix::Addr<UserController>,
    pub site_controller: actix::Addr<SitesController>,
    pub wrapper_nonces: Arc<Mutex<HashSet<String>>>,
}

pub trait AppEntryImpl = ServiceFactory<
    ServiceRequest,
    Response = ServiceResponse<BoxBody>,
    Config = (),
    InitError = (),
    Error = actix_web::Error,
>;

fn build_app(shared_data: ZeroServer) -> App<impl AppEntryImpl> {
    //TODO! Handle REMOTE_ADDR & HTTP_HOST via middleware
    let app = register_plugins(App::new().app_data(Data::new(shared_data)));
    websocket::register_site_plugins(app)
        .route("/", get().to(index))
        .route("/{address:1[^/]+}", get().to(serve_site))
        .route("/{address:1[^/]+}/{inner_path:.*}", get().to(serve_site))
        .route("/raw/{inner_path}.{ext}", get().to(serve_raw_media))
        .route("/uimedia/{inner_path:.*}", get().to(serve_uimedia))
        .route("/{inner_path:.*}", get().to(serve_uimedia))
}

pub async fn run(
    site_controller: Addr<SitesController>,
    user_controller: Addr<UserController>,
) -> std::io::Result<()> {
    info!("Ui Listening On: http://{}:{}", ENV.ui_ip, ENV.ui_port);
    let nonces = Arc::new(Mutex::new(HashSet::new()));
    HttpServer::new(move || {
        let shared_data = ZeroServer {
            site_controller: site_controller.clone(),
            user_controller: user_controller.clone(),
            wrapper_nonces: nonces.clone(),
        };
        build_app(shared_data)
    })
    .bind(format!("{}:{}", &*ENV.ui_ip, &ENV.ui_port))
    .unwrap()
    .run()
    .await
}

pub async fn index(_: HttpRequest) -> impl Responder {
    let mut resp = HttpResponse::PermanentRedirect();
    resp.append_header((header::LOCATION, &*ENV.homepage));
    resp
}

async fn serve_site(req: HttpRequest, query: Query<HashMap<String, String>>) -> HttpResponse {
    let data = req.app_data::<Data<ZeroServer>>().unwrap().clone();
    let mut wrapper = true;
    let address = req.match_info().query("address");
    let inner_path = req.match_info().query("inner_path");
    // let addr_str = address.to_string();
    // let site_controller = data.site_controller.clone();
    // actix::spawn(async move {
    //     info!("Sending site announce to {}", &addr_str);
    //     let address = Address::from_str(&addr_str).unwrap();
    //     let start = Instant::now();
    //     site_controller.do_send(SiteAnnounce { address });
    //     let taken = start.duration_since(start);
    //     println!("{}", taken.as_micros());
    // });
    let header_allow_ajax = !req.match_info().query("ajax_key").is_empty();
    //TODO! Check if ajax_key matches with saved one
    let path = format!("{}/{}", address, inner_path);
    let is_wrapper_necessary = is_wrapper_necessary(&path);
    if !is_wrapper_necessary {
        return serve_sitemedia(req, &format!("/media/{path}"), header_allow_ajax, None).await;
    } else if !inner_path.is_empty()
        && inner_path.contains('.')
        && !inner_path.ends_with(".html")
        && !inner_path.ends_with(".xhtml")
    {
        wrapper = false;
    } else {
        let mut wrapper_nonces = req
            .app_data::<Data<ZeroServer>>()
            .unwrap()
            .wrapper_nonces
            .lock()
            .unwrap();
        let wrapper_nonce = query.get("wrapper_nonce");
        if wrapper_nonce.is_some() && wrapper_nonces.contains(wrapper_nonce.unwrap()) {
            wrapper_nonces.remove(wrapper_nonce.unwrap());
            wrapper = false;
        } else if wrapper_nonce.is_some() {
            warn!("Nonce {:?} invalid!", wrapper_nonce);
        }
    }

    if is_ajax_request(&req) {
        return error403(&req, Some("Ajax request not allowed to load wrapper"));
    } else if is_web_socket_request(&req) {
        return error403(&req, Some("WebSocket request not allowed to load wrapper"));
    }
    if let Some(value) = get_header_value(&req, header_name!("http_accept")) {
        if !value.contains("text/html") {
            let v = format!("Invalid Accept header to load wrapper: {value}");
            return error403(&req, Some(&v));
        }
    }

    if let Some(value) = get_header_value(&req, header_name!("http_x_moz")) {
        if value.contains("prefetch") {
            return error403(&req, Some("Prefetch not allowed to load wrapper"));
        }
    }

    if let Some(value) = get_header_value(&req, header_name!("http_purpose")) {
        if value.contains("prefetch") {
            return error403(&req, Some("Prefetch not allowed to load wrapper"));
        }
    }

    trace!(
        "No valid nonce provided, serving wrapper for zero:://{}",
        address
    );
    serve_wrapper(req, data, !wrapper).await
}

pub fn build_header(
    status: Option<u16>,
    content_type: Option<&str>,
    no_script: Option<bool>,
    allow_ajax: Option<bool>,
    script_nonce: Option<&str>,
    extra_header: Option<HeaderMap>,
    request_method: Option<&str>,
) -> HeaderMap {
    let mut content_type = String::from(content_type.unwrap_or("text/html"));
    let status = status.unwrap_or(200);
    let request_method = request_method.unwrap_or("GET");
    let no_script = no_script.unwrap_or(false);
    let allow_ajax = allow_ajax.unwrap_or(false);
    let extra_headers = extra_header.unwrap_or_default();

    let attachable = Regex::new("svg|xml|x-shockwave-flash|pdf")
        .unwrap()
        .is_match(&content_type);
    let nonce = if let Some(nonce) = script_nonce {
        format!("default-src 'none'; script-src 'nonce-{}'; img-src 'self' blob: data:; style-src 'self' blob: 'unsafe-inline'; connect-src *; frame-src 'self' blob:", &nonce)
    } else {
        "".into()
    };

    let cacheable_type = request_method == "OPTIONS"
        || Regex::new("image|video|font|application/javascript|text/css")
            .unwrap()
            .is_match(&content_type);

    let regex = Regex::new("text/plain|text/html|text/css|application/javascript|application/json|application/manifest+json").unwrap();
    if regex.is_match(&content_type) {
        content_type += "; charset=utf-8";
    }

    let is_html = Mime::from_str(&content_type).unwrap().subtype() == mime::HTML;

    let mut headers = prepare_header![
        header_name!("version") => "HTTP/1.1",
        header::CONNECTION => "keep-alive",
        header_name!("keep-alive") => "max=25, timeout=30",
        header::X_FRAME_OPTIONS => "SAMEORIGIN",;
        no_script =>> header::CONTENT_SECURITY_POLICY => "default-src 'none'; sandbox allow-top-navigation allow-forms; img-src *; font-src * data:; media-src *; style-src * 'unsafe-inline';",
        allow_ajax =>> header::ACCESS_CONTROL_ALLOW_ORIGIN => "null",
        !is_html =>> header::ACCESS_CONTROL_ALLOW_ORIGIN => "*",
        request_method == "OPTIONS" =>> header::ACCESS_CONTROL_ALLOW_HEADERS => "Origin, X-Requested-With, Content-Type, Accept, Cookie, Range",
        request_method == "OPTIONS" =>> header::ACCESS_CONTROL_ALLOW_CREDENTIALS => "true",
        attachable =>> header::CONTENT_DISPOSITION => "attachment",
        cacheable_type & [200, 206].contains(&status) =>> header::CACHE_CONTROL => "public, max-age=600",
        !cacheable_type & [200, 206].contains(&status) =>> header::CACHE_CONTROL => "no-cache, no-store, private, must-revalidate, max-age=0",
    ];

    prepare_header![headers, header::CONTENT_TYPE =>> content_type];

    if !nonce.is_empty() {
        prepare_header![ headers, header::CONTENT_SECURITY_POLICY =>> nonce ];
    }

    for (key, value) in extra_headers.into_iter() {
        headers.append(key, value);
    }
    headers
}
