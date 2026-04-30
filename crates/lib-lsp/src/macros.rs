#[macro_export]
macro_rules! dispatch_lsp_request {
    ($lsp:expr, $request:expr, $writer:expr, {
        $($req_type:path => $handler:ident),* $(,)?
    }) => {
        {
            let method = $request.method.as_str();
            $(
                if method == <$req_type as gen_lsp_types::Request>::METHOD.as_str() {
                    handle_request::<$req_type, _, _>($lsp, $request, $writer, $handler)?;
                } else
            )*
            {
                tracing::warn!("Unimplemented request method: {}", method);
            }
        }
    };
}

#[macro_export]
macro_rules! dispatch_lsp_notification {
    ($lsp:expr, $notification:expr, {
        $($notification_type:path => $handler:ident),* $(,)?
    }) => {
        {
            let method = $notification.method.as_str();
            $(
                if method == <$notification_type as gen_lsp_types::Notification>::METHOD.as_str() {
                    handle_notification::<$notification_type, _>($lsp, $notification, $handler)?;
                } else
            )*
            {
                tracing::warn!("Unimplemented notification method: {}", method);
            }
        }
    };
}

#[macro_export]
macro_rules! get_document {
    ($state:expr, $uri:expr) => {{
        let path = $uri
            .to_file_path()
            .ok_or_else(|| miette::miette!("Invalid URI: {}", &$uri))?;

        $state
            .documents
            .get_document(&path)
            .with_context(|| format!("Document '{}' not found", path.display()))?
    }};
}

#[macro_export]
macro_rules! get_document_mut {
    ($state:expr, $uri:expr) => {{
        let path = $uri
            .to_file_path()
            .ok_or_else(|| miette::miette!("Invalid URI: {}", &$uri))?;

        $state
            .documents
            .get_document_mut(&path)
            .with_context(|| format!("Document '{}' not found", path.display()))?
    }};
}
