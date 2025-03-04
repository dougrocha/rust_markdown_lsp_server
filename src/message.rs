use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    #[serde(rename = "jsonrpc")]
    pub rpc: String,

    pub id: usize,
    pub method: String,
    #[serde(default = "serde_json::Value::default")]
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub params: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    #[serde(rename = "jsonrpc")]
    pub rpc: String,

    pub id: usize,
    pub result: Option<serde_json::Value>,
}

impl Response {
    pub fn new(id: usize, result: Option<Value>) -> Self {
        Self {
            rpc: "2.0.0".to_string(),
            id,
            result,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Notification {
    #[serde(rename = "jsonrpc")]
    pub rpc: String,

    pub method: String,
    #[serde(default = "serde_json::Value::default")]
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub params: serde_json::Value,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Message {
    Request(Request),
    Notification(Notification),
}
