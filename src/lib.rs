use std::{collections::HashMap, ops::Range, path::PathBuf};

use lsp::URI;
use parser::{markdown_parser::markdown_parser, InlineMarkdown, Markdown, Parser, Spanned};

pub mod lsp;
pub mod message;
pub mod rpc;

#[derive(Default)]
pub struct LspServer {
    // Stores link for a specific file
    pub links: HashMap<String, Vec<Reference>>,
    documents: HashMap<String, String>,
    root: Option<PathBuf>,
}

// Find a way to distinguish between multiple types of links
// Internal, External, to other hearders, maybe ids?
#[derive(Debug, Clone, PartialEq)]
pub struct LinkData {
    pub file_name: String,
    pub span: Range<usize>,
    pub url: String,
    pub title: Option<String>,
    pub header: Option<LinkHeader>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkHeader {
    level: usize,
    content: String,
}

#[derive(Debug)]
pub enum Reference {
    // Header of a file
    Header {
        level: usize,
        content: String,
        span: Range<usize>,
    },
    // Tag ID
    Tag(String),
    Link(LinkData),
    WikiLink(LinkData),
    Footnote,
}

impl LspServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_root(&mut self, uri: URI) {
        self.root = Some(uri.to_path_buf());
    }

    pub fn open_document(&mut self, uri: &str, text: &str) {
        if let Some(markdown_tokens) = markdown_parser().parse(text).into_output() {
            self.extract_references(uri, markdown_tokens);
        }

        self.documents.insert(uri.to_string(), text.to_string());
    }

    pub fn update_document(&mut self, uri: &str, text: &str) {
        self.documents.insert(uri.to_string(), text.to_string());

        if let Some(markdown_tokens) = markdown_parser().parse(text).into_output() {
            self.extract_references(uri, markdown_tokens);
        }
    }

    pub fn close_document(&mut self, uri: String) {
        self.documents.remove(&uri);
    }

    pub fn get_document(&self, uri: &str) -> Option<&str> {
        self.documents.get(uri).map(|x| x.as_str())
    }

    pub fn get_document_references(&self, uri: &str) -> &[Reference] {
        self.links.get(uri).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn add_reference(&mut self, file_name: &str, reference: Reference) {
        self.links
            .entry(file_name.to_string())
            .or_default()
            .push(reference);
    }

    pub fn extract_references(&mut self, file_name: &str, markdown_spans: Vec<Spanned<Markdown>>) {
        // since we parse the whole file, clear old links and just replace
        self.links.remove(file_name);

        markdown_spans.into_iter().for_each(|spanned| {
            let Spanned(markdown, span) = spanned;
            match markdown {
                Markdown::FootnoteDefinition { .. } => {}
                Markdown::Header { level, content } => {
                    let reference = Reference::Header {
                        level,
                        content: content.to_string(),
                        span: span.into_range(),
                    };
                    self.add_reference(file_name, reference);
                }
                Markdown::Paragraph(inlines) => {
                    for inline in inlines {
                        let Spanned(inline_markdown, inline_span) = inline;

                        if let InlineMarkdown::Link { title, url, header } = inline_markdown {
                            let link_data = LinkData {
                                file_name: file_name.to_string(),
                                span: inline_span.into_range(),
                                url: url.to_string(),
                                title: Some(title.to_string()),
                                header: header.map(|h| LinkHeader {
                                    level: 1,
                                    content: h.to_string(),
                                }),
                            };
                            let reference = Reference::Link(link_data);
                            self.add_reference(file_name, reference);
                        }

                        if let InlineMarkdown::WikiLink {
                            target,
                            alias,
                            header,
                        } = inline_markdown
                        {
                            let link_data = LinkData {
                                file_name: file_name.to_string(),
                                span: inline_span.into_range(),
                                url: target.to_string(),
                                title: alias.map(String::from),
                                header: header.map(|parser::LinkHeader { level, content }| {
                                    LinkHeader {
                                        level,
                                        content: content.to_string(),
                                    }
                                }),
                            };
                            let reference = Reference::Link(link_data);
                            self.add_reference(file_name, reference);
                        }
                    }
                }
                Markdown::Invalid => {}
            }
        });
    }
}
