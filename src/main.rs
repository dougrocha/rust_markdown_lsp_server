use std::{
    fs::File,
    io::{self, Write},
    str::FromStr,
    sync::{Arc, Mutex},
};

use log::{debug, error, info, warn};
use lsp_types::{
    error_codes,
    request::{self, Request as LspRequest},
    InitializeParams, Uri,
};
use miette::{miette, Context, IntoDiagnostic, Result};
use parser::Parser;
use rust_markdown_lsp::{
    dispatch_lsp_request,
    lsp::{
        code_action::process_code_action,
        completion::{process_completion, process_completion_resolve},
        diagnostics::process_diagnostic,
        did_change::process_did_change,
        did_open::process_did_open,
        goto_definition::process_goto_definition,
        hover::process_hover,
        initialize::process_initialize,
        state::LspState,
    },
    message::{Message, Request, Response},
    rpc::{encode_message, handle_message, write_msg},
    UriExt,
};
use simplelog::*;

fn main() -> Result<()> {
    let _ = WriteLogger::init(
        LevelFilter::max(),
        Config::default(),
        File::create("log.txt").expect("Failed to create log file"),
    );

    // let test_file = std::fs::read_to_string("test.md").unwrap();
    // let (output, errors) = parser::markdown_parser()
    //     .parse(&test_file)
    //     .into_output_errors();
    // println!(
    //     "Parsed frontmatter: {:#?}",
    //     output.clone().unwrap().frontmatter
    // );
    // let document = rust_markdown_lsp::document::Document::new(
    //     Uri::from_str("./test.md").unwrap(),
    //     &test_file,
    //     0,
    // )?;
    // for err in errors {
    //     println!(
    //         "{:#?} - {:?}",
    //         err,
    //         document.byte_to_lsp_range(&err.span().into_range())
    //     );
    // }
    //

    // for el in output.unwrap().body {
    //     match el.0 {
    //         parser::MarkdownNode::Header { level, content: _ } => {
    //             println!(
    //                 "Header: \t{:?}\n \tSpan: {:?}\n \t{:?}\n",
    //                 level,
    //                 el.1,
    //                 document.byte_to_lsp_range(&el.1.into_range())
    //             );
    //         }
    //         parser::MarkdownNode::Paragraph(spanneds) => {
    //             for Spanned(inel, span) in spanneds {
    //                 match inel {
    //                     parser::InlineMarkdownNode::Link(link_type) => {
    //                         println!(
    //                             "Link: \t{:?}\n \tSpan: {:?}\n \t{:?}\n",
    //                             link_type,
    //                             span,
    //                             document.byte_to_lsp_range(&span.into_range())
    //                         );
    //                     }
    //                     _ => {}
    //                 }
    //             }
    //         }
    //         _ => {}
    //     }
    // }
    //
    // return Ok(());

    let stdin = io::stdin();
    let stdout = io::stdout();

    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    let lsp = Arc::new(Mutex::new(LspState::default()));

    let init_params = handle_initialize(&mut reader, &mut writer)?;

    let mut lsp = lsp.lock().unwrap();
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
                            request::Completion => process_completion,
                            request::ResolveCompletionItem => process_completion_resolve,
                            request::DocumentDiagnosticRequest => process_diagnostic,
                        });
                    }
                },
                Message::Notification(notification) => {
                    handle_notification(&mut lsp, notification)?;
                }
            },
            Err(e) => {
                error!("Error handling message: {}", e);
            }
        }
    }

    Ok(())
}

fn load_workspaces(lsp: &mut LspState, init_params: InitializeParams) -> Result<()> {
    let Some(folders) = init_params.workspace_folders else {
        return Err(miette!("Workspaces were not provided."));
    };

    for folder in folders {
        let uri = &folder.uri;
        lsp.set_root(uri.clone());

        let path = uri.path();
        let markdown_files = walkdir::WalkDir::new(path.as_str())
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"));

        for entry in markdown_files {
            let entry_path = entry.path();
            let contents = std::fs::read_to_string(entry_path)
                .into_diagnostic()
                .with_context(|| format!("Failed to read markdown file: {:?}", entry_path))?;

            let uri = Uri::from_file_path(entry_path)
                .with_context(|| format!("Failed to create URI from path: {:?}", entry_path))?;

            lsp.documents.open_document(uri, 0, &contents)?;
        }
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
    lsp: &mut LspState,
    raw_request: Request,
    writer: &mut W,
    handler: F,
) -> Result<()>
where
    R: LspRequest,
    W: Write,
    F: FnOnce(&mut LspState, R::Params) -> Result<R::Result>,
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

    write_msg(writer, &msg)?;
    Ok(())
}

fn handle_notification(
    lsp: &mut LspState,
    notification: rust_markdown_lsp::message::Notification,
) -> Result<()> {
    debug!("textDocument/{}", notification.method);
    match notification.method.as_str() {
        "initialized" => {
            info!("Initialized");
        }
        "textDocument/didOpen" => {
            process_did_open(lsp, notification)?;
        }
        "textDocument/didSave" => {
            serde_json::to_string_pretty(&notification).unwrap();
        }
        "textDocument/didChange" => {
            process_did_change(lsp, notification)?;
        }
        "textDocument/didClose" => {}
        _ => {
            warn!("Unimplemented Notification: {}", notification.method);
        }
    };

    Ok(())
}
