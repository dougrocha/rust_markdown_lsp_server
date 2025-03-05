use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

pub const RPC_VERSION: &str = "2.0";

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Message {
    Request(Request),
    Notification(Notification),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Request<P = Value> {
    #[serde(rename = "jsonrpc")]
    pub rpc: String,
    /// The request ID
    pub id: usize,
    /// The method to be invoked.
    pub method: String,
    /// The method's params
    #[serde(bound = "P: Serialize + DeserializeOwned")]
    pub params: P,
}

impl Request {
    pub fn new<P>(id: usize, method: &str, params: P) -> Self
    where
        P: Serialize,
    {
        Self {
            rpc: RPC_VERSION.to_string(),
            id,
            method: method.to_string(),
            params: serde_json::to_value(params).unwrap(),
        }
    }
}

pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseError {
    /// A number indicating the error type that occurred.
    pub code: i32,
    /// A string providing a short description of the error.
    pub message: String,
    /// A primitive or structured value that contains additional
    /// information about the error. Can be omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response<P = Value> {
    pub jsonrpc: String,
    /// The Request ID
    /// TODO: Change to support number or string
    pub id: usize,
    /// The result of a request. This member is REQUIRED on success.
    /// This member MUST NOT exist if there was an error invoking the method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<P>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

impl Response {
    pub fn new<P>(id: usize, result: P) -> Self
    where
        P: Serialize,
    {
        let value = serde_json::to_value(result).unwrap_or(Value::Null);
        Self {
            jsonrpc: RPC_VERSION.to_string(),
            id,
            result: Some(value),
            error: None,
        }
    }

    pub fn error(id: usize, code: i32, msg: impl Into<String>) -> Self {
        Self {
            jsonrpc: RPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(ResponseError {
                code,
                message: msg.into(),
                data: None,
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Notification<P = Value> {
    pub jsonrpc: String,

    pub method: String,

    #[serde(bound = "P: Serialize + DeserializeOwned + Default")]
    pub params: P,
}

impl<P> Notification<P>
where
    P: Serialize + DeserializeOwned + Default,
{
    pub fn new(method: &str, params: P) -> Self {
        Self {
            jsonrpc: RPC_VERSION.to_string(),
            method: method.to_string(),
            params,
        }
    }
}
