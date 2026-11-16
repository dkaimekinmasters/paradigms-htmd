//! The two conversions and the **Skip tags** rule.

use std::collections::BTreeSet;

use crate::settings::{CmarkSettings, Direction, HtmdSettings, Settings};

/// Split the **Skip tags** field into lowercase tag names to skip and the
/// entries ignored as invalid, each in field order without duplicates.
///
/// htmd matches tag names by case and html5ever lowercases HTML element names,
/// so lowercasing makes the field case-insensitive. A valid name is an ASCII
/// letter followed by ASCII letters, digits or hyphens.
pub fn parse_skip_tags(field: &str) -> (Vec<String>, Vec<String>) {
    let mut valid = Vec::new();
    let mut ignored: Vec<String> = Vec::new();
    let mut seen = BTreeSet::new();
    for entry in field
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|entry| !entry.is_empty())
    {
        if is_valid_tag_name(entry) {
            let name = entry.to_ascii_lowercase();
            if seen.insert(name.clone()) {
                valid.push(name);
            }
        } else if !ignored.iter().any(|earlier| earlier == entry) {
            ignored.push(entry.to_string());
        }
    }
    (valid, ignored)
}

fn is_valid_tag_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Convert HTML to Markdown with htmd.
pub fn html_to_markdown(html: &str, settings: &HtmdSettings) -> std::io::Result<String> {
    let (skip_tags, _) = parse_skip_tags(&settings.skip_tags);
    let mut builder = htmd::HtmlToMarkdown::builder()
        .options(settings.options.clone())
        .scripting_enabled(settings.noscript_as_raw_text);
    if !skip_tags.is_empty() {
        builder = builder.skip_tags(skip_tags.iter().map(String::as_str).collect());
    }
    builder.build().convert(html)
}

/// Convert Markdown to HTML with pulldown-cmark.
pub fn markdown_to_html(markdown: &str, settings: &CmarkSettings) -> String {
    let parser = pulldown_cmark::Parser::new_ext(markdown, settings.to_options());
    let mut html = String::with_capacity(markdown.len() * 3 / 2);
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

/// The conversion shown in the **Source** tab.
pub fn convert(input: &str, settings: &Settings) -> std::io::Result<String> {
    match settings.direction {
        Direction::HtmlToMarkdown => html_to_markdown(input, &settings.htmd),
        Direction::MarkdownToHtml => Ok(markdown_to_html(input, &settings.cmark)),
    }
}
