// use actix_http::error::ResponseError;
// use derive_more::Display;

#[derive(Debug)]
pub enum Error {
    AddressError(String),
    Err(String),
    IOError(String),
    FileNotFound(String),
    WrapperKeyNotFound,
    Deserialization(serde_json::Error),
    MissingError,
    CryptError(String),
    MsgPackEncoding,
    MsgPackDecoding(rmp_serde::decode::Error),
    MailboxError,
    ParseError,
}

// impl From<reqwest::Error> for Error {
//     fn from(error: reqwest::Error) -> Error {
//         match error {
//             _ => Error::ReqwestError,
//         }
//     }
// }

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Error {
        Error::IOError(error.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Error {
        Error::Deserialization(error)
    }
}

impl From<rmp_serde::encode::Error> for Error {
    fn from(error: rmp_serde::encode::Error) -> Error {
        Error::MsgPackEncoding
    }
}

impl From<rmp_serde::decode::Error> for Error {
    fn from(error: rmp_serde::decode::Error) -> Error {
        Error::MsgPackDecoding(error)
    }
}

// impl From<actix::MailboxError> for Error {
//     fn from(error: actix::MailboxError) -> Error {
//         match error {
//             _ => Error::MailboxError,
//         }
//     }
// }

impl From<zeronet_protocol::Error> for Error {
    fn from(error: zeronet_protocol::Error) -> Error {
        println!("{:?}", error);
        Error::MissingError
    }
}

impl From<zeronet_protocol::address::ParseError> for Error {
    fn from(error: zeronet_protocol::address::ParseError) -> Error {
        Error::ParseError
    }
}

impl From<zeronet_cryptography::Error> for Error {
    fn from(error: zeronet_cryptography::Error) -> Error {
        Error::CryptError(error.to_string())
    }
}

// impl ResponseError for Error {}

impl From<&str> for Error {
    fn from(string: &str) -> Error {
        Error::Err(string.to_string())
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Error {
        Error::Err(format!("{:?}", err))
    }
}
