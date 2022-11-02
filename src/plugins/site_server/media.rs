use std::path::Path;

use actix_web::{HttpRequest, HttpResponse, Responder};
use log::*;
use regex::Regex;

use crate::{
    core::error::Error,
    environment::{DEF_MEDIA_PATH, ENV},
    plugins::site_server::file::serve_file,
};

use super::{common::redirect, error::*};

pub async fn serve_sitemedia(
    req: HttpRequest,
    path: &str,
    header_allow_ajax: bool,
    header_no_script: Option<bool>,
) -> HttpResponse {
    let header_no_script = header_no_script.unwrap_or(false);
    let res = parse_media_path(path);
    if res.is_err() {
        match res.unwrap_err() {
            Error::MissingError => {
                return error403(&req, Some("Invalid path"));
            }
            Error::ParseError => {
                return error404(&req, Some(path));
            }
            _ => unreachable!(),
        }
    }
    let (address, inner_path) = res.unwrap();
    let file_path = &ENV.data_path.join(address).join(&inner_path);

    if file_path.is_dir() {
        return redirect(&inner_path);
    } else if file_path.is_file() {
        return match serve_file(
            &req,
            file_path,
            None,
            None,
            None,
            Some(header_no_script),
            Some(header_allow_ajax),
        )
        .await
        {
            Ok((file, headers)) => {
                let mut resp = file.respond_to(&req);
                if let Some(headers_) = headers {
                    let headers = resp.headers_mut();
                    headers.clear();
                    for (key, value) in headers_.into_iter() {
                        headers.append(key, value);
                    }
                }
                resp
            }
            Err(_) => HttpResponse::BadRequest().finish(),
        };
    } else {
        if file_path.ends_with("favicon.ico") || file_path.ends_with("apple-touch-icon.png") {
            return serve_uimedia(req).await;
        }
        //TODO! Handle Missing Files
        unimplemented!("Site Media File Not Exist")
    }
}

pub async fn serve_uimedia(req: HttpRequest) -> HttpResponse {
    let path = req.match_info();
    let inner_path = path.query("inner_path");
    if inner_path.contains("../") {
        error!("Error 403 : {inner_path}");
        return error403(&req, None);
    }

    let mut file_path = (&*DEF_MEDIA_PATH).to_owned();

    //TODO!: InFallible Handling of files
    match inner_path {
        "favicon.ico" | "apple-touch-icon.png" => file_path.push(&Path::new("img")),
        _ => {}
    }
    file_path.push(&Path::new(inner_path));

    // if !file_path.is_file() {
    //     return Err(Error::FileNotFound(file_path.to_str().unwrap().to_string()));
    // }

    match serve_file(&req, file_path.as_path(), None, None, None, None, None).await {
        Ok((file, headers)) => {
            let mut resp = file.respond_to(&req);
            if let Some(headers_) = headers {
                let headers = resp.headers_mut();
                headers.clear();
                for (key, value) in headers_.into_iter() {
                    headers.append(key, value);
                }
            }
            resp
        }
        Err(_) => HttpResponse::BadRequest().finish(),
    }
}

pub async fn serve_raw_media(req: HttpRequest) -> HttpResponse {
    let path = req.match_info();
    let inner_path = &format!("/media/{}.{}", path.query("inner_path"), path.query("ext"));
    println!("Loading Raw Path {inner_path}");
    let header_allow_ajax = !path.query("ajax_key").is_empty();
    serve_sitemedia(req, inner_path, header_allow_ajax, Some(true)).await
}

fn parse_media_path(path: &str) -> Result<(String, String), Error> {
    let mut path = path.replace('\\', "/");
    if path.ends_with('/') {
        path = path + "index.html";
    }
    if path.contains("./") {
        Err(Error::ParseError)
    } else {
        let regex =
            Regex::new("/media/(?P<address>[A-Za-z0-9]+[A-Za-z0-9\\._-]+)(?P<inner_path>/.*|$)")
                .unwrap();
        if let Some(captured) = regex.captures(&path) {
            let addr = captured.name("address").unwrap();
            let inner_path = if let Some(inner) = captured.name("inner_path") {
                let inner = inner.as_str();
                if inner.starts_with('/') {
                    inner.strip_prefix('/').unwrap()
                } else if inner.is_empty() {
                    "index.html"
                } else {
                    inner
                }
            } else {
                "index.html"
            };
            if addr.as_str() == inner_path {
                return Err(Error::MissingError);
            }
            return Ok((addr.as_str().to_owned(), inner_path.to_owned()));
        }
        Err(Error::MissingError)
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

    #[test]
    fn test_parse_media_path() {
        let prepare_path = |addr: &str| format!("/media/{}/index.html", addr);

        let test = parse_media_path("/media/1HelloAddr/");
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), (ADDR.into(), INNER_PATH.into()));

        let test = parse_media_path("/media/1HelloAddr");
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), (ADDR.into(), INNER_PATH.into()));

        let test = parse_media_path(&prepare_path(ADDR));
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), (ADDR.into(), INNER_PATH.into()));

        let test = parse_media_path(&prepare_path(ADDR1));
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), (ADDR1.into(), INNER_PATH.into()));

        let test = parse_media_path(&prepare_path(ADDR2));
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), (ADDR2.into(), INNER_PATH.into()));

        let test = parse_media_path(&prepare_path(ADDR3));
        assert!(test.is_ok());
        assert_eq!(test.unwrap(), (ADDR3.into(), INNER_PATH.into()));

        let test = parse_media_path("/media/ /index.html");
        assert!(test.is_err());
        match test.unwrap_err() {
            Error::MissingError => {}
            _ => unreachable!(),
        }

        let test = parse_media_path("/media/./index.html");
        assert!(test.is_err());

        match test.unwrap_err() {
            Error::ParseError => {}
            _ => unreachable!(),
        }
    }
}
