use lsp_types::{
    code_action::{
        CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse,
    },
    uri::URI,
    workspace::{
        CreateFile, DocumentChanges, OptionalVersionedTextDocumentIdentifier, ResourceOp,
        TextDocumentEdit, TextEdit, WorkspaceEdit,
    },
    Command, Position, Range,
};
use miette::{Context, IntoDiagnostic, Result};

use crate::{
    document::references::{LinkHeader, Reference},
    lsp::server::LspServer,
    message::{error_codes, Request, Response},
};

use super::helpers::extract_header_section;

pub fn process_code_action(lsp: &mut LspServer, request: Request) -> Response {
    match process_code_action_internal(lsp, &request) {
        Ok(result) => Response::new(request.id, result),
        Err(e) => Response::error(request.id, error_codes::INTERNAL_ERROR, e.to_string()),
    }
}

fn process_code_action_internal(
    lsp: &mut LspServer,
    request: &Request,
) -> Result<CodeActionResponse> {
    let params: CodeActionParams = serde_json::from_value(request.params.clone())
        .into_diagnostic()
        .context("Failed to parse code action params")?;

    let uri = params.text_document.uri;
    let range = params.range;

    // If range is not given check if cursor in over a header
    if !range.is_range() {
        return handle_non_range(lsp, &uri, &range);
    }

    let actions: Vec<CodeActionOrCommand> = Vec::new();

    Ok(Some(actions))
}

fn handle_non_range(lsp: &mut LspServer, uri: &URI, range: &Range) -> Result<CodeActionResponse> {
    let document = lsp
        .get_document(uri)
        .context("Document should exist somewhere")?;
    let slice = document.content.slice(..);

    let Some(reference) = document.find_reference_at_position(range.start) else {
        return Ok(None);
    };

    let mut actions: Vec<CodeActionOrCommand> = Vec::new();
    match reference {
        Reference::Header { level, content, .. } => {
            let (header_content, range) = extract_header_section(
                &LinkHeader {
                    level: *level,
                    content: content.to_string(),
                },
                &document.references,
                slice,
            );
            if let Some(header_content) = header_content {
                let document_changes = DocumentChanges::Operations(vec![
                    TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier::new(
                            uri.clone(),
                            None,
                        ),
                        edits: vec![TextEdit {
                            range: document.span_to_range(&range),
                            new_text: "".to_string(),
                        }],
                    }
                    .into(),
                    ResourceOp::Create(CreateFile {
                        uri: URI(format!(
                            "/Users/douglasrocha/dev/rust_markdown_lsp/{}.md",
                            content.to_string()
                        )),
                        options: None,
                        annotation_id: None,
                    })
                    .into(),
                    TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier::new(
                            URI(format!(
                                "/Users/douglasrocha/dev/rust_markdown_lsp/{}.md",
                                content.to_string()
                            )),
                            None,
                        ),
                        edits: vec![TextEdit {
                            range: Range {
                                start: Position {
                                    line: 0,
                                    character: 0,
                                },
                                end: Position {
                                    line: 0,
                                    character: 0,
                                },
                            },
                            new_text: header_content.to_string(),
                        }],
                    }
                    .into(),
                ]);

                let workspace_edit = WorkspaceEdit {
                    changes: None,
                    document_changes: Some(document_changes),
                    change_annotations: None,
                };

                actions.push(
                    CodeAction {
                        title: "Extract header & section".to_string(),
                        kind: Some(CodeActionKind::RefactorExtract),
                        edit: Some(workspace_edit),
                        command: Some(Command {
                            title: "Save new file".to_string(),
                            command: "rustMarkdown.saveFile".to_string(),
                            arguments: Some(vec![serde_json::json!(format!(
                                "/Users/douglasrocha/dev/rust_markdown_lsp/{}.md",
                                content.to_string()
                            ))]),
                        }),
                    }
                    .into(),
                );
            }
        }
        _ => todo!("Handle other cases for code actions"),
    }

    Ok(Some(actions))
}
