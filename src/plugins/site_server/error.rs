use actix_web::{HttpRequest, HttpResponse, Responder};

use super::server::build_header;

pub fn error400(req: &HttpRequest, message: Option<&str>) -> impl Responder {
    let mut res = build_error("Bad Request", message.unwrap_or(""));
    let s = res.headers_mut();
    let headers = build_header!(400, true);
    headers.clone_into(s);
    res.respond_to(req)
}

pub fn error403(req: &HttpRequest, message: Option<&str>) -> impl Responder {
    let mut res = build_error("Forbidden", message.unwrap_or(""));
    let s = res.headers_mut();
    let headers = build_header!(403, true);
    headers.clone_into(s);
    res.respond_to(req)
}

pub fn error404(req: &HttpRequest, path: Option<&str>) -> impl Responder {
    let mut res = build_error("Not Found", path.unwrap_or(""));
    let s = res.headers_mut();
    let headers = build_header!(404, true);
    headers.clone_into(s);
    res.respond_to(req)
}

pub fn error500(req: &HttpRequest, message: Option<&str>) -> impl Responder {
    let mut res = build_error("Server error", message.unwrap_or(":("));
    let s = res.headers_mut();
    let headers = build_header!(500, true);
    headers.clone_into(s);
    res.respond_to(req)
}

pub fn build_error(title: &str, message: &str) -> HttpResponse {
    let body = format!(
        "<style>
        * {{ font-family: Consolas, Monospace; color: #333; }}
        code {{ font-family: Consolas, Monospace; background-color: #EEE }}
        </style>
        <h1>{}</h1>
        <h2>{}</h3>",
        title, message
    );

    HttpResponse::Ok().body(body)
}
