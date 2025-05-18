use std::{
    fs::File,
    io::{self, Write},
    str::FromStr,
};

use log::{debug, error, info, warn};
use lsp_types::{InitializeParams, Uri};
use miette::{miette, Context, IntoDiagnostic, Result};
use parser::{markdown_parser, Parser};
use rust_markdown_lsp::{
    lsp::{
        code_action::process_code_action, did_change::process_did_change,
        did_open::process_did_open, goto_definition::process_goto_definition, hover::process_hover,
        initialize::process_initialize, server::LspServer,
    },
    message::Message,
    rpc::{encode_message, handle_message, write_msg},
};
use simplelog::*;

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
                        if let Err(e) = handle_request(&mut lsp, request, &mut writer) {
                            error!("Error handling request: {}", e)
                        }
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
            let (result, params) = process_initialize(request);
            let msg = encode_message(&result)?;
            write_msg(writer, &msg)?;
            return Ok(params);
        }

        return Err(miette!("First request must be 'initialize'"));
    }

    Err(miette!("First message was not a request"))
}

fn handle_request<W: Write>(
    lsp: &mut LspServer,
    request: rust_markdown_lsp::message::Request,
    writer: &mut W,
) -> Result<()> {
    debug!("Handling request: {}", request.method);
    let result = match request.method.as_str() {
        "textDocument/hover" => process_hover(lsp, request),
        "textDocument/definition" => process_goto_definition(lsp, request),
        "textDocument/codeAction" => process_code_action(lsp, request),
        _ => {
            warn!("Unimplemented Request: {}", request.method);
            return Ok(());
        }
    };
    let msg = encode_message(&result)?;

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
