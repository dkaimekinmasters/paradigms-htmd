//! Saving the playground's state in the page address (spec: Permalinks).
//!
//! The state lives in the fragment, which browsers never send to the server.
//! Format version 1 is `#1.` followed by unpadded base64url of raw-deflated
//! JSON. The JSON holds only what differs from the defaults: `dir`, `input`,
//! `htmd` and `md`, the last two keyed as [`HtmdSettings`] and [`MdSettings`]
//! serialize. Addresses already shared must keep working, so a change to this
//! format needs a new version prefix, with version 1 still decoded.

use crate::settings::{Direction, HtmdSettings, MdSettings};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The longest address the playground writes, so links survive chat apps and
/// mail clients.
pub const MAX_ADDRESS_LENGTH: usize = 32_768;

const VERSION_1_PREFIX: &str = "#1.";

/// Decompressed state larger than this is refused rather than allocated.
const MAX_DECODED_BYTES: usize = 64 * 1024 * 1024;

/// Everything an address records.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SavedState {
    pub dir: Direction,
    /// `None` when the input was too large to share; it then opens empty.
    pub input: Option<String>,
    pub htmd: HtmdSettings,
    pub md: MdSettings,
}

/// What an address's fragment holds.
#[derive(Debug, PartialEq)]
pub enum Fragment {
    /// No saved state: show the default example.
    Empty,
    /// A state to restore exactly.
    State(SavedState),
    /// Something the page cannot fully read; nothing of it applies.
    Invalid,
}

/// Encode a state as a fragment, starting with `#`.
pub fn encode(state: &SavedState) -> String {
    let mut json = serde_json::to_value(state).expect("the state always serializes");
    let defaults = serde_json::to_value(SavedState::default()).expect("defaults serialize");
    if let (Value::Object(fields), Value::Object(default_fields)) = (&mut json, &defaults) {
        prune_defaults(fields, default_fields);
    }
    let compressed = miniz_oxide::deflate::compress_to_vec(json.to_string().as_bytes(), 9);
    format!("{VERSION_1_PREFIX}{}", URL_SAFE_NO_PAD.encode(compressed))
}

/// Drop every field equal to its default, recursing into nested objects and
/// dropping those left empty.
fn prune_defaults(fields: &mut Map<String, Value>, defaults: &Map<String, Value>) {
    fields.retain(|key, value| {
        let Some(default) = defaults.get(key) else {
            return true;
        };
        if let (Value::Object(inner), Value::Object(inner_defaults)) = (&mut *value, default) {
            prune_defaults(inner, inner_defaults);
            return !inner.is_empty();
        }
        value != default
    });
}

/// Read a fragment, as `location.hash` returns it.
pub fn decode(fragment: &str) -> Fragment {
    if fragment.is_empty() || fragment == "#" {
        return Fragment::Empty;
    }
    match decode_version_1(fragment) {
        Some(state) => Fragment::State(state),
        None => Fragment::Invalid,
    }
}

fn decode_version_1(fragment: &str) -> Option<SavedState> {
    let payload = fragment.strip_prefix(VERSION_1_PREFIX)?;
    let compressed = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let json =
        miniz_oxide::inflate::decompress_to_vec_with_limit(&compressed, MAX_DECODED_BYTES).ok()?;
    let state: SavedState = serde_json::from_slice(&json).ok()?;
    (state.htmd.is_valid() && state.md.is_valid()).then_some(state)
}

/// The fragment to write for `state` on a page at `base` (the address without
/// its fragment), and whether the input had to be left out to fit within
/// [`MAX_ADDRESS_LENGTH`].
pub fn fragment_for(base: &str, state: &SavedState) -> (String, bool) {
    let fragment = encode(state);
    if base.len() + fragment.len() <= MAX_ADDRESS_LENGTH {
        return (fragment, false);
    }
    let without_input = SavedState {
        input: None,
        ..state.clone()
    };
    (encode(&without_input), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Extension, HeadingStyle};

    #[test]
    fn round_trip_restores_exactly() {
        let mut state = SavedState {
            dir: Direction::MarkdownToHtml,
            input: Some("# Hi ünïcödé\n\n$x^2$".into()),
            ..Default::default()
        };
        state.htmd.heading_style = HeadingStyle::Setext;
        state.htmd.skip_tags = "script".into();
        state.md.set(Extension::OldFootnotes, true);
        state.md.set(Extension::Tables, false);
        assert_eq!(decode(&encode(&state)), Fragment::State(state));
    }

    #[test]
    fn only_differences_from_defaults_are_written() {
        let state = SavedState {
            input: Some("x".into()),
            ..Default::default()
        };
        let fragment = encode(&state);
        let compressed = URL_SAFE_NO_PAD.decode(&fragment[3..]).unwrap();
        let json = miniz_oxide::inflate::decompress_to_vec(&compressed).unwrap();
        assert_eq!(std::str::from_utf8(&json).unwrap(), r#"{"input":"x"}"#);
    }

    #[test]
    fn no_fragment_means_no_state() {
        assert_eq!(decode(""), Fragment::Empty);
        assert_eq!(decode("#"), Fragment::Empty);
    }

    #[test]
    fn unreadable_addresses_are_invalid_as_a_whole() {
        let encode_json = |json: &str| {
            let compressed = miniz_oxide::deflate::compress_to_vec(json.as_bytes(), 9);
            format!("#1.{}", URL_SAFE_NO_PAD.encode(compressed))
        };
        for fragment in [
            "#hello".to_string(),
            "#1.!!!".to_string(),
            "#2.abc".to_string(),
            encode_json(r#"{"input":"x","unknown":1}"#),
            encode_json(r#"{"dir":"sideways"}"#),
            encode_json(r#"{"htmd":{"bullet-spacing":9}}"#),
            encode_json(r#"{"md":{"old-footnotes":true,"footnotes":false}}"#),
            encode_json(r#"{"md":{"emoji":true}}"#),
        ] {
            assert_eq!(decode(&fragment), Fragment::Invalid, "{fragment}");
        }
    }

    #[test]
    fn oversized_input_is_left_out() {
        let mut seed = 0x2545_f491_u32;
        let incompressible = (0..60_000)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                char::from(b'!' + (seed % 90) as u8)
            })
            .collect();
        let state = SavedState {
            input: Some(incompressible),
            ..Default::default()
        };
        let base = "https://example.com/";
        let (fragment, too_large) = fragment_for(base, &state);
        assert!(too_large);
        assert!(base.len() + fragment.len() <= MAX_ADDRESS_LENGTH);
        let Fragment::State(restored) = decode(&fragment) else {
            panic!("not restored")
        };
        assert_eq!(restored.input, None);
    }
}
