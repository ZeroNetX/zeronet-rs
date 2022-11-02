use actix_web::{http::header, HttpResponse};

pub fn redirect(path: &str) -> HttpResponse {
    let mut resp = HttpResponse::PermanentRedirect();
    resp.append_header((header::LOCATION, path));
    resp.finish()
}
