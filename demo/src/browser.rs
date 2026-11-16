//! Thin wrappers over browser APIs which may be missing or blocked.

use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

fn window() -> web_sys::Window {
    web_sys::window().expect("the playground runs in a window")
}

fn property(target: &JsValue, name: &str) -> Option<JsValue> {
    Reflect::get(target, &JsValue::from_str(name))
        .ok()
        .filter(|value| !value.is_undefined() && !value.is_null())
}

/// Show the crash message through `window.htmdShowCrash` in `index.html`.
pub fn show_crash(detail: &str) {
    let window = window();
    if let Some(show) =
        property(&window, "htmdShowCrash").and_then(|f| f.dyn_into::<Function>().ok())
    {
        let _ = show.call1(&window, &JsValue::from_str(detail));
    }
}

/// Whether `index.html` has seen a crash, including a trap, such as a stack
/// overflow, which never reaches the panic hook.
pub fn crash_reported() -> bool {
    property(&window(), "htmdCrashed").is_some_and(|flag| flag.is_truthy())
}

/// Milliseconds from `performance.now()`.
pub fn now() -> f64 {
    window()
        .performance()
        .map_or(0.0, |performance| performance.now())
}

/// Copy `text`; false when the Clipboard API is missing (as on plain HTTP)
/// or refuses. Called through `Reflect` because web-sys would throw on a
/// missing `navigator.clipboard`.
pub async fn copy_text(text: &str) -> bool {
    let window = window();
    let Some(clipboard) = property(&window, "navigator").and_then(|n| property(&n, "clipboard"))
    else {
        return false;
    };
    let Some(write_text) =
        property(&clipboard, "writeText").and_then(|f| f.dyn_into::<Function>().ok())
    else {
        return false;
    };
    match write_text
        .call1(&clipboard, &JsValue::from_str(text))
        .ok()
        .and_then(|promise| promise.dyn_into::<Promise>().ok())
    {
        Some(promise) => JsFuture::from(promise).await.is_ok(),
        None => false,
    }
}

/// Select the text inside `element`, for copying by hand.
pub fn select_contents(element: &web_sys::Element) {
    let window = window();
    let (Some(document), Ok(Some(selection))) = (window.document(), window.get_selection()) else {
        return;
    };
    if let Ok(range) = document.create_range()
        && range.select_node_contents(element).is_ok()
    {
        let _ = selection.remove_all_ranges();
        let _ = selection.add_range(&range);
    }
}

/// The address without its fragment.
pub fn address_base() -> String {
    let href = window().location().href().unwrap_or_default();
    href.split('#').next().unwrap_or_default().to_string()
}

/// The fragment, without its `#`.
pub fn address_fragment() -> String {
    let hash = window().location().hash().unwrap_or_default();
    hash.strip_prefix('#').unwrap_or(&hash).to_string()
}

/// Replace the fragment without adding a history entry or firing
/// `hashchange`.
pub fn replace_fragment(fragment: &str) {
    if let Ok(history) = window().history() {
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&format!("#{fragment}")));
    }
}

/// The full address.
pub fn address() -> String {
    window().location().href().unwrap_or_default()
}

/// Ask the user to confirm; false if the dialog is blocked.
pub fn confirm(message: &str) -> bool {
    window().confirm_with_message(message).unwrap_or(false)
}

/// Whether the screen is narrow (spec: under 800 px wide).
pub fn is_narrow_screen() -> bool {
    window()
        .match_media("(max-width: 799.98px)")
        .ok()
        .flatten()
        .is_some_and(|query| query.matches())
}
