use log::info;
use lsp_types::{
    CodeActionKind, CodeActionOptions, HoverProviderCapability, InitializeParams, InitializeResult,
    OneOf, ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
};

use crate::message::{Request, Response};

pub fn process_initialize(request: Request) -> (Response, InitializeParams) {
    let test: serde_json::Value = serde_json::from_value(request.params.clone()).unwrap();
    let initialize_params: InitializeParams = serde_json::from_value(request.params).unwrap();

    info!("{:?}", serde_json::to_string(&test));
    info!("{:?}", initialize_params.client_info);

    let initialize_result = InitializeResult {
        capabilities: ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            definition_provider: Some(OneOf::Left(true)),
            code_action_provider: Some(lsp_types::CodeActionProviderCapability::Options(
                CodeActionOptions {
                    code_action_kinds: Some(vec![CodeActionKind::REFACTOR_EXTRACT]),
                    ..Default::default()
                },
            )),
            ..Default::default()
        },
        server_info: Some(ServerInfo {
            name: "doug-learn-lsp".to_string(),
            version: Some("0.0.0.0.0.0-beta1.final".to_string()),
        }),
    };
    let result = serde_json::to_value(initialize_result).unwrap();

    log::debug!("{}:", result);

    (
        Response::from_ok(request.id, Some(result)),
        initialize_params,
    )
}
