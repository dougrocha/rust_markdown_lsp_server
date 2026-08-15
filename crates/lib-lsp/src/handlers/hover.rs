use lib_core::document::Reference;

use gen_lsp_types::{Contents, Hover, HoverParams, MarkupContent, MarkupKind};
use miette::{Context, Result};
use tracing::debug;

use crate::{
    get_document, helpers::get_content, server_state::ServerState,
    text_buffer_conversions::TextBufferConversions, uri::UriExt,
};

pub fn process_hover(lsp: &mut ServerState, params: HoverParams) -> Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    debug!("process_hover: uri={:?}, position={:?}", uri, position);

    let document = get_document!(lsp, &uri);

    let Some(offset) = document
        .source
        .slice(..)
        .try_position_to_byte_offset(position)
    else {
        return Ok(None);
    };

    let reference = document.get_reference_at_offset(offset);

    match reference {
        Some(Reference::Link(link)) => {
            let target = link.target_str(&document.source);
            let header = link.header_str(&document.source);

            debug!("Found link: target={}, header={:?}", target, header);

            let range = document
                .source
                .slice(..)
                .byte_to_lsp_range(link.span);

            let contents = get_content(lsp, document, &target, header.as_deref())?;

            Ok(Some(Hover {
                contents: Contents::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: contents,
                }),
                range: Some(range),
            }))
        }
        Some(Reference::Header(_)) => Ok(None),
        Some(Reference::FootnoteDef(_)) => Ok(None),
        Some(Reference::FootnoteRef(footnote_ref)) => {
            let identifier = footnote_ref.identifier_str(&document.source);

            let Some(def) = document.find_footnote_definition(&identifier) else {
                return Ok(None);
            };

            let content = def.content_str(&document.source);
            let range = document
                .source
                .slice(..)
                .byte_to_lsp_range(footnote_ref.span);

            Ok(Some(Hover {
                contents: Contents::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: content.to_string(),
                }),
                range: Some(range),
            }))
        }
        None => {
            debug!("No reference found at position {:?}", position);
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        Contents, Position, TextDocumentIdentifier, TextDocumentPositionParams,
        WorkDoneProgressParams,
    };

    use super::*;
    use crate::test_utils::TestWorkspace;

    #[test]
    fn hovers_footnote_reference_with_definition_content() {
        let mut ws = TestWorkspace::new();
        ws.add_file(
            "/workspace/notes.md",
            1,
            "See[^note] here\n\n[^note]: the footnote text",
        );

        let params = HoverParams {
            work_done_progress_params: WorkDoneProgressParams::default(),
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "file:///workspace/notes.md".parse().unwrap(),
                },
                position: Position::new(0, 5),
            },
        };

        let result = process_hover(&mut ws.state, params).unwrap().unwrap();
        let Contents::MarkupContent(content) = result.contents else {
            panic!("expected markup content");
        };
        assert_eq!(content.value, "the footnote text");
    }
}
