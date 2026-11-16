//! The page state stored in the address fragment, which browsers never send
//! to the server (spec: Permalinks).
//!
//! Format version 1 is `v=1&d=<direction>`, then `key=value` for each option
//! which differs from its default, then `i=<input>` unless the input does not
//! fit. Values are percent-encoded UTF-8. Reading is strict: any unknown key,
//! repeated key or unreadable value makes the whole fragment invalid. Keys and
//! values of version 1 must keep their meaning in later versions, so that
//! shared links keep working.

use std::collections::BTreeSet;

use crate::settings::{
    BR_STYLES, BULLET_MARKERS, CODE_BLOCK_STYLES, CODE_FENCES, Choice, Direction, Extension,
    HEADING_STYLES, HR_STYLES, LINK_STYLES, REFERENCE_STYLES, Settings, TRANSLATION_MODES,
};

/// The longest address the page writes, so that links survive chat apps and
/// mail clients.
pub const MAX_ADDRESS_LEN: usize = 32_768;

const VERSION: &str = "1";

/// Settings and input restored from an address.
#[derive(Clone, Debug, PartialEq)]
pub struct PageState {
    /// The direction and options.
    pub settings: Settings,
    /// The input; empty when the address was too long to hold it.
    pub input: String,
}

/// The fragment for the current state.
#[derive(Clone, Debug, PartialEq)]
pub struct Fragment {
    /// The fragment, without the leading `#`.
    pub text: String,
    /// The input did not fit and was left out.
    pub input_dropped: bool,
}

/// The fragment could not be fully read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidLink;

/// Encode `settings` and `input` for an address whose part before the `#` is
/// `address_base_len` characters long.
pub fn encode(settings: &Settings, input: &str, address_base_len: usize) -> Fragment {
    let mut text = format!("v={VERSION}&d={}", direction_key(settings.direction));
    for (key, value) in non_default_options(settings) {
        text.push('&');
        text.push_str(key);
        text.push('=');
        text.push_str(&percent_encode(&value));
    }
    let settings_len = text.len();
    text.push_str("&i=");
    text.push_str(&percent_encode(input));
    let input_dropped = address_base_len + 1 + text.len() > MAX_ADDRESS_LEN;
    if input_dropped {
        text.truncate(settings_len);
    }
    Fragment {
        text,
        input_dropped,
    }
}

/// Decode a fragment, without its leading `#`. An empty fragment carries no
/// saved state.
pub fn decode(fragment: &str) -> Result<Option<PageState>, InvalidLink> {
    if fragment.is_empty() {
        return Ok(None);
    }
    let mut seen = BTreeSet::new();
    let mut version = None;
    let mut direction = None;
    let mut input = String::new();
    let mut settings = Settings::default();
    for pair in fragment.split('&') {
        let (key, encoded) = pair.split_once('=').ok_or(InvalidLink)?;
        if !seen.insert(key) {
            return Err(InvalidLink);
        }
        let value = percent_decode(encoded)?;
        match key {
            "v" => version = Some(value),
            "d" => {
                direction = Some(match value.as_str() {
                    "html" => Direction::HtmlToMarkdown,
                    "markdown" => Direction::MarkdownToHtml,
                    _ => return Err(InvalidLink),
                })
            }
            "i" => input = value,
            _ => apply_option(&mut settings, key, &value)?,
        }
    }
    if version.as_deref() != Some(VERSION) {
        return Err(InvalidLink);
    }
    settings.direction = direction.ok_or(InvalidLink)?;
    // The encoder always writes footnotes with old-style footnotes, since one
    // requires the other.
    if settings.cmark.is_enabled(Extension::OldFootnotes)
        && !settings.cmark.is_enabled(Extension::Footnotes)
    {
        return Err(InvalidLink);
    }
    Ok(Some(PageState { settings, input }))
}

fn direction_key(direction: Direction) -> &'static str {
    match direction {
        Direction::HtmlToMarkdown => "html",
        Direction::MarkdownToHtml => "markdown",
    }
}

/// The permalink key of an extension's checkbox.
fn extension_key(extension: Extension) -> &'static str {
    match extension {
        Extension::Tables => "tables",
        Extension::Math => "math",
        Extension::Footnotes => "footnotes",
        Extension::OldFootnotes => "oldfootnotes",
        Extension::Strikethrough => "strikethrough",
        Extension::TaskLists => "tasklists",
        Extension::SmartPunctuation => "smartpunct",
        Extension::HeadingAttributes => "headingattrs",
        Extension::YamlMetadata => "yaml",
        Extension::PlusesMetadata => "pluses",
        Extension::Gfm => "gfm",
        Extension::DefinitionLists => "deflists",
        Extension::Superscript => "superscript",
        Extension::Subscript => "subscript",
        Extension::Wikilinks => "wikilinks",
    }
}

fn bool_key(on: bool) -> String {
    String::from(if on { "1" } else { "0" })
}

