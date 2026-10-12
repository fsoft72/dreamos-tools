use pulldown_cmark::{Event, Parser, Tag, TagEnd};

/// One renderable block of a Markdown document. `markup` fields hold
/// escaped Pango markup, ready for `gtk::Label::set_markup`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading { level: u8, markup: String },
    Paragraph(String),
    ListItem { depth: usize, bullet: String, markup: String },
    Code(String),
    Image { src: String, alt: String },
    Rule,
}

/// A parsed Markdown document plus the metadata extracted by convention:
/// first level-1 heading = title, first paragraph = summary, first local
/// image = screenshot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub first_local_image: Option<String>,
}

/// Escapes text for safe inclusion in Pango markup.
pub fn escape_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\'' => out.push_str("&#39;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// True when an image/link target points to a remote resource (has a URL
/// scheme) rather than a file relative to the document.
pub fn is_remote(src: &str) -> bool {
    src.contains("://") || src.starts_with("data:")
}

#[derive(Debug, Clone)]
enum BlockKind {
    Heading(u8),
    Paragraph,
    ListItem { depth: usize, bullet: String },
}

/// Accumulates parser events into blocks. Inline text is buffered both as
/// Pango markup (for rendering) and plain text (for metadata).
#[derive(Default)]
struct Builder {
    doc: Document,
    kind: Option<BlockKind>,
    markup: String,
    plain: String,
    lists: Vec<Option<u64>>,
    code: Option<String>,
    image: Option<(String, String)>,
}

impl Builder {
    /// Emits the buffered inline text as a block of the current kind.
    fn flush(&mut self) {
        let markup = std::mem::take(&mut self.markup);
        let plain = std::mem::take(&mut self.plain);
        let text = plain.trim();
        if text.is_empty() {
            return;
        }
        let Some(kind) = self.kind.clone() else {
            return;
        };
        let markup = markup.trim().to_string();
        match kind {
            BlockKind::Heading(level) => {
                if level == 1 && self.doc.title.is_none() {
                    self.doc.title = Some(text.to_string());
                }
                self.doc.blocks.push(Block::Heading { level, markup });
            }
            BlockKind::Paragraph => {
                if self.doc.summary.is_none() {
                    self.doc.summary = Some(text.to_string());
                }
                self.doc.blocks.push(Block::Paragraph(markup));
            }
            BlockKind::ListItem { depth, bullet } => {
                self.doc.blocks.push(Block::ListItem { depth, bullet, markup });
            }
        }
    }

    /// Appends literal text to the inline buffers (or to the open image alt
    /// text / code block, when one is open).
    fn text(&mut self, text: &str) {
        if let Some((_, alt)) = self.image.as_mut() {
            alt.push_str(text);
            return;
        }
        if let Some(code) = self.code.as_mut() {
            code.push_str(text);
            return;
        }
        self.markup.push_str(&escape_markup(text));
        self.plain.push_str(text);
    }

    /// Starts a new list item, computing its bullet from the list stack.
    fn start_item(&mut self) {
        self.flush();
        let depth = self.lists.len().saturating_sub(1);
        let bullet = match self.lists.last_mut() {
            Some(Some(n)) => {
                let bullet = format!("{n}.");
                *n += 1;
                bullet
            }
            _ => "\u{2022}".to_string(),
        };
        self.kind = Some(BlockKind::ListItem { depth, bullet });
    }

    /// Handles a block/inline start tag.
    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => {
                self.flush();
                self.kind = Some(BlockKind::Heading(level as u8));
            }
            Tag::Paragraph => {
                if matches!(self.kind, Some(BlockKind::ListItem { .. })) {
                    return;
                }
                self.flush();
                self.kind = Some(BlockKind::Paragraph);
            }
            Tag::List(start) => {
                self.flush();
                self.lists.push(start);
            }
            Tag::Item => self.start_item(),
            Tag::CodeBlock(_) => {
                self.flush();
                self.code = Some(String::new());
            }
            Tag::Emphasis => self.markup.push_str("<i>"),
            Tag::Strong => self.markup.push_str("<b>"),
            Tag::Link { dest_url, .. } => {
                self.markup.push_str(&format!("<a href=\"{}\">", escape_markup(&dest_url)));
            }
            Tag::Image { dest_url, .. } => {
                self.flush();
                self.image = Some((dest_url.to_string(), String::new()));
            }
            _ => {}
        }
    }

    /// Handles a block/inline end tag.
    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => {
                self.flush();
                self.kind = None;
            }
            TagEnd::Paragraph => {
                if matches!(self.kind, Some(BlockKind::ListItem { .. })) {
                    return;
                }
                self.flush();
                self.kind = None;
            }
            TagEnd::Item => {
                self.flush();
                self.kind = None;
            }
            TagEnd::List(_) => {
                self.lists.pop();
            }
            TagEnd::CodeBlock => {
                let code = self.code.take().unwrap_or_default();
                self.doc.blocks.push(Block::Code(code.trim_end_matches('\n').to_string()));
            }
            TagEnd::Emphasis => self.markup.push_str("</i>"),
            TagEnd::Strong => self.markup.push_str("</b>"),
            TagEnd::Link => self.markup.push_str("</a>"),
            TagEnd::Image => {
                let Some((src, alt)) = self.image.take() else {
                    return;
                };
                if self.doc.first_local_image.is_none() && !is_remote(&src) {
                    self.doc.first_local_image = Some(src.clone());
                }
                self.doc.blocks.push(Block::Image { src, alt });
            }
            _ => {}
        }
    }
}

