use lib_core::get_document;
use lsp_types::{Location, ReferenceParams};
use miette::{Context, Result};

use crate::{helpers::references::ReferenceCollector, server::Server};

pub fn process_references(
    lsp: &mut Server,
    params: ReferenceParams,
) -> Result<Option<Vec<Location>>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    let document = get_document!(lsp, &uri);
    let reference_at_position = document.get_reference_at_position(position);

    let mut reference_locations = if let Some(reference) = reference_at_position {
        ReferenceCollector::new(document, &uri, reference, lsp).collect_from(&lsp.documents)
    } else {
        ReferenceCollector::collect_file_reference_locations(lsp, &uri)
    };

    // Include the hovered reference itself if requested
    if params.context.include_declaration
        && let Some(reference) = reference_at_position
    {
        let declaration_location = Location::new(uri.clone(), reference.range);
        reference_locations.insert(0, declaration_location);
    }

    Ok(Some(reference_locations))
}
