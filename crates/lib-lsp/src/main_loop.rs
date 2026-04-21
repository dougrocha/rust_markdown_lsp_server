use lsp_types::{
    InitializeParams, error_codes,
    notification::{self, Notification as LspNotification},
    request::{self, Request as LspRequest},
};
use miette::{Result, miette};
use std::io::{self, BufRead, Write};

use crate::{
    Server, dispatch_lsp_request,
    handlers::{
        code_action::process_code_action,
        completion::{completion_resolve::process_completion_resolve, process_completion},
        diagnostics::process_diagnostic,
        did_change::process_did_change,
        did_open::process_did_open,
        document_symbol::process_document_symbol,
        goto_definition::process_goto_definition,
        hover::process_hover,
        initialize::process_initialize,
        references::process_references,
        workspace_symbol::process_workspace_symbol,
    },
    messages::{Message, Notification, Request, Response},
    rpc::{encode_message, handle_message, write_msg},
};
use crate::{dispatch_lsp_notification, rpc};

pub fn run_lsp() -> Result<()> {
    let (stdin, stdout) = (io::stdin(), io::stdout());
    let (mut reader, mut writer) = (stdin.lock(), stdout.lock());

    let mut lsp = Server::default();
    lsp.load_config("rust-markdown-lsp.toml");

    let init_params = handle_initialize(&mut reader, &mut writer)?;
    lsp.load_workspaces(init_params.workspace_folders)?;
    lsp.set_client_capabilities(init_params.capabilities);

    loop {
        match handle_message(&mut reader)? {
            Message::Request(request) => match request.method.as_str() {
                "shutdown" => {
                    tracing::info!("Shutting down");
                    break;
                }
                _ => {
                    dispatch_lsp_request!(&mut lsp, request, &mut writer, {
                        request::HoverRequest => process_hover,
                        request::GotoDefinition => process_goto_definition,
                        request::CodeActionRequest => process_code_action,
                        request::Completion => process_completion,
                        request::ResolveCompletionItem => process_completion_resolve,
                        request::References => process_references,
                        request::DocumentDiagnosticRequest => process_diagnostic,
                        request::DocumentSymbolRequest => process_document_symbol,
                        request::WorkspaceSymbolRequest => process_workspace_symbol,
                    });
                }
            },
            Message::Notification(notification) => {
                tracing::trace!("textDocument/{}", notification.method);

                match notification.method.as_str() {
                    "exit" => {
                        tracing::trace!("Received exit notification");
                    }
                    "initialized" => {
                        tracing::trace!("Initialization confirmed!");
                    }
                    _ => {
                        dispatch_lsp_notification!(&mut lsp, notification, {
                            notification::DidOpenTextDocument => process_did_open,
                            notification::DidChangeTextDocument => process_did_change,
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
#[tracing::instrument(skip_all, fields(method = R::METHOD))]
pub(crate) fn handle_request<R, W, F>(
    lsp: &mut Server,
    raw_request: Request,
    writer: &mut W,
    handler: F,
) -> Result<()>
where
    R: LspRequest,
    W: Write,
    F: FnOnce(&mut Server, R::Params) -> Result<R::Result>,
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
            Response::from_error(raw_request.id, error_codes::REQUEST_FAILED, err.to_string())
        }
    };

    let msg = encode_message(&response)?;

    write_msg(writer, &msg)?;
    Ok(())
}

#[tracing::instrument(skip_all, fields(method = R::METHOD))]
pub(crate) fn handle_notification<R, F>(
    lsp: &mut Server,
    raw_notification: Notification,
    handler: F,
) -> Result<()>
where
    R: LspNotification,
    F: FnOnce(&mut Server, R::Params) -> Result<()>,
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
