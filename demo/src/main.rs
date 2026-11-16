#[cfg(target_arch = "wasm32")]
mod app;
#[cfg(target_arch = "wasm32")]
mod browser;

#[cfg(target_arch = "wasm32")]
fn main() {
    // A panic leaves the Rust state unusable, so the crash message is drawn
    // by the page's own script (`index.html`), not by Leptos.
    std::panic::set_hook(Box::new(|info| {
        let message = info.to_string();
        web_sys::console::error_1(&message.as_str().into());
        browser::show_crash(&message);
    }));
    leptos::mount::mount_to_body(app::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("The playground runs in the browser; build it with `trunk build`.");
}
