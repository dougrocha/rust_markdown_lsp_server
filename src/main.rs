use log::{debug, error, info, warn};
use lsp_types::{
    error_codes,
    request::{self, Request as LspRequest},
    InitializeParams, Uri,
};
use miette::{miette, Context, IntoDiagnostic, Result};
use rust_markdown_lsp::{
    dispatch_lsp_request,
    lsp::{
        code_action::process_code_action,
        completion::{process_completion, process_completion_resolve},
        did_change::process_did_change,
        did_open::process_did_open,
        goto_definition::process_goto_definition,
        hover::process_hover,
        initialize::process_initialize,
        server::LspServer,
    },
    message::{Message, Request, Response},
    rpc::{encode_message, handle_message, write_msg},
};
use simplelog::*;
use std::{
    fs::File,
    io::{self, Write},
    str::FromStr,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = WriteLogger::init(
        LevelFilter::max(),
        Config::default(),
        File::create("log.txt").expect("Failed to create log file"),
    );

    // let output = std::fs::read_to_string("test.md").unwrap();
    // let (output, errors) = markdown_parser().parse(&output).into_output_errors();
    // println!("Parsed frontmatter: {:#?}", output.unwrap().frontmatter);
    // println!("Parsed errors: {:#?}", errors);
    //
    // return Ok(());

    let stdin = io::stdin();
    let stdout = io::stdout();

    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    let mut lsp = LspServer::new();

    let init_params = handle_initialize(&mut reader, &mut writer)?;

    load_workspaces(&mut lsp, init_params)?;

    loop {
        match handle_message(&mut reader) {
            Ok(message) => match message {
                Message::Request(request) => match request.method.as_str() {
                    "shutdown" => {
                        log::info!("Shutting down");
                        break;
                    }
                    _ => {
                        dispatch_lsp_request!(&mut lsp, request, &mut writer, {
                            request::HoverRequest => process_hover,
                            request::GotoDefinition => process_goto_definition,
                            request::CodeActionRequest => process_code_action,
                        });
                    }
                },
                Message::Notification(notification) => {
                    handle_notification(&mut lsp, notification);
                }
            },
            Err(e) => {
                error!("Error handling message: {}", e);
            }
        }
    }

    Ok(())
}

fn load_workspaces(lsp: &mut LspServer, init_params: InitializeParams) -> Result<()> {
    let Some(folders) = init_params.workspace_folders else {
        return Ok(());
    };

    let folder = folders
        .first()
        .context("No workspace folders provided by the client")?;
    let uri = &folder.uri;
    lsp.set_root(uri.clone());

    let path = uri.path();
    let markdown_files = walkdir::WalkDir::new(path.as_str())
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "md"));

    for entry in markdown_files {
        let path_str = entry.path().to_string_lossy().to_string();
        let contents = std::fs::read_to_string(&entry.path())
            .into_diagnostic()
            .with_context(|| format!("Failed to read markdown file: {}", path_str))?;

        let uri = Uri::from_str(&path_str)
            .into_diagnostic()
            .with_context(|| format!("Invalid URI from path: {}", path_str))?;

        lsp.open_document(uri, &contents);
    }

    Ok(())
}

fn handle_initialize<R: io::BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
) -> Result<InitializeParams> {
    let message = handle_message(reader)?;

    if let Message::Request(request) = message {
        if request.method == "initialize" {
            let (result, params) = process_initialize(request)?;
            let msg = encode_message(&result)?;
            write_msg(writer, &msg)?;
            return Ok(params);
        }

        return Err(miette!("First request must be 'initialize'"));
    }

    Err(miette!("First message was not a request"))
}

fn handle_request<R, W, F>(
    lsp: &mut LspServer,
    raw_request: Request,
    writer: &mut W,
    handler: F,
) -> Result<()>
where
    R: LspRequest,
    W: Write,
    F: FnOnce(&mut LspServer, R::Params) -> Result<R::Result>,
{
    let params: R::Params = serde_json::from_value(raw_request.params)
        .into_diagnostic()
        .context("Failed to deserialize request params")?;

    let response = match handler(lsp, params) {
        Ok(result) => Response::from_ok(raw_request.id, result),
        Err(err) => {
            Response::from_error(raw_request.id, error_codes::REQUEST_FAILED, err.to_string())
        }
    };

    let msg = encode_message(&response)?;

    log::debug!("{:#?}", msg);
    write_msg(writer, &msg)?;
    Ok(())
}

fn handle_notification(
    lsp: &mut LspServer,
    notification: rust_markdown_lsp::message::Notification,
) {
    debug!("textDocument/{}", notification.method);
    match notification.method.as_str() {
        "initialized" => {
            info!("Initialized");
        }
        "textDocument/didOpen" => {
            process_did_open(lsp, notification);
        }
        "textDocument/didSave" => {
            serde_json::to_string_pretty(&notification).unwrap();
        }
        "textDocument/didChange" => {
            process_did_change(lsp, notification);
        }
        "textDocument/didClose" => {}
        _ => {
            warn!("Unimplemented Notification: {}", notification.method);
        }
    };
}
