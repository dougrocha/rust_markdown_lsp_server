#[macro_export]
macro_rules! get_document {
    ($lsp:expr, $uri:expr) => {
        $lsp.documents
            .get_document($uri)
            .with_context(|| format!("Document '{}' not found", $uri.as_str()))?
    };
}

#[macro_export]
macro_rules! get_document_mut {
    ($lsp:expr, $uri:expr) => {
        $lsp.documents
            .get_document_mut($uri)
            .with_context(|| format!("Document '{}' not found", $uri.as_str()))?
    };
}
