use actix_web::{
    http::header::{self, AsHeaderName, HeaderValue},
    HttpRequest, HttpResponse,
};

pub fn redirect(path: &str) -> HttpResponse {
    let mut resp = HttpResponse::PermanentRedirect();
    resp.append_header((header::LOCATION, path));
    resp.finish()
}

pub fn get_header_value(req: &HttpRequest, key: impl AsHeaderName) -> Option<&str> {
    let res = req.headers().get(key);
    if let Some(header) = res {
        if let Ok(value) = header.to_str() {
            Some(value)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn is_web_socket_request(req: &HttpRequest) -> bool {
    let res = get_header_value(req, header_name!("http_upgrade"));
    if res.is_none() {
        return false;
    }
    res.unwrap() == header_value!("websocket")
}

pub fn is_ajax_request(req: &HttpRequest) -> bool {
    let res = get_header_value(req, header_name!("http_x_requested_with"));
    if res.is_none() {
        return false;
    }
    res.unwrap() == header_value!("XMLHttpRequest")
}

pub fn is_script_nonce_supported(req: &HttpRequest) -> bool {
    let res = get_header_value(req, header_name!("http_user_agent"));
    if res.is_none() {
        return true;
    }
    let user_agent = res.unwrap();
    if user_agent.contains("Edge/") {
        false
    } else if user_agent.contains("Safari/") & !user_agent.contains("Chrome/") {
        false
    } else {
        true
    }
}

pub fn get_referer(req: &HttpRequest) -> Option<&HeaderValue> {
    let res = req.headers().get(header_name!("http_referer"));
    res
}
