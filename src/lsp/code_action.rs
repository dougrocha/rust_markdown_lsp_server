use std::{collections::HashMap, str::FromStr};

use lsp_types::{
    error_codes, ChangeAnnotation, CodeAction, CodeActionKind, CodeActionOrCommand,
    CodeActionParams, CodeActionResponse, Command, CreateFile, CreateFileOptions,
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    Position, Range, ResourceOp, TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};
use miette::{miette, Context, IntoDiagnostic, Result};

use crate::{
    document::references::{LinkHeader, Reference},
    lsp::{helpers::extract_header_section, server::LspServer},
    message::{Request, Response},
    path::get_parent_path,
};

pub fn process_code_action(lsp: &mut LspServer, request: Request) -> Response {
    match process_code_action_internal(lsp, &request) {
        Ok(result) => Response::from_ok(request.id, result),
        Err(e) => Response::from_error(request.id, error_codes::REQUEST_FAILED, e.to_string()),
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
    if range.start == range.end {
        return handle_non_range(lsp, &uri, &range);
    }

    let actions: Vec<CodeActionOrCommand> = Vec::new();

    Ok(actions)
}

fn handle_non_range(lsp: &mut LspServer, uri: &Uri, range: &Range) -> Result<CodeActionResponse> {
    let document = lsp
        .get_document(uri)
        .context("Document should exist somewhere")?;
    let slice = document.content.slice(..);

    let Some(reference) = document.find_reference_at_position(range.start) else {
        return Ok(vec![]);
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

            let parent = get_parent_path(uri).unwrap();
            let new_file_uri = Uri::from_str(&format!(
                "file://{}/{}.md",
                parent,
                (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    % 1000000) as u32
            ))
            .into_diagnostic()
            .context("New document should be valid path")?;

            if let Some(header_content) = header_content {
                let document_changes = DocumentChanges::Operations(vec![
                    DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                        uri: new_file_uri.clone(),
                        options: None,
                        annotation_id: None,
                    })),
                    DocumentChangeOperation::Edit(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            uri: new_file_uri,
                            version: None,
                        },
                        edits: vec![OneOf::Left(TextEdit::new(
                            Range::new(
                                Position::new(0, 0),
                                Position::new(slice.lines().count() as u32, 0),
                            ),
                            header_content.to_string(),
                        ))],
                    }),
                    DocumentChangeOperation::Edit(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            uri: uri.clone(),
                            version: None,
                        },
                        edits: vec![OneOf::Left(TextEdit::new(
                            document.span_to_range(&range),
                            // TODO: Change this from empty to link to new file and link
                            "".to_string(),
                        ))],
                    }),
                ]);

                let workspace_edit = WorkspaceEdit {
                    changes: None,
                    document_changes: Some(document_changes),
                    change_annotations: None,
                };

                actions.push(
                    CodeAction {
                        title: "Extract header & section".to_string(),
                        kind: Some(CodeActionKind::REFACTOR_EXTRACT),
                        edit: Some(workspace_edit),
                        command: Some(Command {
                            title: "Save new file".to_string(),
                            command: "rustMarkdown.saveFile".to_string(),
                            arguments: Some(vec![serde_json::json!(format!(
                                "/Users/douglasrocha/dev/rust_markdown_lsp/{}.md",
                                content.to_string()
                            ))]),
                        }),
                        ..Default::default()
                    }
                    .into(),
                );
            }
        }
        _ => return Err(miette!("Other cases not handled yet.")),
    }

    Ok(actions)
}
