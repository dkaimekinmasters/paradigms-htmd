//! The spec's "Expected results" tables, row by row, plus its acceptance
//! criteria that can be checked without a browser.

use htmd_playground::convert::{html_to_markdown, markdown_to_html};
use htmd_playground::presets::{DEFAULT_EXAMPLE, presets};
use htmd_playground::settings::*;

fn to_markdown(html: &str, adjust: impl FnOnce(&mut HtmdSettings)) -> String {
    let mut settings = HtmdSettings::default();
    adjust(&mut settings);
    html_to_markdown(html, &settings).unwrap()
}

fn to_html(markdown: &str, adjust: impl FnOnce(&mut MdSettings)) -> String {
    let mut settings = MdSettings::default();
    adjust(&mut settings);
    markdown_to_html(markdown, &settings)
}

/// Compare ignoring a trailing newline, as the spec allows.
#[track_caller]
fn check(actual: String, expected: &str) {
    assert_eq!(actual.trim_end_matches('\n'), expected);
}

#[test]
fn html_to_markdown_rows() {
    let link = r#"<p><a href="https://e.com">t</a></p>"#;
    let br = "<p>a<br>b</p>";
    let list = "<ul><li>a</li><li>b</li></ul>";
    let ordered = r#"<ol start="3"><li>a</li></ol>"#;
    let rust = r#"<pre><code class="language-rust">fn x() {}</code></pre>"#;
    let inline_code = "<p><code> a </code></p>";
    let attributes = r#"<p class="x">a</p>"#;
    let script = "<p>a</p><script>alert(1)</script>";
    let noscript = "<p>a</p><noscript><b>x</b></noscript>";

    check(to_markdown("<h1>Hi</h1>", |_| {}), "# Hi");
    check(
        to_markdown("<h1>Hi</h1>", |s| s.heading_style = HeadingStyle::Setext),
        "Hi\n==",
    );
    check(
        to_markdown("<p><strong>a</strong> and <em>b</em></p>", |_| {}),
        "**a** and *b*",
    );
    check(to_markdown(list, |_| {}), "*   a\n*   b");
    check(
        to_markdown(list, |s| {
            s.bullet_marker = BulletMarker::Dash;
            s.bullet_spacing = 1;
        }),
        "- a\n- b",
    );
    check(to_markdown(ordered, |_| {}), "3.  a");
    check(to_markdown(ordered, |s| s.number_spacing = 1), "3. a");
    check(to_markdown("<hr>", |_| {}), "* * *");
    check(
        to_markdown("<hr>", |s| s.horizontal_rule = HrStyle::Dashes),
        "- - -",
    );
    check(to_markdown(br, |_| {}), "a  \nb");
    check(
        to_markdown(br, |s| s.line_break = BrStyle::Backslash),
        "a\\\nb",
    );
    check(
        to_markdown(br, |s| s.translation_mode = TranslationMode::Faithful),
        "a<br>b",
    );
    check(to_markdown(link, |_| {}), "[t](https://e.com)");
    check(
        to_markdown(link, |s| s.link_style = LinkStyle::Referenced),
        "[t][1]\n\n[1]: https://e.com",
    );
    check(
        to_markdown(link, |s| {
            s.link_style = LinkStyle::Referenced;
            s.reference_style = ReferenceStyle::Collapsed;
        }),
        "[t][]\n\n[t]: https://e.com",
    );
    check(
        to_markdown(link, |s| {
            s.link_style = LinkStyle::Referenced;
            s.reference_style = ReferenceStyle::Shortcut;
        }),
        "[t]\n\n[t]: https://e.com",
    );
    check(
        to_markdown(r#"<p><a href="https://e.com">https://e.com</a></p>"#, |s| {
            s.link_style = LinkStyle::InlineAutolinks
        }),
        "<https://e.com>",
    );
    check(
        to_markdown(r#"<p><img src="i.png" alt="cat"></p>"#, |_| {}),
        "![cat](i.png)",
    );
    check(to_markdown(rust, |_| {}), "```rust\nfn x() {}\n```");
    check(
        to_markdown(rust, |s| s.code_fence = CodeFence::Tildes),
        "~~~rust\nfn x() {}\n~~~",
    );
    check(
        to_markdown("<pre><code>fn x() {}</code></pre>", |s| {
            s.code_block_style = CodeBlockStyle::Indented
        }),
        "    fn x() {}",
    );
    check(to_markdown(inline_code, |_| {}), "`a`");
    check(
        to_markdown(inline_code, |s| s.preserve_code_whitespace = true),
        "`  a  `",
    );
    check(
        to_markdown(
            "<table><tr><th>a</th></tr><tr><td>b</td></tr></table>",
            |_| {},
        ),
        "| a |\n| - |\n| b |",
    );
    check(
        to_markdown(
            r#"<p><span class="math math-inline">x^2</span></p>"#,
            |_| {},
        ),
        "$x^2$",
    );
    check(
        to_markdown(
            r#"<p><span class="math math-display">x^2</span></p>"#,
            |_| {},
        ),
        "$$x^2$$",
    );
    check(to_markdown(attributes, |_| {}), "a");
    check(
        to_markdown(attributes, |s| {
            s.translation_mode = TranslationMode::Faithful
        }),
        r#"<p class="x">a</p>"#,
    );
    check(to_markdown(script, |_| {}), "a\n\nalert(1)");
    check(to_markdown(script, |s| s.skip_tags = "SCRIPT".into()), "a");
    check(to_markdown(noscript, |_| {}), "a\n\n\\<b>x\\</b>");
    check(
        to_markdown(noscript, |s| s.noscript_raw_text = false),
        "a\n\n**x**",
    );
}

