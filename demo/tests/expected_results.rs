//! The spec's Expected results and Round trip, run against the playground's
//! own conversion code. A trailing newline in the output is ignored.

use htmd::options::{
    BrStyle, BulletListMarker, CodeBlockFence, CodeBlockStyle, HeadingStyle, HrStyle,
    LinkReferenceStyle, LinkStyle, TranslationMode,
};
use htmd_playground::convert::convert;
use htmd_playground::settings::{Direction, Extension, HtmdSettings, Settings};

fn html_to_markdown(html: &str, configure: impl FnOnce(&mut HtmdSettings)) -> String {
    let mut settings = Settings::default();
    configure(&mut settings.htmd);
    let markdown = convert(html, &settings).expect("htmd conversion");
    markdown.trim_end_matches('\n').to_string()
}

fn markdown_to_html(markdown: &str, configure: impl FnOnce(&mut Settings)) -> String {
    let mut settings = Settings {
        direction: Direction::MarkdownToHtml,
        ..Settings::default()
    };
    configure(&mut settings);
    let html = convert(markdown, &settings).expect("pulldown-cmark conversion");
    html.trim_end_matches('\n').to_string()
}

fn defaults(_: &mut HtmdSettings) {}

const H1: &str = "<h1>Hi</h1>";
const UL: &str = "<ul><li>a</li><li>b</li></ul>";
const OL: &str = r#"<ol start="3"><li>a</li></ol>"#;
const BR: &str = "<p>a<br>b</p>";
const LINK: &str = r#"<p><a href="https://e.com">t</a></p>"#;
const RUST_BLOCK: &str = r#"<pre><code class="language-rust">fn x() {}</code></pre>"#;
const CODE_SPACES: &str = "<p><code> a </code></p>";
const CLASS: &str = r#"<p class="x">a</p>"#;
const SCRIPT: &str = "<p>a</p><script>alert(1)</script>";
const NOSCRIPT: &str = "<p>a</p><noscript><b>x</b></noscript>";

#[test]
fn heading() {
    assert_eq!(html_to_markdown(H1, defaults), "# Hi");
    assert_eq!(
        html_to_markdown(H1, |s| s.options.heading_style = HeadingStyle::Setex),
        "Hi\n=="
    );
}

#[test]
fn emphasis() {
    assert_eq!(
        html_to_markdown("<p><strong>a</strong> and <em>b</em></p>", defaults),
        "**a** and *b*"
    );
}

#[test]
fn bullet_list() {
    assert_eq!(html_to_markdown(UL, defaults), "*   a\n*   b");
    assert_eq!(
        html_to_markdown(UL, |s| {
            s.options.bullet_list_marker = BulletListMarker::Dash;
            s.options.ul_bullet_spacing = 1;
        }),
        "- a\n- b"
    );
}

#[test]
fn ordered_list() {
    assert_eq!(html_to_markdown(OL, defaults), "3.  a");
    assert_eq!(
        html_to_markdown(OL, |s| s.options.ol_number_spacing = 1),
        "3. a"
    );
}

#[test]
fn horizontal_rule() {
    assert_eq!(html_to_markdown("<hr>", defaults), "* * *");
    assert_eq!(
        html_to_markdown("<hr>", |s| s.options.hr_style = HrStyle::Dashes),
        "- - -"
    );
}

#[test]
fn line_break() {
    assert_eq!(html_to_markdown(BR, defaults), "a  \nb");
    assert_eq!(
        html_to_markdown(BR, |s| s.options.br_style = BrStyle::Backslash),
        "a\\\nb"
    );
    assert_eq!(
        html_to_markdown(BR, |s| s.options.translation_mode =
            TranslationMode::Faithful),
        "a<br>b"
    );
}

#[test]
fn links() {
    assert_eq!(html_to_markdown(LINK, defaults), "[t](https://e.com)");
    assert_eq!(
        html_to_markdown(LINK, |s| s.options.link_style = LinkStyle::Referenced),
        "[t][1]\n\n[1]: https://e.com"
    );
    assert_eq!(
        html_to_markdown(LINK, |s| {
            s.options.link_style = LinkStyle::Referenced;
            s.options.link_reference_style = LinkReferenceStyle::Collapsed;
        }),
        "[t][]\n\n[t]: https://e.com"
    );
    assert_eq!(
        html_to_markdown(LINK, |s| {
            s.options.link_style = LinkStyle::Referenced;
            s.options.link_reference_style = LinkReferenceStyle::Shortcut;
        }),
        "[t]\n\n[t]: https://e.com"
    );
    assert_eq!(
        html_to_markdown(r#"<p><a href="https://e.com">https://e.com</a></p>"#, |s| {
            s.options.link_style = LinkStyle::InlinedPreferAutolinks
        }),
        "<https://e.com>"
    );
}

