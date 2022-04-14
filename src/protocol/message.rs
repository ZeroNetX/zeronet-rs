use serde::{de::DeserializeOwned, Deserialize, Serialize};
use zeronet_protocol::requestable::Requestable;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum Message<T> {
    Request(Request<T>),
    Response(Response<T>),
}

impl<T: DeserializeOwned + Serialize> Message<T> {
    pub fn request(cmd: &str, req_id: usize, body: T) -> Message<T> {
        let request = Request {
            cmd: cmd.to_string(),
            req_id,
            params: body,
        };
        Message::Request(request)
    }
    pub fn response(to: usize, body: T) -> Message<T> {
        let response = Response {
            cmd: "response".to_string(),
            to,
            body,
        };
        Message::Response(response)
    }
}

impl<T: Serialize + DeserializeOwned> Requestable for Message<T> {
    type Key = usize;

    fn req_id(&self) -> Option<Self::Key> {
        match self {
            Message::Request(req) => Some(req.req_id),
            _ => None,
        }
    }
    fn to(&self) -> Option<Self::Key> {
        match self {
            Message::Response(res) => Some(res.to),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request<T> {
    pub cmd: String,
    pub req_id: usize,
    pub params: T,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response<T> {
    pub cmd: String,
    pub to: usize,
    #[serde(flatten)]
    pub body: T,
}