#[test]
fn markdown_to_html_rows() {
    check(to_html("# Hi", |_| {}), "<h1>Hi</h1>");
    check(
        to_html("$x^2$", |_| {}),
        r#"<p><span class="math math-inline">x^2</span></p>"#,
    );
    check(
        to_html("$$x^2$$", |_| {}),
        r#"<p><span class="math math-display">x^2</span></p>"#,
    );
    check(
        to_html("$x^2$", |s| s.set(Extension::Math, false)),
        "<p>$x^2$</p>",
    );
    let table = to_html("| a |\n|---|\n| b |", |_| {});
    assert!(
        table.contains("<thead><tr><th>a</th></tr></thead>"),
        "{table}"
    );
    assert!(table.contains("<tbody>\n<tr><td>b</td></tr>"), "{table}");
    check(to_html("~~a~~", |_| {}), "<p>~~a~~</p>");
    check(
        to_html("~~a~~", |s| s.set(Extension::Strikethrough, true)),
        "<p><del>a</del></p>",
    );
    let task = to_html("- [x] a", |s| s.set(Extension::TaskLists, true));
    assert!(
        task.contains(r#"<input disabled="" type="checkbox" checked=""/>"#),
        "{task}"
    );
    check(
        to_html("\"a\" -- b", |s| s.set(Extension::SmartPunctuation, true)),
        "<p>“a” – b</p>",
    );
}

#[test]
fn round_trip() {
    let html = to_html("# Hi\n\n$x^2$", |_| {});
    check(to_markdown(&html, |_| {}), "# Hi\n\n$x^2$");
}

/// Acceptance: switching direction twice on the default example gives back
/// the original HTML, ignoring whitespace between tags and a trailing newline.
#[test]
fn default_example_survives_two_switches() {
    let markdown = to_markdown(DEFAULT_EXAMPLE, |_| {});
    let html = to_html(&markdown, |_| {});
    let squeeze = |text: &str| text.trim().replace(">\n<", "><");
    assert_eq!(squeeze(&html), squeeze(DEFAULT_EXAMPLE));
}

#[test]
fn old_style_footnotes_imply_footnotes() {
    let mut settings = MdSettings::default();
    settings.set(Extension::OldFootnotes, true);
    assert!(settings.get(Extension::Footnotes));
    settings.set(Extension::Footnotes, false);
    assert!(!settings.get(Extension::OldFootnotes));
}

/// A change to one htmd option.
type OptionChange = fn(&mut HtmdSettings);

/// Acceptance: every option visibly changes the result of at least one
/// preset, in Source or Rendered (Rendered is pulldown-cmark's HTML).
#[test]
fn every_option_changes_some_preset() {
    let html_presets = presets(Direction::HtmlToMarkdown);
    let default_htmd = HtmdSettings::default();
    let changes: Vec<(&str, OptionChange)> = vec![
        (
            "translation mode",
            (|s| s.translation_mode = TranslationMode::Faithful),
        ),
        (
            "heading style",
            (|s| s.heading_style = HeadingStyle::Setext),
        ),
        (
            "horizontal rule",
            (|s| s.horizontal_rule = HrStyle::Underscores),
        ),
        ("line break", (|s| s.line_break = BrStyle::Backslash)),
        ("bullet marker", (|s| s.bullet_marker = BulletMarker::Dash)),
        ("bullet spacing", (|s| s.bullet_spacing = 1)),
        ("number spacing", (|s| s.number_spacing = 4)),
        (
            "code block style",
            (|s| s.code_block_style = CodeBlockStyle::Indented),
        ),
        ("code fence", (|s| s.code_fence = CodeFence::Tildes)),
        (
            "link style",
            (|s| s.link_style = LinkStyle::InlineAutolinks),
        ),
        (
            "reference style",
            (|s| {
                s.link_style = LinkStyle::Referenced;
                s.reference_style = ReferenceStyle::Shortcut;
            }),
        ),
        (
            "preserve code whitespace",
            (|s| s.preserve_code_whitespace = true),
        ),
        ("noscript", (|s| s.noscript_raw_text = false)),
        ("skip tags", (|s| s.skip_tags = "em".into())),
    ];
    for (name, change) in changes {
        let mut changed = default_htmd.clone();
        change(&mut changed);
        let mut baseline = default_htmd.clone();
        if name == "reference style" {
            baseline.link_style = LinkStyle::Referenced;
        }
        assert!(
            html_presets.iter().any(|preset| {
                html_to_markdown(preset.text, &baseline).unwrap()
                    != html_to_markdown(preset.text, &changed).unwrap()
            }),
            "{name} changes no preset"
        );
    }

    let markdown_presets = presets(Direction::MarkdownToHtml);
    for extension in Extension::ALL {
        let default_md = MdSettings::default();
        let mut toggled = default_md;
        toggled.set(extension, !default_md.get(extension));
        assert!(
            markdown_presets.iter().any(|preset| {
                markdown_to_html(preset.text, &default_md)
                    != markdown_to_html(preset.text, &toggled)
            }),
            "{} changes no preset",
            extension.label()
        );
    }
}

/// The spec's speed target is measured in a browser; this native check only
/// guards against the Article example becoming grossly slow.
#[test]
fn article_converts_quickly() {
    let article = presets(Direction::HtmlToMarkdown)
        .iter()
        .find(|preset| preset.name == "Article")
        .unwrap();
    let settings = HtmdSettings::default();
    let mut times: Vec<_> = (0..10)
        .map(|_| {
            let start = std::time::Instant::now();
            html_to_markdown(article.text, &settings).unwrap();
            start.elapsed()
        })
        .collect();
    times.sort();
    assert!(times[5] < std::time::Duration::from_millis(10), "{times:?}");
}
