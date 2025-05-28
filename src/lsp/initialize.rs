use log::info;
use lsp_types::{
    CodeActionKind, CodeActionOptions, CodeActionProviderCapability, CompletionOptions,
    DiagnosticOptions, DiagnosticRegistrationOptions, DiagnosticServerCapabilities,
    HoverProviderCapability, InitializeParams, InitializeResult, OneOf, ServerCapabilities,
    ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, WorkDoneProgressOptions,
};
use miette::{IntoDiagnostic, Result};

use crate::message::{Request, Response};

pub fn process_initialize(request: Request) -> Result<(Response, InitializeParams)> {
    let initialize_params: InitializeParams =
        serde_json::from_value(request.params).into_diagnostic()?;

    info!("Client Info: {:?}", initialize_params.client_info);

    let initialize_result = InitializeResult {
        capabilities: ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            definition_provider: Some(OneOf::Left(true)),
            code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
                code_action_kinds: Some(vec![CodeActionKind::REFACTOR_EXTRACT]),
                ..Default::default()
            })),
            diagnostic_provider: Some(DiagnosticServerCapabilities::RegistrationOptions(
                DiagnosticRegistrationOptions {
                    diagnostic_options: DiagnosticOptions {
                        inter_file_dependencies: true,
                        workspace_diagnostics: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )),
            references_provider: Some(OneOf::Left(true)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(true),
                trigger_characters: Some(vec![
                    "#".to_string(),
                    "[".to_string(),
                    ":".to_string(),
                    "(".to_string(),
                ]),
                ..Default::default()
            }),
            ..Default::default()
        },
        server_info: Some(ServerInfo {
            name: "doug-learn-lsp".to_string(),
            version: Some("0.0.0.0.0.0-beta1.final".to_string()),
        }),
    };
    let result = serde_json::to_value(initialize_result).unwrap();

    log::trace!("InitializeResult: {:?}", result);

    Ok((
        Response::from_ok(request.id, Some(result)),
        initialize_params,
    ))
}
