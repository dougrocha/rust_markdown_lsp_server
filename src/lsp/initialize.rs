use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::message::{Request, Response};

use super::URI;

#[derive(Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct InitializeParams {
    process_id: Option<usize>,
    client_info: Option<ClientInfo>,
    workspace_folders: Option<Vec<WorkspaceFolder>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct WorkspaceFolder {
    /// The associated URI for this workspace folder.
    uri: URI,
    /// The name of the workspace folder. Used to refer to this
    /// workspace folder in the user interface.
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientInfo {
    name: String,
    version: Option<String>,
}
pub type ServerInfo = ClientInfo;

#[derive(Serialize, Debug)]
pub struct InitializeResult {
    capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    server_info: Option<ServerInfo>,
}

#[derive(Serialize, Debug)]
pub struct ServerCapabilities {
    #[serde(rename = "textDocumentSync")]
    text_document_sync: Option<usize>,
    #[serde(rename = "hoverProvider")]
    hover_provider: bool,
}

pub fn process_initialize(request: Request) -> (Response, InitializeParams) {
    let initialize_params: InitializeParams = serde_json::from_value(request.params).unwrap();

    info!("{:?}", initialize_params.client_info);

    let initialize_result = InitializeResult {
        capabilities: ServerCapabilities {
            text_document_sync: Some(1),
            hover_provider: true,
        },
        server_info: Some(ServerInfo {
            name: "doug-learn-lsp".to_string(),
            version: Some("0.0.0.0.0.0-beta1.final".to_string()),
        }),
    };
    let result = serde_json::to_value(initialize_result).unwrap();

    (Response::new(request.id, Some(result)), initialize_params)
}
