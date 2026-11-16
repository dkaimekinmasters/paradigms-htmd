//! Conversion logic for the htmd playground, kept free of browser APIs so it
//! can be tested on the host. The Leptos user interface lives in the binary.

pub mod convert;
pub mod examples;
pub mod permalink;
pub mod preview;
pub mod settings;

/// Input above this many bytes of UTF-8 pauses conversion (spec: Limits and
/// failures).
pub const LARGE_INPUT_BYTES: usize = 1024 * 1024;
