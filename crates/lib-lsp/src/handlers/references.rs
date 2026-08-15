use gen_lsp_types::{Location, ReferenceParams};
use miette::{Context, Result};

use crate::{
    get_document, helpers::references::ReferenceCollector, server_state::ServerState,
    text_buffer_conversions::TextBufferConversions, uri::UriExt,
};

pub fn process_references(
    lsp: &mut ServerState,
    params: ReferenceParams,
) -> Result<Option<Vec<Location>>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = get_document!(lsp, &uri);
    let slice = document.source.slice(..);
    let reference_at_position = slice
        .try_position_to_byte_offset(position)
        .and_then(|offset| document.get_reference_at_offset(offset));

    let mut reference_locations = if let Some(reference) = reference_at_position {
        ReferenceCollector::new(document, &uri, reference, lsp).collect_from(&lsp.documents)
    } else {
        ReferenceCollector::collect_file_reference_locations(lsp, &uri)
    };

    // Include the hovered reference itself if requested
    if params.context.include_declaration
        && let Some(reference) = reference_at_position
    {
        let range = slice.byte_to_lsp_range(reference.span());
        let declaration_location = Location::new(uri.clone(), range);
        reference_locations.insert(0, declaration_location);
    }

    Ok(Some(reference_locations))
}

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        Position, ReferenceContext, TextDocumentIdentifier, TextDocumentPositionParams,
        WorkDoneProgressParams,
    };

    use super::*;
    use crate::test_utils::TestWorkspace;

    fn params(uri: &str, line: u32, character: u32, include_declaration: bool) -> ReferenceParams {
        ReferenceParams {
            context: ReferenceContext {
                include_declaration,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: Default::default(),
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: uri.parse().unwrap(),
                },
                position: Position::new(line, character),
            },
        }
    }

    #[test]
    fn finds_links_pointing_at_a_header() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/target.md", 1, "# Section\n\nBody")
            .add_file("/workspace/a.md", 1, "[link](./target.md#Section)")
            .add_file("/workspace/b.md", 1, "[[target#Section]]");

        let locations = process_references(
            &mut ws.state,
            params("file:///workspace/target.md", 0, 2, false),
        )
        .unwrap()
        .unwrap();

        assert_eq!(locations.len(), 2);
        let uris: Vec<String> = locations.iter().map(|l| l.uri.to_string()).collect();
        assert!(uris.contains(&"file:///workspace/a.md".to_string()));
        assert!(uris.contains(&"file:///workspace/b.md".to_string()));
    }

    #[test]
    fn includes_declaration_when_requested() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/target.md", 1, "# Section\n\nBody")
            .add_file("/workspace/a.md", 1, "[link](./target.md#Section)");

        let locations = process_references(
            &mut ws.state,
            params("file:///workspace/target.md", 0, 2, true),
        )
        .unwrap()
        .unwrap();

        assert_eq!(locations.len(), 2);
        assert_eq!(locations[0].uri.to_string(), "file:///workspace/target.md");
    }

    #[test]
    fn finds_whole_file_references_when_cursor_not_on_a_reference() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/target.md", 1, "Body text here")
            .add_file("/workspace/a.md", 1, "[link](./target.md)");

        let locations = process_references(
            &mut ws.state,
            params("file:///workspace/target.md", 0, 2, false),
        )
        .unwrap()
        .unwrap();

        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].uri.to_string(), "file:///workspace/a.md");
    }
}
