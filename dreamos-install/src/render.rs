use dreamos_install::markdown::{escape_markup, is_remote, Block, Document};
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, ContentFit, Frame, Label, Orientation, Picture, Separator};
use std::path::Path;

const LIST_INDENT: i32 = 18;

/// Checks markup the way GtkLabel will use it. Links (`<a href>`) are a
/// GtkLabel extension unknown to Pango, so they are mapped to `<span>`
/// before validating the rest with Pango.
fn is_valid_markup(markup: &str) -> bool {
    let mut pango_only = String::with_capacity(markup.len());
    let mut rest = markup;
    while let Some(start) = rest.find("<a href=") {
        pango_only.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>') else {
            return false;
        };
        pango_only.push_str("<span>");
        rest = &rest[start + end + 1..];
    }
    pango_only.push_str(rest);
    let pango_only = pango_only.replace("</a>", "</span>");
    gtk4::pango::parse_markup(&pango_only, '\0').is_ok()
}

/// Builds a left-aligned, wrapping label from Pango markup. Falls back to
/// plain text when the markup does not parse, so a malformed document
/// still shows its content instead of an empty label.
fn markup_label(markup: &str) -> Label {
    let label = Label::new(None);
    if is_valid_markup(markup) {
        label.set_markup(markup);
    } else {
        eprintln!("dreamos-install: invalid markup, showing as text: {markup}");
        label.set_text(markup);
    }
    label.set_wrap(true);
    label.set_xalign(0.0);
    label.set_halign(Align::Fill);
    label.set_selectable(true);
    label
}

/// Pango size keyword for a heading level.
fn heading_size(level: u8) -> &'static str {
    match level {
        1 => "xx-large",
        2 => "x-large",
        3 => "large",
        _ => "medium",
    }
}

/// Widget for an image block: the picture when it is a readable local file,
/// otherwise a placeholder with the alt text.
fn image_widget(src: &str, alt: &str, base_dir: &Path) -> gtk4::Widget {
    let path = base_dir.join(src);
    if is_remote(src) || !path.is_file() {
        let text = if alt.is_empty() { src } else { alt };
        return markup_label(&format!("<i>[image: {}]</i>", escape_markup(text))).upcast();
    }
    let picture = Picture::for_filename(&path);
    picture.set_can_shrink(true);
    picture.set_content_fit(ContentFit::Contain);
    picture.set_halign(Align::Start);
    picture.set_alternative_text(Some(alt));
    picture.upcast()
}

/// Builds the widget for one Markdown block.
fn block_widget(block: &Block, base_dir: &Path) -> gtk4::Widget {
    match block {
        Block::Heading { level, markup } => {
            let size = heading_size(*level);
            markup_label(&format!("<span size='{size}' weight='bold'>{markup}</span>")).upcast()
        }
        Block::Paragraph(markup) => markup_label(markup).upcast(),
        Block::ListItem { depth, bullet, markup } => {
            let row = GtkBox::new(Orientation::Horizontal, 6);
            row.set_margin_start(LIST_INDENT * (*depth as i32 + 1));
            let bullet = Label::new(Some(bullet));
            bullet.set_valign(Align::Start);
            row.append(&bullet);
            let text = markup_label(markup);
            text.set_hexpand(true);
            row.append(&text);
            row.upcast()
        }
        Block::Code(code) => {
            let label = markup_label(&format!("<tt>{}</tt>", escape_markup(code)));
            label.set_wrap(false);
            label.set_margin_top(6);
            label.set_margin_bottom(6);
            label.set_margin_start(8);
            label.set_margin_end(8);
            let frame = Frame::new(None);
            frame.set_child(Some(&label));
            frame.upcast()
        }
        Block::Image { src, alt } => image_widget(src, alt, base_dir),
        Block::Rule => Separator::new(Orientation::Horizontal).upcast(),
    }
}

/// Renders a parsed Markdown document as a vertical box of widgets.
/// Relative image paths are resolved against `base_dir`.
pub fn render_document(doc: &Document, base_dir: &Path) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 10);
    container.set_margin_top(16);
    container.set_margin_bottom(16);
    container.set_margin_start(16);
    container.set_margin_end(16);
    for block in &doc.blocks {
        container.append(&block_widget(block, base_dir));
    }
    container
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_valid_markup() {
        assert!(is_valid_markup("see <a href=\"https://a.b/?q=1&amp;r=2\">the <b>site</b></a>."));
        assert!(is_valid_markup("<b>bold</b> &amp; <tt>x&lt;y</tt>"));
    }

    #[test]
    fn broken_markup_is_rejected() {
        assert!(!is_valid_markup("<b>unclosed"));
        assert!(!is_valid_markup("<a href=\"x\""));
    }
}
