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