/// Parses Markdown source into renderable blocks and extracts metadata.
/// Raw HTML is dropped; tables and other extensions are not enabled.
pub fn parse(source: &str) -> Document {
    let mut builder = Builder::default();
    for event in Parser::new(source) {
        match event {
            Event::Start(tag) => builder.start(tag),
            Event::End(tag) => builder.end(tag),
            Event::Text(text) => builder.text(&text),
            Event::Code(code) => {
                if builder.image.is_some() {
                    builder.text(&code);
                    continue;
                }
                builder.markup.push_str(&format!("<tt>{}</tt>", escape_markup(&code)));
                builder.plain.push_str(&code);
            }
            Event::SoftBreak => builder.text(" "),
            Event::HardBreak => {
                builder.markup.push('\n');
                builder.plain.push('\n');
            }
            Event::Rule => {
                builder.flush();
                builder.doc.blocks.push(Block::Rule);
            }
            _ => {}
        }
    }
    builder.flush();
    builder.doc
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# GIMP\n\nEditor di *immagini* raster.\n\n![schermata](screenshot.png)\n\nDescrizione lunga.\n";

    #[test]
    fn extracts_metadata_by_convention() {
        let doc = parse(SAMPLE);
        assert_eq!(doc.title.as_deref(), Some("GIMP"));
        assert_eq!(doc.summary.as_deref(), Some("Editor di immagini raster."));
        assert_eq!(doc.first_local_image.as_deref(), Some("screenshot.png"));
    }

    #[test]
    fn builds_blocks_in_order() {
        let doc = parse(SAMPLE);
        assert_eq!(
            doc.blocks,
            vec![
                Block::Heading { level: 1, markup: "GIMP".into() },
                Block::Paragraph("Editor di <i>immagini</i> raster.".into()),
                Block::Image { src: "screenshot.png".into(), alt: "schermata".into() },
                Block::Paragraph("Descrizione lunga.".into()),
            ]
        );
    }

    #[test]
    fn missing_metadata_is_none() {
        let doc = parse("## Only a subheading\n");
        assert_eq!(doc.title, None);
        assert_eq!(doc.summary, None);
        assert_eq!(doc.first_local_image, None);
    }

    #[test]
    fn remote_images_are_not_screenshots() {
        let doc = parse("![a](https://example.com/a.png)\n\n![b](b.png)\n");
        assert_eq!(doc.first_local_image.as_deref(), Some("b.png"));
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn escapes_markup_and_renders_inline_styles() {
        let doc = parse("A **bold** & `x<y` [link](https://a.b/?q=1&r=2)\n");
        assert_eq!(
            doc.blocks,
            vec![Block::Paragraph(
                "A <b>bold</b> &amp; <tt>x&lt;y</tt> <a href=\"https://a.b/?q=1&amp;r=2\">link</a>".into()
            )]
        );
    }

    #[test]
    fn lists_and_code_blocks() {
        let doc = parse("- one\n  - nested\n\n1. first\n2. second\n\n```\nsudo apt\n```\n\n---\n");
        assert_eq!(
            doc.blocks,
            vec![
                Block::ListItem { depth: 0, bullet: "\u{2022}".into(), markup: "one".into() },
                Block::ListItem { depth: 1, bullet: "\u{2022}".into(), markup: "nested".into() },
                Block::ListItem { depth: 0, bullet: "1.".into(), markup: "first".into() },
                Block::ListItem { depth: 0, bullet: "2.".into(), markup: "second".into() },
                Block::Code("sudo apt".into()),
                Block::Rule,
            ]
        );
    }

    #[test]
    fn list_items_do_not_become_summary() {
        let doc = parse("# T\n\n- item\n\nReal summary.\n");
        assert_eq!(doc.summary.as_deref(), Some("Real summary."));
    }
}
