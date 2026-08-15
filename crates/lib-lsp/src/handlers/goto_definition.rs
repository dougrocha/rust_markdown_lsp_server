use lib_core::{
    document::{Document, Reference},
    path::slug::header_slug,
};

use gen_lsp_types::{Definition, DefinitionParams, DefinitionResponse, Location, Range};
use miette::{Context, Result, miette};

use crate::{
    get_document, handlers::link_resolver::resolve_target_uri, server_state::ServerState,
    text_buffer_conversions::TextBufferConversions, uri::UriExt,
};

pub fn process_goto_definition(
    lsp: &mut ServerState,
    params: DefinitionParams,
) -> Result<Option<DefinitionResponse>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    let document = get_document!(lsp, &uri);

    let offset = document
        .source
        .slice(..)
        .try_position_to_byte_offset(position)
        .context("Position out of bounds")?;

    let reference = document
        .get_reference_at_offset(offset)
        .context("No reference found at cursor position")?;

    match reference {
        Reference::Link(link) => {
            let target = link.target_str(&document.source);
            let header = link.header_str(&document.source);

            let (target_doc, range) =
                find_definition(lsp, document, &target, header.as_deref())?;

            let target_uri = UriExt::from_file_path(&target_doc.path)
                .ok_or_else(|| miette!("Failed to convert path to URI: {:?}", target_doc.path))?;

            Ok(Some(DefinitionResponse::Definition(Definition::Location(
                Location {
                    uri: target_uri,
                    range,
                },
            ))))
        }
        Reference::Header(_) => Ok(None),
    }
}

fn find_definition<'a>(
    lsp: &'a ServerState,
    document: &Document,
    target: &str,
    header: Option<&str>,
) -> Result<(&'a Document, Range)> {
    let target_uri = resolve_target_uri(lsp, document, target)?;
    let target_doc = get_document!(lsp, &target_uri);

    let Some(header_text) = header else {
        return Ok((target_doc, Range::default()));
    };

    let target_content = header_text.strip_prefix('#').unwrap_or(header_text);
    let normalized_target = header_slug(target_content);

    let found = target_doc.headers().find(|header| {
        let content = header.content_str(&target_doc.source);

        if content == target_content {
            return true;
        }

        let normalized_content = header_slug(&content);

        normalized_content == target_content || normalized_content == normalized_target
    });

    match found {
        Some(header) => {
            let range = target_doc.source.slice(..).byte_to_lsp_range(header.span);
            Ok((target_doc, range))
        }
        None => {
            tracing::warn!(
                "Header '#{}' not found in document '{}'. Falling back to file start.",
                target_content,
                target
            );
            Ok((target_doc, Range::default()))
        }
    }
}

#[cfg(test)]
mod tests {
    use gen_lsp_types::{
        DefinitionResponse, PartialResultParams, Position, TextDocumentIdentifier,
        TextDocumentPositionParams, WorkDoneProgressParams,
    };

    use super::*;
    use crate::test_utils::TestWorkspace;

    fn params(uri: &str, line: u32, character: u32) -> DefinitionParams {
        DefinitionParams {
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: uri.parse().unwrap(),
                },
                position: Position::new(line, character),
            },
        }
    }

    #[test]
    fn goes_to_target_file_start() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/notes.md", 1, "[link](./target.md)")
            .add_file("/workspace/target.md", 1, "# Target\n\nBody");

        let result =
            process_goto_definition(&mut ws.state, params("file:///workspace/notes.md", 0, 2))
                .unwrap()
                .unwrap();

        let DefinitionResponse::Definition(Definition::Location(loc)) = result else {
            panic!("expected a location");
        };
        assert_eq!(loc.uri.to_string(), "file:///workspace/target.md");
        assert_eq!(loc.range, Range::default());
    }

    #[test]
    fn goes_to_header_in_target_file() {
        let mut ws = TestWorkspace::new();
        ws.add_file("/workspace/notes.md", 1, "[link](./target.md#Section)")
            .add_file(
                "/workspace/target.md",
                1,
                "# Target\n\n## Section\n\nBody",
            );

        let result =
            process_goto_definition(&mut ws.state, params("file:///workspace/notes.md", 0, 2))
                .unwrap()
                .unwrap();

        let DefinitionResponse::Definition(Definition::Location(loc)) = result else {
            panic!("expected a location");
        };
        assert_eq!(loc.uri.to_string(), "file:///workspace/target.md");
        assert_eq!(loc.range.start.line, 2);
    }
}
