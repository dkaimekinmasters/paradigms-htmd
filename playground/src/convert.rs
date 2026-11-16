//! Running the two converters with the playground's settings.

use crate::sanitize::sandboxed_document;
use crate::settings::{Direction, HtmdSettings, MdSettings};
use markup5ever_rcdom::{Handle, NodeData};
use std::fmt;

/// The deepest element nesting the playground passes to htmd.
///
/// htmd walks the tree recursively. Measured natively with a 1 MiB stack it
/// manages about 550 levels of `<table><tr><td>`, its most stack-hungry case;
/// the wasm build links an 8 MiB stack (see `.cargo/config.toml`), so this
/// limit leaves a wide margin. Deeper input is reported as a crash instead of
/// overflowing the stack, which in wasm would corrupt the whole page.
pub const MAX_NESTING: usize = 1000;

/// Why a conversion produced no output.
#[derive(Clone, Debug, PartialEq)]
pub enum ConversionError {
    /// The input nests elements deeper than [`MAX_NESTING`].
    TooDeep { depth: usize },
    /// The converter reported an error.
    Failed(String),
}

impl fmt::Display for ConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::TooDeep { depth } => write!(
                formatter,
                "the input nests elements {depth} levels deep, more than the {MAX_NESTING} the converter can handle"
            ),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

/// Everything the output pane and status line show for one conversion.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Output {
    /// The Source tab: the converter's output text.
    pub source: String,
    /// The Rendered tab, as a sandboxable document.
    pub rendered: String,
    /// The Original tab, shown only for HTML → Markdown.
    pub original: Option<String>,
    /// How long producing [`Output::source`] took, in milliseconds.
    pub millis: f64,
    /// The length of [`Output::source`] in Unicode code points.
    pub chars: usize,
}

/// Convert `input` in `direction` and build every tab's content. `now`
/// returns a time in milliseconds; only the Source conversion is timed, as
/// the spec's Status line section requires.
pub fn run(
    direction: Direction,
    input: &str,
    htmd_settings: &HtmdSettings,
    md_settings: &MdSettings,
    now: impl Fn() -> f64,
) -> Result<Output, ConversionError> {
    let start = now();
    let source = match direction {
        Direction::HtmlToMarkdown => html_to_markdown(input, htmd_settings)?,
        Direction::MarkdownToHtml => markdown_to_html(input, md_settings),
    };
    let millis = now() - start;
    let rendered_html = match direction {
        Direction::HtmlToMarkdown => markdown_to_html(&source, md_settings),
        Direction::MarkdownToHtml => source.clone(),
    };
    let original = (direction == Direction::HtmlToMarkdown)
        .then(|| sandboxed_document(input, "Original HTML"));
    Ok(Output {
        rendered: sandboxed_document(&rendered_html, "Rendered output"),
        original,
        millis,
        chars: source.chars().count(),
        source,
    })
}

/// Convert HTML to Markdown with htmd.
pub fn html_to_markdown(html: &str, settings: &HtmdSettings) -> Result<String, ConversionError> {
    let (skip, _) = parse_skip_tags(&settings.skip_tags);
    let mut builder = htmd::HtmlToMarkdown::builder()
        .options(settings.to_htmd())
        .scripting_enabled(settings.noscript_raw_text);
    if !skip.is_empty() {
        builder = builder.skip_tags(skip.iter().map(String::as_str).collect());
    }
    let converter = builder.build();
    let tree = converter
        .html_to_tree(html)
        .map_err(|error| ConversionError::Failed(error.to_string()))?;
    let depth = nesting_depth(&tree);
    if depth > MAX_NESTING {
        return Err(ConversionError::TooDeep { depth });
    }
    Ok(converter.tree_to_markdown(&tree))
}

/// Convert Markdown to HTML with pulldown-cmark.
pub fn markdown_to_html(markdown: &str, settings: &MdSettings) -> String {
    let parser = pulldown_cmark::Parser::new_ext(markdown, settings.to_pulldown());
    let mut html = String::with_capacity(markdown.len() * 3 / 2);
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

/// Split the Skip tags field into lowercased tag names and the entries
/// ignored because they are not tag names.
///
/// htmd matches tag names by case, while html5ever lowercases every HTML tag
/// it parses, so lowercasing here makes Skip tags case-insensitive as the spec
/// requires.
pub fn parse_skip_tags(field: &str) -> (Vec<String>, Vec<String>) {
    let mut valid: Vec<String> = Vec::new();
    let mut ignored = Vec::new();
    for entry in field
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|entry| !entry.is_empty())
    {
        if is_tag_name(entry) {
            let name = entry.to_ascii_lowercase();
            if !valid.contains(&name) {
                valid.push(name);
            }
        } else {
            ignored.push(entry.to_string());
        }
    }
    (valid, ignored)
}

/// An ASCII letter followed by ASCII letters, digits, or hyphens; this covers
/// HTML elements and custom elements such as `my-widget`.
fn is_tag_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// The deepest chain of nested elements, counted without recursion.
pub fn nesting_depth(root: &Handle) -> usize {
    let mut deepest = 0;
    let mut pending = vec![(root.clone(), 0usize)];
    while let Some((node, depth)) = pending.pop() {
        let depth = if matches!(node.data, NodeData::Element { .. }) {
            depth + 1
        } else {
            depth
        };
        deepest = deepest.max(depth);
        pending.extend(
            node.children
                .borrow()
                .iter()
                .map(|child| (child.clone(), depth)),
        );
    }
    deepest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_tags_split_on_commas_and_spaces() {
        let (valid, ignored) = parse_skip_tags(" SCRIPT, style  my-widget,,1x <b> ");
        assert_eq!(valid, ["script", "style", "my-widget"]);
        assert_eq!(ignored, ["1x", "<b>"]);
    }

    #[test]
    fn deep_input_is_refused() {
        let html = "<div>".repeat(MAX_NESTING + 10);
        assert!(matches!(
            html_to_markdown(&html, &HtmdSettings::default()),
            Err(ConversionError::TooDeep { .. })
        ));
    }

    #[test]
    fn nesting_within_the_limit_converts() {
        let html = "<blockquote>".repeat(50) + "x";
        assert!(html_to_markdown(&html, &HtmdSettings::default()).is_ok());
    }
}
