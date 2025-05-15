use log::info;
use lsp_types::uri::URI;
use serde::{Deserialize, Serialize};

use crate::message::{Request, Response};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub process_id: Option<usize>,
    pub client_info: Option<ClientInfo>,
    pub workspace_folders: Option<Vec<WorkspaceFolder>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFolder {
    /// The associated URI for this workspace folder.
    pub uri: URI,
    /// The name of the workspace folder. Used to refer to this
    /// workspace folder in the user interface.
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientInfo {
    pub name: String,
    pub version: Option<String>,
}

pub type ServerInfo = ClientInfo;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    capabilities: ServerCapabilities,
    server_info: Option<ServerInfo>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    text_document_sync: Option<usize>,
    hover_provider: bool,
    definition_provider: bool,
    code_action_provider: bool,
}

pub fn process_initialize(request: Request) -> (Response, InitializeParams) {
    let initialize_params: InitializeParams = serde_json::from_value(request.params).unwrap();

    info!("{:?}", initialize_params.client_info);

    let initialize_result = InitializeResult {
        capabilities: ServerCapabilities {
            text_document_sync: Some(1),
            hover_provider: true,
            definition_provider: true,
            code_action_provider: true,
        },
        server_info: Some(ServerInfo {
            name: "doug-learn-lsp".to_string(),
            version: Some("0.0.0.0.0.0-beta1.final".to_string()),
        }),
    };
    let result = serde_json::to_value(initialize_result).unwrap();

    (Response::new(request.id, Some(result)), initialize_params)
}