fn non_default_options(settings: &Settings) -> Vec<(&'static str, String)> {
    let defaults = Settings::default();
    let (options, default_options) = (&settings.htmd.options, &defaults.htmd.options);
    let choice = |choices_key: fn(&htmd::options::Options) -> &'static str| {
        (
            choices_key(options).to_string(),
            choices_key(default_options).to_string(),
        )
    };
    let mut candidates: Vec<(&'static str, (String, String))> = vec![
        (
            "mode",
            choice(|o| Choice::key_of(&TRANSLATION_MODES, o.translation_mode)),
        ),
        (
            "heading",
            choice(|o| Choice::key_of(&HEADING_STYLES, o.heading_style)),
        ),
        ("hr", choice(|o| Choice::key_of(&HR_STYLES, o.hr_style))),
        ("br", choice(|o| Choice::key_of(&BR_STYLES, o.br_style))),
        (
            "bullet",
            choice(|o| Choice::key_of(&BULLET_MARKERS, o.bullet_list_marker)),
        ),
        (
            "bulletspaces",
            (
                options.ul_bullet_spacing.to_string(),
                default_options.ul_bullet_spacing.to_string(),
            ),
        ),
        (
            "numberspaces",
            (
                options.ol_number_spacing.to_string(),
                default_options.ol_number_spacing.to_string(),
            ),
        ),
        (
            "code",
            choice(|o| Choice::key_of(&CODE_BLOCK_STYLES, o.code_block_style)),
        ),
        (
            "fence",
            choice(|o| Choice::key_of(&CODE_FENCES, o.code_block_fence)),
        ),
        (
            "links",
            choice(|o| Choice::key_of(&LINK_STYLES, o.link_style)),
        ),
        (
            "refs",
            choice(|o| Choice::key_of(&REFERENCE_STYLES, o.link_reference_style)),
        ),
        (
            "precode",
            (
                bool_key(options.preformatted_code),
                bool_key(default_options.preformatted_code),
            ),
        ),
        (
            "noscript",
            (
                bool_key(settings.htmd.noscript_as_raw_text),
                bool_key(defaults.htmd.noscript_as_raw_text),
            ),
        ),
        (
            "skip",
            (
                settings.htmd.skip_tags.clone(),
                defaults.htmd.skip_tags.clone(),
            ),
        ),
    ];
    candidates.extend(Extension::ALL.map(|extension| {
        (
            extension_key(extension),
            (
                bool_key(settings.cmark.is_enabled(extension)),
                bool_key(extension.default_enabled()),
            ),
        )
    }));
    candidates
        .into_iter()
        .filter(|(_, (value, default))| value != default)
        .map(|(key, (value, _))| (key, value))
        .collect()
}

fn apply_option(settings: &mut Settings, key: &str, value: &str) -> Result<(), InvalidLink> {
    fn pick<T: Copy + PartialEq>(choices: &[Choice<T>], key: &str) -> Result<T, InvalidLink> {
        Choice::value_of(choices, key).ok_or(InvalidLink)
    }
    fn spacing(value: &str) -> Result<u8, InvalidLink> {
        match value {
            "1" | "2" | "3" | "4" => Ok(value.as_bytes()[0] - b'0'),
            _ => Err(InvalidLink),
        }
    }
    fn flag(value: &str) -> Result<bool, InvalidLink> {
        match value {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(InvalidLink),
        }
    }
    let options = &mut settings.htmd.options;
    match key {
        "mode" => options.translation_mode = pick(&TRANSLATION_MODES, value)?,
        "heading" => options.heading_style = pick(&HEADING_STYLES, value)?,
        "hr" => options.hr_style = pick(&HR_STYLES, value)?,
        "br" => options.br_style = pick(&BR_STYLES, value)?,
        "bullet" => options.bullet_list_marker = pick(&BULLET_MARKERS, value)?,
        "bulletspaces" => options.ul_bullet_spacing = spacing(value)?,
        "numberspaces" => options.ol_number_spacing = spacing(value)?,
        "code" => options.code_block_style = pick(&CODE_BLOCK_STYLES, value)?,
        "fence" => options.code_block_fence = pick(&CODE_FENCES, value)?,
        "links" => options.link_style = pick(&LINK_STYLES, value)?,
        "refs" => options.link_reference_style = pick(&REFERENCE_STYLES, value)?,
        "precode" => options.preformatted_code = flag(value)?,
        "noscript" => settings.htmd.noscript_as_raw_text = flag(value)?,
        "skip" => settings.htmd.skip_tags = value.to_string(),
        _ => {
            let extension = Extension::ALL
                .into_iter()
                .find(|extension| extension_key(*extension) == key)
                .ok_or(InvalidLink)?;
            // Set the flag directly: `CmarkSettings::set` would let the order
            // of the footnote keys change the result.
            settings.cmark.set_raw(extension, flag(value)?);
        }
    }
    Ok(())
}

/// Percent-encode everything except RFC 3986 unreserved characters.
fn percent_encode(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// Decode `%XX` escapes; other characters stand for themselves, since a
/// browser may leave characters typed into the address unescaped.
fn percent_decode(text: &str) -> Result<String, InvalidLink> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = bytes.get(index + 1..index + 3).ok_or(InvalidLink)?;
            if !hex.iter().all(u8::is_ascii_hexdigit) {
                return Err(InvalidLink);
            }
            let hex = std::str::from_utf8(hex).map_err(|_| InvalidLink)?;
            decoded.push(u8::from_str_radix(hex, 16).map_err(|_| InvalidLink)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| InvalidLink)
}
