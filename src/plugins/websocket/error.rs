use actix::MailboxError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Error {
    pub error: String,
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error {
            error: err.to_string(),
        }
    }
}

impl From<MailboxError> for Error {
    fn from(err: MailboxError) -> Error {
        Error {
            error: err.to_string(),
        }
    }
}

impl From<crate::core::error::Error> for Error {
    fn from(err: crate::core::error::Error) -> Error {
        Error {
            error: format!("{err:?}"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error {
            error: err.to_string(),
        }
    }
}
