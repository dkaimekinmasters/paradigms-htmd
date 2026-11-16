//! Behavior the spec requires beyond the Expected results: the acceptance
//! criteria, presets, Skip tags, permalinks, and preview safety.

use htmd_playground::convert::{convert, parse_skip_tags};
use htmd_playground::examples::{DEFAULT_EXAMPLE, MARKDOWN_EXAMPLES, examples, is_preset};
use htmd_playground::permalink::{InvalidLink, MAX_ADDRESS_LEN, PageState, decode, encode};
use htmd_playground::preview::preview_document;
use htmd_playground::settings::{CmarkSettings, Direction, Extension, Settings};

/// Acceptance criterion: switching direction twice on the default example
/// gives back the original HTML, ignoring a trailing newline.
#[test]
fn default_example_survives_two_switches() {
    let mut settings = Settings::default();
    let markdown = convert(DEFAULT_EXAMPLE, &settings).unwrap();
    settings.direction = Direction::MarkdownToHtml;
    let html = convert(&markdown, &settings).unwrap();
    assert_eq!(html.trim_end_matches('\n'), DEFAULT_EXAMPLE);
}

#[test]
fn basic_is_first_and_each_extension_has_a_preset_in_table_order() {
    assert_eq!(examples(Direction::HtmlToMarkdown)[0].label(), "Basic");
    assert_eq!(examples(Direction::MarkdownToHtml)[0].label(), "Basic");
    let preset_extensions: Vec<_> = MARKDOWN_EXAMPLES[1..]
        .iter()
        .map(|example| example.extension.unwrap())
        .collect();
    assert_eq!(preset_extensions, Extension::ALL);
    assert!(is_preset(DEFAULT_EXAMPLE));
    assert!(!is_preset("<p>typed</p>"));
}

/// Each extension preset changes when its extension is checked, as the
/// Examples section's "the preset always shows its feature" requires.
#[test]
fn extension_presets_show_their_feature() {
    for example in &MARKDOWN_EXAMPLES[1..] {
        let extension = example.extension.unwrap();
        let mut settings = Settings {
            direction: Direction::MarkdownToHtml,
            ..Settings::default()
        };
        // Old-style footnotes is compared with plain footnotes.
        settings.cmark = CmarkSettings::default();
        settings.cmark.set_raw(extension, false);
        if extension == Extension::OldFootnotes {
            settings.cmark.set(Extension::Footnotes, true);
        }
        let without = convert(example.text, &settings).unwrap();
        settings.cmark.set(extension, true);
        let with = convert(example.text, &settings).unwrap();
        assert_ne!(without, with, "{} preset", extension.label());
    }
}

#[test]
fn footnote_rule() {
    let mut cmark = CmarkSettings::default();
    cmark.set(Extension::OldFootnotes, true);
    assert!(cmark.is_enabled(Extension::Footnotes));
    cmark.set(Extension::Footnotes, false);
    assert!(!cmark.is_enabled(Extension::OldFootnotes));
}

#[test]
fn skip_tags_field() {
    let (valid, ignored) = parse_skip_tags("SCRIPT, style  my-el,1x <b> script");
    assert_eq!(valid, ["script", "style", "my-el"]);
    assert_eq!(ignored, ["1x", "<b>"]);
}

#[test]
fn permalink_round_trip() {
    let mut settings = Settings {
        direction: Direction::MarkdownToHtml,
        ..Settings::default()
    };
    settings.htmd.options.ul_bullet_spacing = 1;
    settings.htmd.skip_tags = "script, style".to_string();
    settings.cmark.set(Extension::OldFootnotes, true);
    settings.cmark.set(Extension::Tables, false);
    let input = "# Hi & bye\n\n100% «ünïcode» = ok?";
    let fragment = encode(&settings, input, 40);
    assert!(!fragment.input_dropped);
    assert_eq!(
        decode(&fragment.text),
        Ok(Some(PageState {
            settings,
            input: input.to_string()
        }))
    );
}

#[test]
fn permalink_records_only_changed_options() {
    let fragment = encode(&Settings::default(), "x", 40);
    assert_eq!(fragment.text, "v=1&d=html&i=x");
}

#[test]
fn permalink_drops_input_that_does_not_fit() {
    let base = "https://dkaimekinmasters.github.io/paradigms-htmd/";
    let input = "x".repeat(MAX_ADDRESS_LEN);
    let fragment = encode(&Settings::default(), &input, base.len());
    assert!(fragment.input_dropped);
    assert_eq!(fragment.text, "v=1&d=html");
    let restored = decode(&fragment.text).unwrap().unwrap();
    assert_eq!(restored.input, "");

    let fitting = "x".repeat(MAX_ADDRESS_LEN - base.len() - "#v=1&d=html&i=".len());
    let fragment = encode(&Settings::default(), &fitting, base.len());
    assert!(!fragment.input_dropped);
    assert_eq!(base.len() + 1 + fragment.text.len(), MAX_ADDRESS_LEN);
}

#[test]
fn permalink_rejects_anything_unreadable() {
    assert_eq!(decode(""), Ok(None));
    for fragment in [
        "d=html&i=x",
        "v=2&d=html",
        "v=1",
        "v=1&d=sideways",
        "v=1&d=html&color=red",
        "v=1&d=html&i=a&i=b",
        "v=1&d=html&i=%zz",
        "v=1&d=html&i=%+1",
        "v=1&d=html&i=%FF",
        "v=1&d=html&bulletspaces=5",
        "v=1&d=html&tables=yes",
        "v=1&d=html&oldfootnotes=1",
        "v=1&d=html&noequals",
    ] {
        assert_eq!(decode(fragment), Err(InvalidLink), "{fragment}");
    }
}

#[test]
fn preview_cannot_navigate_or_refresh() {
    let document = preview_document(
        r#"<!DOCTYPE html><html><head><meta http-equiv="refresh" content="0;url=https://e.com"></head><body><a href="https://e.com" target="_self">t</a><script>alert(1)</script></body></html>"#,
    );
    assert!(document.starts_with("<!DOCTYPE html><meta http-equiv=\"Content-Security-Policy\""));
    assert_eq!(document.matches("<!DOCTYPE").count(), 1);
    assert!(!document.contains(" href="), "{document}");
    assert!(
        document.contains(r#"data-href="https://e.com""#),
        "{document}"
    );
    assert!(!document.contains("refresh"), "{document}");
}
