use std::{
    fs::File,
    io::{self, Write},
};

use log::{debug, info, warn};
use rust_markdown_lsp::{
    lsp::{
        did_change::process_did_change, did_open::process_did_open, hover::process_hover,
        initialize::process_initialize,
    },
    message::Message,
    rpc::{encode_message, handle_message},
    LspServer,
};
use simplelog::*;

fn main() {
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

    loop {
        let message = handle_message(&mut reader);

        match message {
            Message::Request(request) => {
                let result = match request.method.as_str() {
                    "initialize" => process_initialize(request),
                    "textDocument/hover" => process_hover(&mut lsp, request),
                    _ => {
                        warn!("Unimplemented Request: {}", request.method);
                        continue;
                    }
                };
                let msg = encode_message(result);
                writer.write_all(msg.as_bytes()).unwrap();
                writer.flush().unwrap();
            }
            Message::Notification(notification) => {
                let not = serde_json::to_string_pretty(&notification).unwrap();
                match notification.method.as_str() {
                    "initialized" => {
                        info!("Initialized");
                        continue;
                    }
                    "textDocument/didOpen" => {
                        process_did_open(&mut lsp, notification);
                        continue;
                    }
                    "textDocument/didSave" => {
                        debug!("Did Save: {:#?}", not);
                        continue;
                    }
                    "textDocument/didChange" => {
                        process_did_change(&mut lsp, notification);
                        continue;
                    }
                    "textDocument/didClose" => {
                        continue;
                    }
                    _ => {
                        warn!("Unimplemented Notification: {}", notification.method);
                        continue;
                    }
                };
            }
        }
    }
}
