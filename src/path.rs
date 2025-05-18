use lsp_types::Uri;

pub fn get_parent_path(uri: &Uri) -> Option<String> {
    let mut segments = uri.path().segments().collect::<Vec<_>>();

    segments.pop();

    Some(format!("/{}", segments.join("/")))
}
