use std::{
    fs::File,
    io::{self, Write},
};

use log::{debug, error, info, warn};
use rust_markdown_lsp::{
    lsp::{
        did_change::process_did_change,
        did_open::process_did_open,
        hover::process_hover,
        initialize::{process_initialize, InitializeParams},
    },
    message::Message,
    rpc::{encode_message, handle_message, write_msg},
    LspServer,
};
use simplelog::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = WriteLogger::init(
        LevelFilter::max(),
        Config::default(),
        File::create("log.txt").expect("Failed to create log file"),
    );

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

fn load_workspaces(
    lsp: &mut LspServer,
    init_params: InitializeParams,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(folders) = init_params.workspace_folders {
        let folder = folders.first().ok_or("Workspace folder does not exist")?;
        lsp.set_root(folder.uri.clone());

        let path = folder.uri.as_str().trim_start_matches("file://");
        let markdowns = walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .map(|e| e.path().to_string_lossy().to_string())
            .collect::<Vec<String>>();

        for md_file in markdowns {
            let text = std::fs::read_to_string(&md_file)?;
            lsp.open_document(&md_file, &text);
        }
    }

    Ok(())
}

fn handle_initialize<R: io::BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
) -> Result<InitializeParams, Box<dyn std::error::Error>> {
    let message = handle_message(reader)?;

    if let Message::Request(request) = message {
        if request.method == "initialize" {
            let (result, params) = process_initialize(request);
            let msg = encode_message(&result)?;
            writer.write_all(msg.as_bytes())?;
            writer.flush()?;
            return Ok(params);
        }

        return Err("First request MUST be initialize".into());
    };

    Err("First message was not a request".into())
}

fn handle_request<W: Write>(
    lsp: &mut LspServer,
    request: rust_markdown_lsp::message::Request,
    writer: &mut W,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = match request.method.as_str() {
        "textDocument/hover" => process_hover(lsp, request),
        _ => {
            warn!("Unimplemented Request: {}", request.method);
            return Ok(());
        }
    };
    let msg = encode_message(&result)?;
    write_msg(writer, &msg)?;
    Ok(())
}

fn handle_notification(
    lsp: &mut LspServer,
    notification: rust_markdown_lsp::message::Notification,
) {
    debug!("textDocument/{:?}", notification.method);
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
