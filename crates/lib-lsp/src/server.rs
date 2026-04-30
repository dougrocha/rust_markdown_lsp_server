use gen_lsp_types::{InitializeParams, Notification as LspNotification, Request as LspRequest};
use miette::{Result, miette};
use std::io::{self, BufRead, Write};

use crate::{
    ServerState, dispatch_lsp_request,
    handlers::{
        code_action::process_code_action,
        completion::{completion_resolve::process_completion_resolve, process_completion},
        diagnostics::process_diagnostic,
        did_change::process_did_change,
        did_close::process_did_close,
        did_open::process_did_open,
        document_symbol::process_document_symbol,
        goto_definition::process_goto_definition,
        hover::process_hover,
        initialize::process_initialize,
        references::process_references,
        rename::{
            did_rename::process_did_rename, process_prepare_rename, process_rename,
            will_rename::process_will_rename_files,
        },
        will_create::{process_did_create, process_will_create_files},
        workspace_symbol::process_workspace_symbol,
    },
    messages::{Message, Notification, Request, Response},
    rpc::{encode_message, handle_message, write_msg},
};
use crate::{dispatch_lsp_notification, rpc};

pub fn run_lsp() -> Result<()> {
    tracing::info!("LSP server starting...");

    let (stdin, stdout) = (io::stdin(), io::stdout());
    let (mut reader, mut writer) = (stdin.lock(), stdout.lock());

    let mut lsp = ServerState::default();
    lsp.load_config("rust-markdown-lsp.toml");

    let init_params = handle_initialize(&mut reader, &mut writer)?;
    let workspace_folders = match init_params
        .workspace_folders_initialize_params
        .workspace_folders
    {
        Some(gen_lsp_types::WorkspaceFolders::WorkspaceFolderList(folders)) => Some(folders),
        _ => None,
    };
    lsp.load_workspaces(workspace_folders)?;
    lsp.set_client_capabilities(init_params.capabilities);

    loop {
        match handle_message(&mut reader)? {
            Message::Request(request) => {
                tracing::trace!("{}", request.method);

                match request.method.as_str() {
                    "shutdown" => {
                        tracing::info!("Shutting down");
                        break;
                    }
                    _ => {
                        dispatch_lsp_request!(&mut lsp, request, &mut writer, {
                            gen_lsp_types::HoverRequest => process_hover,
                            gen_lsp_types::DefinitionRequest => process_goto_definition,
                            gen_lsp_types::CodeActionRequest => process_code_action,
                            gen_lsp_types::CompletionRequest => process_completion,
                            gen_lsp_types::CompletionResolveRequest => process_completion_resolve,
                            gen_lsp_types::ReferencesRequest => process_references,
                            gen_lsp_types::DocumentDiagnosticRequest => process_diagnostic,
                            gen_lsp_types::DocumentSymbolRequest => process_document_symbol,
                            gen_lsp_types::WorkspaceSymbolRequest => process_workspace_symbol,
                            gen_lsp_types::PrepareRenameRequest => process_prepare_rename,
                            gen_lsp_types::RenameRequest => process_rename,
                            gen_lsp_types::WillRenameFilesRequest => process_will_rename_files,
                            gen_lsp_types::WillCreateFilesRequest => process_will_create_files,
                        });
                    }
                }
            }
            Message::Notification(notification) => {
                tracing::trace!("{}", notification.method);

                match notification.method.as_str() {
                    "exit" => {
                        tracing::trace!("Received exit notification");
                    }
                    "initialized" => {
                        tracing::trace!("Initialization confirmed!");
                    }
                    _ => {
                        dispatch_lsp_notification!(&mut lsp, notification, {
                            gen_lsp_types::DidOpenTextDocumentNotification => process_did_open,
                            gen_lsp_types::DidChangeTextDocumentNotification => process_did_change,
                            gen_lsp_types::DidCloseTextDocumentNotification => process_did_close,
                            gen_lsp_types::DidRenameFilesNotification => process_did_rename,
                            gen_lsp_types::DidCreateFilesNotification => process_did_create,
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_initialize<R, W>(reader: &mut R, writer: &mut W) -> Result<InitializeParams>
where
    R: BufRead,
    W: Write,
{
    let message = handle_message(reader)?;

    let Message::Request(request) = message else {
        return Err(miette!("First message was not a request"));
    };

    if request.method == "initialize" {
        let (result, params) = process_initialize(request)?;
        let msg = encode_message(&result)?;
        write_msg(writer, &msg)?;
        return Ok(params);
    }

    Err(miette!("First request must be 'initialize'"))
}

/// Handles a typed LSP request by deserializing params, calling handler, and writing response.
/// Called by the `dispatch_lsp_request!` macro.
#[tracing::instrument(skip_all, fields(method = %R::METHOD))]
pub(crate) fn handle_request<R, W, F>(
    lsp: &mut ServerState,
    raw_request: Request,
    writer: &mut W,
    handler: F,
) -> Result<()>
where
    R: LspRequest,
    W: Write,
    F: FnOnce(&mut ServerState, R::Params) -> Result<R::Result>,
{
    let params = match serde_json::from_value::<R::Params>(raw_request.params) {
        Ok(p) => p,
        Err(e) => {
            // If deserialization fails, send an error response.
            let err_msg = format!("Invalid request parameters: {}", e);
            tracing::warn!("{}", err_msg);

            let response =
                Response::from_error(raw_request.id, rpc::error_codes::INVALID_PARAMS, err_msg);
            let msg = encode_message(&response)?;
            write_msg(writer, &msg)?;
            return Ok(());
        }
    };

    let response = match handler(lsp, params) {
        Ok(result) => Response::from_ok(raw_request.id, result),
        Err(err) => {
            tracing::error!("Request failed [method: {}]: {:?}", R::METHOD, err);
            Response::from_error(
                raw_request.id,
                rpc::error_codes::REQUEST_FAILED,
                err.to_string(),
            )
        }
    };

    let msg = encode_message(&response)?;

    write_msg(writer, &msg)?;
    Ok(())
}

#[tracing::instrument(skip_all, fields(method = %R::METHOD))]
pub(crate) fn handle_notification<R, F>(
    lsp: &mut ServerState,
    raw_notification: Notification,
    handler: F,
) -> Result<()>
where
    R: LspNotification,
    F: FnOnce(&mut ServerState, R::Params) -> Result<()>,
{
    let params = match serde_json::from_value::<R::Params>(raw_notification.params) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                "Invalid notification parameters [method: {}]: {}",
                R::METHOD,
                e
            );
            return Ok(());
        }
    };

    if let Err(err) = handler(lsp, params) {
        tracing::error!("Notification failed [method: {}]: {:?}", R::METHOD, err);
        return Err(err);
    }

    Ok(())
}
