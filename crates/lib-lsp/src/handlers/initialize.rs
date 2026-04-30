use gen_lsp_types::{
    ChangeNotifications, CodeActionOptions, CodeActionProvider, CompletionOptions,
    DefinitionProvider, DiagnosticOptions, DiagnosticProvider, DocumentSymbolOptions,
    DocumentSymbolProvider, FileOperationFilter, FileOperationOptions, FileOperationPattern,
    FileOperationPatternKind, FileOperationRegistrationOptions, HoverProvider, InitializeParams,
    InitializeResult, ReferenceOptions, ReferencesProvider, RenameOptions, RenameProvider,
    ServerCapabilities, ServerInfo, TextDocumentSync, WorkspaceFoldersServerCapabilities,
    WorkspaceOptions, WorkspaceSymbolOptions, WorkspaceSymbolProvider,
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
            text_document_sync: Some(TextDocumentSync::Kind(
                gen_lsp_types::TextDocumentSyncKind::Full,
            )),
            hover_provider: Some(HoverProvider::Bool(true)),
            definition_provider: Some(DefinitionProvider::Bool(true)),
            code_action_provider: Some(CodeActionProvider::CodeActionOptions(CodeActionOptions {
                code_action_kinds: Some(vec![gen_lsp_types::CodeActionKind::RefactorExtract]),
                ..Default::default()
            })),
            diagnostic_provider: Some(DiagnosticProvider::DiagnosticOptions(DiagnosticOptions {
                inter_file_dependencies: true,
                workspace_diagnostics: true,
                ..Default::default()
            })),
            references_provider: Some(ReferencesProvider::ReferenceOptions(ReferenceOptions {
                ..Default::default()
            })),
            document_symbol_provider: Some(DocumentSymbolProvider::DocumentSymbolOptions(
                DocumentSymbolOptions {
                    label: Some("Markdown Symbols".to_string()),
                    ..Default::default()
                },
            )),
            workspace: Some(WorkspaceOptions {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(ChangeNotifications::Bool(true)),
                }),
                file_operations: Some(FileOperationOptions {
                    will_rename: Some(FileOperationRegistrationOptions {
                        filters: vec![markdown_file_filter.clone()],
                    }),
                    did_rename: Some(FileOperationRegistrationOptions {
                        filters: vec![markdown_file_filter],
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            workspace_symbol_provider: Some(WorkspaceSymbolProvider::WorkspaceSymbolOptions(
                WorkspaceSymbolOptions {
                    resolve_provider: Some(false),
                    ..Default::default()
                },
            )),
            rename_provider: Some(RenameProvider::RenameOptions(RenameOptions {
                prepare_provider: Some(true),
                ..Default::default()
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
