use lsp_types::{
    CodeActionKind, CodeActionOptions, CodeActionProviderCapability, CompletionOptions,
    DiagnosticOptions, DiagnosticRegistrationOptions, DiagnosticServerCapabilities,
    DocumentSymbolOptions, FileOperationFilter, FileOperationPattern, FileOperationPatternKind,
    FileOperationRegistrationOptions, HoverProviderCapability, InitializeParams, InitializeResult,
    OneOf, RenameOptions, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind, WorkspaceFileOperationsServerCapabilities,
    WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities, WorkspaceSymbolOptions,
};
use miette::{IntoDiagnostic, Result};

use crate::messages::{Request, Response};

pub fn process_initialize(request: Request) -> Result<(Response, InitializeParams)> {
    let initialize_params: InitializeParams =
        serde_json::from_value(request.params).into_diagnostic()?;

    tracing::info!("Client Info: {:?}", initialize_params.client_info);

    let markdown_file_filter = FileOperationFilter {
        scheme: Some("file".to_string()),
        pattern: FileOperationPattern {
            glob: "**/*.md".to_string(),
            matches: Some(FileOperationPatternKind::File),
            ..Default::default()
        },
    };

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
            document_symbol_provider: Some(OneOf::Right(DocumentSymbolOptions {
                label: Some("Markdown Symbols".to_string()),
                work_done_progress_options: Default::default(),
            })),
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(OneOf::Left(true)),
                }),
                file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                    will_rename: Some(FileOperationRegistrationOptions {
                        filters: vec![markdown_file_filter.clone()],
                    }),
                    did_rename: Some(FileOperationRegistrationOptions {
                        filters: vec![markdown_file_filter],
                    }),
                    ..Default::default()
                }),
            }),
            workspace_symbol_provider: Some(OneOf::Right(WorkspaceSymbolOptions {
                resolve_provider: Some(false),
                work_done_progress_options: Default::default(),
            })),
            rename_provider: Some(OneOf::Right(RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options: Default::default(),
            })),
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
            version: Some("0.0.1.test".to_string()),
        }),
    };
    let result = serde_json::to_value(initialize_result).into_diagnostic()?;

    tracing::trace!("InitializeResult: {result:?}");

    Ok((
        Response::from_ok(request.id, Some(result)),
        initialize_params,
    ))
}