#[test]
fn image() {
    assert_eq!(
        html_to_markdown(r#"<p><img src="i.png" alt="cat"></p>"#, defaults),
        "![cat](i.png)"
    );
}

#[test]
fn code_blocks() {
    assert_eq!(
        html_to_markdown(RUST_BLOCK, defaults),
        "```rust\nfn x() {}\n```"
    );
    assert_eq!(
        html_to_markdown(RUST_BLOCK, |s| s.options.code_block_fence =
            CodeBlockFence::Tildes),
        "~~~rust\nfn x() {}\n~~~"
    );
    assert_eq!(
        html_to_markdown("<pre><code>fn x() {}</code></pre>", |s| {
            s.options.code_block_style = CodeBlockStyle::Indented
        }),
        "    fn x() {}"
    );
}

#[test]
fn inline_code() {
    assert_eq!(html_to_markdown(CODE_SPACES, defaults), "`a`");
    assert_eq!(
        html_to_markdown(CODE_SPACES, |s| s.options.preformatted_code = true),
        "`  a  `"
    );
}

#[test]
fn table() {
    assert_eq!(
        html_to_markdown(
            "<table><tr><th>a</th></tr><tr><td>b</td></tr></table>",
            defaults
        ),
        "| a |\n| - |\n| b |"
    );
}

#[test]
fn math() {
    assert_eq!(
        html_to_markdown(
            r#"<p><span class="math math-inline">x^2</span></p>"#,
            defaults
        ),
        "$x^2$"
    );
    assert_eq!(
        html_to_markdown(
            r#"<p><span class="math math-display">x^2</span></p>"#,
            defaults
        ),
        "$$x^2$$"
    );
}

#[test]
fn attributes() {
    assert_eq!(html_to_markdown(CLASS, defaults), "a");
    assert_eq!(
        html_to_markdown(CLASS, |s| s.options.translation_mode =
            TranslationMode::Faithful),
        r#"<p class="x">a</p>"#
    );
}

#[test]
fn script_and_skip_tags() {
    assert_eq!(html_to_markdown(SCRIPT, defaults), "a\n\nalert(1)");
    assert_eq!(
        html_to_markdown(SCRIPT, |s| s.skip_tags = "SCRIPT".to_string()),
        "a"
    );
}

#[test]
fn noscript() {
    assert_eq!(html_to_markdown(NOSCRIPT, defaults), "a\n\n\\<b>x\\</b>");
    assert_eq!(
        html_to_markdown(NOSCRIPT, |s| s.noscript_as_raw_text = false),
        "a\n\n**x**"
    );
}

#[test]
fn markdown_heading_and_math() {
    assert_eq!(markdown_to_html("# Hi", |_| {}), "<h1>Hi</h1>");
    assert_eq!(
        markdown_to_html("$x^2$", |_| {}),
        r#"<p><span class="math math-inline">x^2</span></p>"#
    );
    assert_eq!(
        markdown_to_html("$$x^2$$", |_| {}),
        r#"<p><span class="math math-display">x^2</span></p>"#
    );
    assert_eq!(
        markdown_to_html("$x^2$", |s| s.cmark.set(Extension::Math, false)),
        "<p>$x^2$</p>"
    );
}

#[test]
fn markdown_table() {
    let html = markdown_to_html("| a |\n|---|\n| b |", |_| {});
    assert!(html.starts_with("<table>"), "{html}");
    assert!(
        html.contains("<thead><tr><th>a</th></tr></thead>"),
        "{html}"
    );
    assert!(html.contains("<tbody>\n<tr><td>b</td></tr>"), "{html}");
}

#[test]
fn markdown_strikethrough() {
    assert_eq!(markdown_to_html("~~a~~", |_| {}), "<p>~~a~~</p>");
    assert_eq!(
        markdown_to_html("~~a~~", |s| s.cmark.set(Extension::Strikethrough, true)),
        "<p><del>a</del></p>"
    );
}

#[test]
fn markdown_task_list() {
    let html = markdown_to_html("- [x] a", |s| s.cmark.set(Extension::TaskLists, true));
    assert!(html.contains("<li>"), "{html}");
    let checkbox = &html[html.find("<input").expect(&html)..];
    let checkbox = &checkbox[..checkbox.find('>').expect(&html)];
    for attribute in [r#"type="checkbox""#, "checked", "disabled"] {
        assert!(checkbox.contains(attribute), "{html}");
    }
}

#[test]
fn markdown_smart_punctuation() {
    assert_eq!(
        markdown_to_html("\"a\" -- b", |s| s
            .cmark
            .set(Extension::SmartPunctuation, true)),
        "<p>“a” – b</p>"
    );
}

/// Converting to HTML and switching direction converts the output back with
/// the same options.
#[test]
fn round_trip() {
    let markdown = "# Hi\n\n$x^2$";
    let html = markdown_to_html(markdown, |_| {});
    assert_eq!(html_to_markdown(&html, defaults), markdown);
}
