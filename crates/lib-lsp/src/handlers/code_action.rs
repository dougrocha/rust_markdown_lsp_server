use gen_lsp_types::{
    CodeAction, CodeActionKind, CodeActionParams, CodeActionResponse, CreateFile, DeleteFile,
    DeleteFileOptions, DocumentChange, Edit, OptionalVersionedTextDocumentIdentifier, Position,
    Range, TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};
use lib_core::document::references::ReferenceKind;
use miette::{Context, Result, miette};

use crate::{
    get_document,
    handlers::link_resolver::resolve_target_uri,
    helpers::{extract_header_section, generate_link_text, get_content},
    server_state::ServerState,
    uri::UriExt,
};

pub fn process_code_action(
    lsp: &mut ServerState,
    params: CodeActionParams,
) -> Result<Option<Vec<CodeActionResponse>>> {
    let uri = params.text_document.uri;
    let range = params.range;

    // If range is not given check if cursor in over a header
    if range.start == range.end {
        return handle_non_range(lsp, &uri, &range);
    }

    let actions: Vec<CodeActionResponse> = Vec::new();

    Ok(Some(actions))
}

fn handle_non_range(
    lsp: &mut ServerState,
    uri: &Uri,
    range: &Range,
) -> Result<Option<Vec<CodeActionResponse>>> {
    let document = get_document!(lsp, uri);
    let slice = document.content.slice(..);

    let source_root = lsp.get_workspace_root_for_path(&document.path);

    let Some(reference) = document.get_reference_at_position(range.start) else {
        return Ok(Some(vec![]));
    };

    let doc_parent_path = document
        .path
        .parent()
        .ok_or_else(|| miette!("Could not determine parent directory"))?;

    let mut actions: Vec<CodeActionResponse> = Vec::new();

    match &reference.kind {
        ReferenceKind::Header { content, level } => {
            let (header_content, range) =
                extract_header_section(content, &document.references, slice);
            let delta = 1i32 - *level as i32;

            let new_filename = format!(
                "{}.md",
                (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    % 1000000) as u32
            );

            let new_file_path = doc_parent_path.join(new_filename);
            let new_file_uri = Uri::from_file_path(new_file_path)
                .context("new document should be a valid path")?;

            if let Some(header_content) = header_content {
                let document_changes = vec![
                    DocumentChange::CreateFile(CreateFile {
                        uri: new_file_uri.clone(),
                        options: None,
                        annotation_id: None,
                    }),
                    DocumentChange::TextDocumentEdit(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            text_document_identifier: gen_lsp_types::TextDocumentIdentifier {
                                uri: new_file_uri.clone(),
                            },
                            version: None,
                        },
                        edits: vec![Edit::TextEdit(TextEdit::new(
                            Range::new(Position::new(0, 0), Position::new(0, 0)),
                            normalize_header_levels(&header_content.to_string(), delta),
                        ))],
                    }),
                    DocumentChange::TextDocumentEdit(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            text_document_identifier: gen_lsp_types::TextDocumentIdentifier {
                                uri: uri.clone(),
                            },
                            version: Some(document.version),
                        },
                        edits: vec![Edit::TextEdit(TextEdit::new(range, {
                            let link_text = generate_link_text(
                                &lsp.config.links,
                                uri,
                                &new_file_uri,
                                source_root,
                            )
                            .unwrap_or_else(|_| new_file_uri.as_ref().to_string());

                            format!("[{content}]({link_text})\n\n")
                        }))],
                    }),
                ];

                let workspace_edit = WorkspaceEdit {
                    changes: None,
                    document_changes: Some(document_changes),
                    change_annotations: None,
                };

                actions.push(CodeActionResponse::CodeAction(CodeAction {
                    title: "Extract header & section".to_owned(),
                    kind: Some(CodeActionKind::RefactorExtract),
                    edit: Some(workspace_edit),
                    ..Default::default()
                }));
            }
        }
        ReferenceKind::Link { target, header, .. }
        | ReferenceKind::WikiLink { target, header, .. } => {
            let target_uri = resolve_target_uri(lsp, document, target)?;

            // TODO: normalize later
            let target_doc_content = get_content(lsp, document, target, header.as_deref())?;

            let document_changes = vec![
                DocumentChange::TextDocumentEdit(TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier {
                        text_document_identifier: gen_lsp_types::TextDocumentIdentifier {
                            uri: uri.clone(),
                        },
                        version: Some(document.version),
                    },
                    edits: vec![Edit::TextEdit(TextEdit::new(
                        reference.range,
                        target_doc_content,
                    ))],
                }),
                // TODO: This is okay if our link does not include a header,
                // if the link is link#header and header is a small section of the file
                // you see what i mean?
                DocumentChange::DeleteFile(DeleteFile {
                    uri: target_uri,
                    options: Some(DeleteFileOptions {
                        ignore_if_not_exists: Some(false),
                        recursive: None,
                    }),
                    annotation_id: None,
                }),
            ];

            let workspace_edit = WorkspaceEdit {
                changes: None,
                document_changes: Some(document_changes),
                change_annotations: None,
            };

            actions.push(CodeActionResponse::CodeAction(CodeAction {
                title: "Inline section".to_owned(),
                kind: Some(CodeActionKind::RefactorInline),
                edit: Some(workspace_edit),
                ..Default::default()
            }));
        }
    }

    Ok(Some(actions))
}

fn normalize_header_levels(content: &str, delta: i32) -> String {
    content
        .split('\n')
        .map(|line| {
            let hashes = line.chars().take_while(|c| *c == '#').count();
            if hashes == 0 || !line[hashes..].starts_with(' ') {
                return line.to_string();
            }
            let new_level = (hashes as i32 + delta).max(1) as usize;
            format!("{} {}", "#".repeat(new_level), &line[hashes + 1..])
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_already_h1() {
        let content = "# Title\n\nSome text.";
        assert_eq!(normalize_header_levels(content, 0), content);
    }

    #[test]
    fn test_normalize_h3_to_h1() {
        let input = "### My Section\n\nParagraph.\n\n#### Sub\n\nMore.";
        let expected = "# My Section\n\nParagraph.\n\n## Sub\n\nMore.";
        assert_eq!(normalize_header_levels(input, -2), expected);
    }

    #[test]
    fn test_normalize_clamps_at_h1() {
        // H2 with delta -5 should clamp to H1, not go negative
        let input = "## Section\n\n### Child";
        let expected = "# Section\n\n# Child";
        assert_eq!(normalize_header_levels(input, -5), expected);
    }

    #[test]
    fn test_normalize_ignores_non_headers() {
        let input = "# Title\n\nThis has a #hashtag in it.\n\n## Sub";
        let expected = "# Title\n\nThis has a #hashtag in it.\n\n# Sub";
        assert_eq!(normalize_header_levels(input, -1), expected);
    }
}
