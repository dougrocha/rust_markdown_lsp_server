use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse, Command,
    CreateFile, DocumentChangeOperation, DocumentChanges, OneOf,
    OptionalVersionedTextDocumentIdentifier, Position, Range, ResourceOp, TextDocumentEdit,
    TextEdit, Uri, WorkspaceEdit,
};
use miette::{miette, Context, Result};

use crate::{
    document::references::{ReferenceKind, TargetHeader},
    get_document,
    lsp::{helpers::extract_header_section, server::Server},
    path::get_parent_path,
    UriExt,
};

pub fn process_code_action(
    lsp: &mut Server,
    params: CodeActionParams,
) -> Result<Option<CodeActionResponse>> {
    let uri = params.text_document.uri;
    let range = params.range;

    // If range is not given check if cursor in over a header
    if range.start == range.end {
        return handle_non_range(lsp, &uri, &range);
    }

    let actions: Vec<CodeActionOrCommand> = Vec::new();

    Ok(Some(actions))
}

fn handle_non_range(
    lsp: &mut Server,
    uri: &Uri,
    range: &Range,
) -> Result<Option<CodeActionResponse>> {
    let document = get_document!(lsp, uri);
    let slice = document.content.slice(..);

    let Some(reference) = document.get_reference_at_position(range.start) else {
        return Ok(Some(vec![]));
    };

    let mut actions: Vec<CodeActionOrCommand> = Vec::new();
    match &reference.kind {
        ReferenceKind::Header { level, content, .. } => {
            let (header_content, range) = extract_header_section(
                &TargetHeader {
                    level: *level,
                    content: content.to_owned(),
                },
                &document.references,
                slice,
            );

            let parent = get_parent_path(uri).unwrap();
            let new_file_uri = Uri::from_file_path(format!(
                "{}/{}.md",
                parent,
                (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    % 1000000) as u32
            ))
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
                            range,
                            // TODO: Change this from empty to link to new file and link
                            "".to_owned(),
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
                        title: "Extract header & section".to_owned(),
                        kind: Some(CodeActionKind::REFACTOR_EXTRACT),
                        edit: Some(workspace_edit),
                        command: Some(Command {
                            title: "Save new file".to_owned(),
                            command: "rustMarkdown.saveFile".to_owned(),
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

    Ok(Some(actions))
}
