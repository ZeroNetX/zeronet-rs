use std::path::Path;

use actix_files::NamedFile;
use actix_web::{
    http::header::{self, HeaderMap},
    HttpRequest,
};
use log::error;

use super::server::build_header;
use crate::core::error::Error;

pub async fn serve_file<P>(
    req: &HttpRequest,
    inner_path: P,
    file_size: Option<u64>,
    header_length: Option<bool>,
    send_header: Option<bool>,
    header_noscript: Option<bool>,
    allow_ajax: Option<bool>,
) -> Result<(NamedFile, Option<HeaderMap>), Error>
where
    P: AsRef<Path>,
{
    //
    // let block_size = 64 * 1024;
    let send_header = send_header.unwrap_or(true);
    let mut header_length = header_length.unwrap_or(true);
    let header_noscript = header_noscript.unwrap_or(false);
    let header_allow_ajax = allow_ajax.unwrap_or(false);
    let mut extra_headers = HeaderMap::new();
    let file_size = file_size.unwrap_or(0);
    // let path_parts = 0;

    let file = NamedFile::open(&inner_path);
    if let Ok(file) = file {
        //file_size
        let file_size = if file_size == 0 {
            file.metadata().len()
        } else {
            file_size
        };
        let range = req.headers().get(header::RANGE);
        let content_type = file.content_type();
        let is_html = content_type.type_() == mime::HTML;
        if is_html {
            header_length = false;
        }
        let headers = if send_header {
            let req_path = req.match_info();
            let key = "zeronet_content_encoding";
            let content_encoding = req_path.get(key).unwrap_or("").to_owned();
            let need_header = content_encoding
                .split(",")
                .all(|a| ["gzip", "compress", "deflate", "identity", "br"].contains(&a));
            if need_header {
                prepare_header![extra_headers, header::CONTENT_ENCODING =>> content_encoding]
            }
            prepare_header![extra_headers, header::ACCEPT_RANGES, "bytes"];
            if header_length {
                let len = file_size.to_string();
                prepare_header![extra_headers, header::CONTENT_LENGTH =>> len]
            }
            if range.is_some() {
                unimplemented!("Partial File Requests are not implemented, Please file Bug Report");
            }
            let content_type = content_type.to_string();
            let headers = build_header!(
                Some(200),
                Some(&content_type),
                Some(header_noscript),
                Some(header_allow_ajax),
                None,
                Some(extra_headers),
                None
            );
            Some(headers)
        } else {
            None
        };
        return Ok((file, headers));
    } else {
        error!("serve_file: {:?} Not Found", inner_path.as_ref());
        Err(Error::FileNotFound(format!(
            "{:?}",
            inner_path.as_ref().as_os_str()
        )))
    }
}
