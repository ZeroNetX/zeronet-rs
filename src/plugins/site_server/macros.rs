#[macro_export]
macro_rules! header_name {
    ($value:expr) => {
        actix_web::http::header::HeaderName::from_static($value)
    };
}

#[macro_export]
macro_rules! header_value {
    ($value:expr) => {
        actix_web::http::header::HeaderValue::from_static($value)
    };
}

//TODO!: Add documentation and tests for build_header! and build_header
#[macro_export]
macro_rules! build_header {
    () => {
        build_header!(None, None, None, None, None, None, None)
    };
    ($status:expr, $content_type:expr, $script_nonce:expr) => {
        build_header!(
            Some($status),
            None,
            None,
            None,
            Some($script_nonce),
            None,
            None
        )
    };
    ($status:expr) => {
        build_header!(Some($status), None, None, None, None, None, None)
    };
    ($extra_headers:expr) => {
        build_header!(None, None, None, None, None, Some($extra_headers), None)
    };
    ($content_type:expr) => {
        build_header!(None, Some($content_type), None, None, None, None, None)
    };
    ($status:expr, $no_script:expr) => {
        build_header!(
            Some($status),
            None,
            Some($no_script),
            None,
            None,
            None,
            None
        )
    };
    ($status:expr, $content_type:expr, $no_script:expr, $allow_ajax:expr, $script_nonce:expr, $extra_headers:expr, $request_method:expr) => {
        build_header(
            $status,
            $content_type,
            $no_script,
            $allow_ajax,
            $script_nonce,
            $extra_headers,
            $request_method,
        )
    };
}

/// prepare_header macro improves code readability of HeaderMap key values when appending to HeaderMap
/// ```
/// let mut header_map = HeaderMap::new();
/// header_map.append(HeaderName::from_static("version"), HeaderValue::from_static("HTTP/1.1"));
/// header_map.append(header::X_FRAME_OPTIONS, HeaderValue::from_static("SAMEORIGIN"));
/// ```
///
/// becomes
/// ```
/// let header_map = prepare_header![
///     header_name!("version") => "HTTP/1.1",
///     header::X_FRAME_OPTIONS => "SAMEORIGIN",
/// ];
/// ```
/// There are more complex pattern available
/// You can add headers conditionally
/// via
/// ```
/// let header_map = prepare_header![
///     header_name!("version") => "HTTP/1.1",
///     header::X_FRAME_OPTIONS => "SAMEORIGIN",;
///     cache =>> header::CACHE_CONTROL => "public, max-age=600",
///     cache =>> header::CACHE_CONTROL => "public, max-age=600",
/// ];
/// ```
/// If header_value is [String] syntax is
/// ```
/// let mut header_map = prepare_header![];
/// prepare_header![header_map, header_name!("hello") =>> String("world")]
/// ```
///
///
#[macro_export]
macro_rules! prepare_header {
    ($($key:expr => $value:expr,)*) => {{
        let mut header_map = actix_web::http::header::HeaderMap::new();
        $(
            prepare_header!(header_map, $key, $value);
        )*
        header_map
    }};
    ($($key:expr => $value:expr,)*; $($condition:expr =>> $key_c:expr => $value_c:expr,)*) => {{
        let mut header_map = actix_web::http::header::HeaderMap::new();
        $(
            prepare_header!(header_map, $key, $value);
        )*

        $(
            if $condition {
                prepare_header!(header_map, $key_c, $value_c);
            }
        )*
        header_map
    }};
    ($header_map:expr, $key:expr =>> $value:expr) => {{
        let value: String = $value;
        $header_map.append($key, actix_web::http::header::HeaderValue::from_str(&value).unwrap());
    }};
    ($header_map:expr, $key:expr, $value:expr) => {
        $header_map.append($key, header_value!($value));
    };
}

#[cfg(test)]
mod tests {
    use actix_web::http::header;

    #[test]
    fn prepare_header() {
        let headers = prepare_header![
            header_name!("version") => "HTTP/1.1",
            header::X_FRAME_OPTIONS => "SAMEORIGIN",
            header::CONNECTION => "Keep-Alive",
            header::CONTENT_TYPE => "text/css",
            header::CACHE_CONTROL => "no-cache",
        ];
        assert_eq!(headers.len(), 5);
        assert_eq!(
            headers.get(header_name!("version")),
            Some(&header_value!("HTTP/1.1"))
        );
        assert_eq!(
            headers.get(header::X_FRAME_OPTIONS),
            Some(&header_value!("SAMEORIGIN"))
        );
        assert_eq!(
            headers.get(header::CONNECTION),
            Some(&header_value!("Keep-Alive"))
        );
        assert_eq!(
            headers.get(header::CONTENT_TYPE),
            Some(&header_value!("text/css"))
        );
        assert_eq!(
            headers.get(header::CACHE_CONTROL),
            Some(&header_value!("no-cache"))
        );
    }
}
