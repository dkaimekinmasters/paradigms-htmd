//! Thin wrappers over browser APIs the playground needs.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

thread_local! {
    static LAST_PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Log panics to the console and remember the message for [`guarded`].
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        LAST_PANIC.with(|last| *last.borrow_mut() = Some(info.to_string()));
        console_error_panic_hook::hook(info);
    }));
}

/// Run `work`, turning a panic or a wasm trap into an error message instead
/// of an exception that escapes into the page.
///
/// The work runs as a JS function called through `Function.call`, whose
/// wasm-bindgen import catches exceptions; a trap or panic (which aborts in
/// wasm) unwinds only to that call. Rust state touched by `work` may be left
/// inconsistent, which is why the page stops converting after a crash.
pub fn guarded<T: 'static>(work: impl FnOnce() -> T + 'static) -> Result<T, String> {
    let result = Rc::new(RefCell::new(None));
    let slot = Rc::clone(&result);
    let mut work = Some(work);
    let closure = Closure::<dyn FnMut()>::new(move || {
        if let Some(work) = work.take() {
            *slot.borrow_mut() = Some(work());
        }
    });
    let function: &js_sys::Function = closure.as_ref().unchecked_ref();
    match function.call0(&JsValue::NULL) {
        Ok(_) => Ok(result
            .borrow_mut()
            .take()
            .expect("the closure ran to completion")),
        Err(error) => {
            closure.forget();
            let panic = LAST_PANIC.with(|last| last.borrow_mut().take());
            Err(panic.unwrap_or_else(|| describe_js_error(&error)))
        }
    }
}

fn describe_js_error(error: &JsValue) -> String {
    match error.dyn_ref::<js_sys::Error>() {
        Some(error) => String::from(error.to_string()),
        None => error
            .as_string()
            .unwrap_or_else(|| "an unknown error".to_string()),
    }
}

fn window() -> web_sys::Window {
    web_sys::window().expect("the playground runs in a window")
}

/// Milliseconds since the page loaded, with sub-millisecond precision.
pub fn now() -> f64 {
    window()
        .performance()
        .map_or(0.0, |performance| performance.now())
}

/// The current address's fragment, including its `#`.
pub fn fragment() -> String {
    window().location().hash().unwrap_or_default()
}

/// The current address without its fragment.
pub fn address_without_fragment() -> String {
    let href = window().location().href().unwrap_or_default();
    match href.split_once('#') {
        Some((base, _)) => base.to_string(),
        None => href,
    }
}

/// The page's address without saved state, for "Start over".
pub fn address_without_state() -> String {
    let location = window().location();
    format!(
        "{}{}{}",
        location.origin().unwrap_or_default(),
        location.pathname().unwrap_or_default(),
        location.search().unwrap_or_default()
    )
}

/// The full current address.
pub fn address() -> String {
    window().location().href().unwrap_or_default()
}

/// Change the address without adding a history entry.
pub fn replace_address(address: &str) {
    if let Ok(history) = window().history() {
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(address));
    }
}

/// Reload the page.
pub fn reload() {
    let _ = window().location().reload();
}

/// Ask the user to confirm with a browser dialog.
pub fn confirm(message: &str) -> bool {
    window().confirm_with_message(message).unwrap_or(false)
}

/// Whether the screen is at least 800 px wide (the spec's narrow-screen
/// threshold).
pub fn is_wide_screen() -> bool {
    window()
        .match_media("(min-width: 800px)")
        .ok()
        .flatten()
        .is_some_and(|query| query.matches())
}

/// Copy text to the clipboard; `Err` when the browser does not allow it.
pub async fn copy_to_clipboard(text: &str) -> Result<(), ()> {
    let navigator = window().navigator();
    let clipboard =
        js_sys::Reflect::get(&navigator, &JsValue::from_str("clipboard")).map_err(|_| ())?;
    if clipboard.is_undefined() || clipboard.is_null() {
        return Err(());
    }
    let clipboard: web_sys::Clipboard = clipboard.unchecked_into();
    wasm_bindgen_futures::JsFuture::from(clipboard.write_text(text))
        .await
        .map(|_| ())
        .map_err(|_| ())
}

/// Focus an element by id and select all its text, for manual copying.
pub fn focus_and_select(id: &str) {
    let Some(element) = window()
        .document()
        .and_then(|document| document.get_element_by_id(id))
    else {
        return;
    };
    if let Some(textarea) = element.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        let _ = textarea.focus();
        textarea.select();
    } else if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
        let _ = input.focus();
        input.select();
    }
}

/// Move keyboard focus to an element by id.
pub fn focus(id: &str) {
    if let Some(element) = window()
        .document()
        .and_then(|document| document.get_element_by_id(id))
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = element.focus();
    }
}
