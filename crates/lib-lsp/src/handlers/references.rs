use std::path::Path;

use gen_lsp_types::{Location, ReferenceParams, Uri};
use miette::{Context, Result};

use lib_core::{
    document::{Document, Reference},
    path::slug::header_slug,
    resolver::{AnchorMatch, ReferenceTarget, VaultContext, find_link_references},
};

use crate::{
    get_document, server_state::ServerState,
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

    let cx = lsp.vault_context();

    let mut reference_locations = match reference_at_position {
        Some(reference @ (Reference::FootnoteRef(_) | Reference::FootnoteDef(_))) => {
            footnote_locations(document, &uri, reference)
        }
        Some(reference) => match ReferenceTarget::from_reference(reference, document, &cx) {
            Some(target) => {
                link_reference_locations(&cx, &target, Some((&document.path, reference)))
            }
            None => Vec::new(),
        },
        None => {
            let Some(path) = uri.to_file_path() else {
                return Ok(Some(Vec::new()));
            };
            let target = ReferenceTarget {
                file: path.into_owned(),
                anchor: AnchorMatch::Any,
            };
            link_reference_locations(&cx, &target, None)
        }
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

/// Collect the location of every link that points at `target`, minus the cursor's own link.
fn link_reference_locations(
    cx: &VaultContext<'_>,
    target: &ReferenceTarget,
    source: Option<(&Path, Reference<'_>)>,
) -> Vec<Location> {
    let source_span = source.map(|(_, reference)| reference.span());
    let source_path = source.map(|(path, _)| path);

    let mut locations: Vec<Location> = find_link_references(target, cx)
        .filter(|link_ref| {
            source_path != Some(link_ref.document.path.as_path())
                || source_span != Some(link_ref.link.span)
        })
        .filter_map(|link_ref| {
            let uri = Uri::from_file_path(&link_ref.document.path)?;
            let range = link_ref
                .document
                .source
                .slice(..)
                .byte_to_lsp_range(link_ref.link.span);
            Some(Location::new(uri, range))
        })
        .collect();

    // A link with an anchor also references the target heading itself.
    if let (Some((_, Reference::Link(_))), AnchorMatch::Slug(slug)) = (source, &target.anchor)
        && let Some(doc) = cx.vault.get_document(&target.file)
        && let Some(header) = doc
            .headers()
            .find(|h| header_slug(&h.content_str(&doc.source)) == header_slug(slug))
        && let Some(uri) = Uri::from_file_path(&doc.path)
    {
        let range = doc.source.slice(..).byte_to_lsp_range(header.span);
        locations.push(Location::new(uri, range));
    }

    locations
}

/// Collect the other document-local footnotes with the same identifier as `source`.
fn footnote_locations(doc: &Document, uri: &Uri, source: Reference<'_>) -> Vec<Location> {
    let identifier = match source {
        Reference::FootnoteRef(footnote) => footnote.identifier_str(&doc.source),
        Reference::FootnoteDef(footnote) => footnote.identifier_str(&doc.source),
        _ => return Vec::new(),
    };
    let source_span = source.span();
    let slice = doc.source.slice(..);

    let refs = doc
        .footnote_references()
        .filter(|footnote| footnote.span != source_span)
        .filter(|footnote| footnote.identifier_str(&doc.source) == identifier)
        .map(|footnote| Location::new(uri.clone(), slice.byte_to_lsp_range(footnote.span)));
    let defs = doc
        .footnote_definitions()
        .filter(|footnote| footnote.span != source_span)
        .filter(|footnote| footnote.identifier_str(&doc.source) == identifier)
        .map(|footnote| Location::new(uri.clone(), slice.byte_to_lsp_range(footnote.span)));

    refs.chain(defs).collect()
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
    fn references_on_a_link_exclude_the_cursor_link() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/target.md", 1, "# T\n\nbody")
            .add_file("/workspace/a.md", 1, "[[target]] and [[target]]")
            .add_file("/workspace/b.md", 1, "[link](./target.md)");

        // count locations that start at `char` on line 0 of `uri`
        let at = |locs: &[Location], uri: &str, char: u32| {
            locs.iter()
                .filter(|l| {
                    l.uri.as_str() == uri
                        && l.range.start.line == 0
                        && l.range.start.character == char
                })
                .count()
        };

        // cursor on the first `[[target]]` in a.md (bytes 0..10)
        let without_decl = process_references(
            &mut ws.state,
            params("file:///workspace/a.md", 0, 3, false),
        )
        .unwrap()
        .unwrap();

        assert_eq!(at(&without_decl, "file:///workspace/a.md", 0), 0);
        assert_eq!(at(&without_decl, "file:///workspace/a.md", 15), 1);
        assert_eq!(at(&without_decl, "file:///workspace/b.md", 0), 1);
        assert_eq!(without_decl.len(), 2);

        // cursor on the same link with include_declaration
        let with_decl = process_references(
            &mut ws.state,
            params("file:///workspace/a.md", 0, 3, true),
        )
        .unwrap()
        .unwrap();

        assert_eq!(at(&with_decl, "file:///workspace/a.md", 0), 1);
        assert_eq!(at(&with_decl, "file:///workspace/a.md", 15), 1);
        assert_eq!(at(&with_decl, "file:///workspace/b.md", 0), 1);
        assert_eq!(with_decl.len(), 3);
    }

    #[test]
    fn references_resolve_each_candidate_against_its_own_root() {
        let mut ws = TestWorkspace::new();
        ws.state.insert_root("file:///rootA".parse().unwrap());
        ws.state.insert_root("file:///rootB".parse().unwrap());
        ws.add_file("/rootA/note.md", 1, "# Note\n\nbody text")
            .add_file("/rootA/ref.md", 1, "[x](/note.md)")
            .add_file("/rootB/ref.md", 1, "[x](/note.md)");

        // cursor on "body text", not on a reference
        let locations = process_references(
            &mut ws.state,
            params("file:///rootA/note.md", 2, 1, false),
        )
        .unwrap()
        .unwrap();

        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].uri.to_string(), "file:///rootA/ref.md");
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

    #[test]
    fn finds_references_from_footnote_definition() {
        let mut ws = TestWorkspace::new();
        ws.add_file(
            "/workspace/notes.md",
            1,
            "See[^note] and also[^note] again\n\n[^note]: the footnote text",
        );

        // cursor on the "[^note]:" definition line
        let locations = process_references(
            &mut ws.state,
            params("file:///workspace/notes.md", 2, 3, false),
        )
        .unwrap()
        .unwrap();

        assert_eq!(locations.len(), 2);
    }

    #[test]
    fn finds_references_from_footnote_reference() {
        let mut ws = TestWorkspace::new();
        ws.add_file(
            "/workspace/notes.md",
            1,
            "See[^note] and also[^note] again\n\n[^note]: the footnote text",
        );

        // cursor on the first `[^note]` inline reference
        let locations = process_references(
            &mut ws.state,
            params("file:///workspace/notes.md", 0, 5, true),
        )
        .unwrap()
        .unwrap();

        // the second inline reference + the definition, plus the declaration itself
        assert_eq!(locations.len(), 3);
    }
}
