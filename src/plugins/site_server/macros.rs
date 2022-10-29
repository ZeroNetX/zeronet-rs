#[macro_export]
macro_rules! header_name {
    ($value:expr) => {
        HeaderName::from_static($value)
    };
}

#[macro_export]
macro_rules! header_value {
    ($value:expr) => {
        HeaderValue::from_static($value)
    };
}

#[macro_export]
macro_rules! build_header {
    () => {
        build_header!(None, None, None, None, None, None, None)
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
