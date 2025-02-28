use std::{collections::HashMap, ops::Range};

use chumsky::Parser;
use parser::{markdown_parser, InlineMarkdown, Markdown, SpannedMarkdown};

pub mod lsp;
pub mod message;
pub mod parser;
pub mod rpc;

#[derive(Default)]
pub struct LspServer {
    links: HashMap<String, Vec<MarkdownLink>>,
    documents: HashMap<String, String>,
}

#[derive(Debug)]
pub struct MarkdownLink {
    source: String,
    span: Range<usize>,
    to: String,
}

impl MarkdownLink {
    pub fn new(source: String, span: Range<usize>, to: String) -> Self {
        Self { source, span, to }
    }
}

impl LspServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_document(&mut self, uri: String, text: String) {
        let markdown_tokens = markdown_parser().parse(&*text).unwrap();

        self.extract_links_to_state(&uri, markdown_tokens);
        self.documents.insert(uri, text);
    }

    pub fn update_document(&mut self, uri: &str, text: String) {
        self.documents.insert(uri.to_string(), text.clone());
        self.links.remove(uri);

        if let Ok(markdown_tokens) = markdown_parser().parse(&*text) {
            self.extract_links_to_state(uri, markdown_tokens);
        }
    }

    pub fn close_document(&mut self, uri: String) {
        self.documents.remove(&uri);
    }

    pub fn get_document(&self, uri: &str) -> Option<&str> {
        self.documents.get(uri).map(|x| x.as_str())
    }

    pub fn get_document_links(&self, uri: &str) -> Option<&[MarkdownLink]> {
        self.links.get(uri).map(|v| v.as_slice())
    }

    pub fn extract_links_to_state(
        &mut self,
        file_name: &str,
        markdown_spans: Vec<SpannedMarkdown>,
    ) {
        for span in markdown_spans {
            match &span.0 {
                Markdown::Paragraph(inlines)
                | Markdown::Header {
                    content: inlines, ..
                }
                | Markdown::ReferenceDefinition {
                    content: inlines, ..
                } => {
                    for inline in inlines {
                        if let InlineMarkdown::Link { url, .. } = inline {
                            self.links.entry(file_name.to_string()).or_default().push(
                                MarkdownLink::new(
                                    file_name.to_string(),
                                    span.1.clone(),
                                    url.to_string(),
                                ),
                            );
                        }
                    }
                }
                // Ignore other Markdown types
                _ => {}
            }
        }
    }
}
