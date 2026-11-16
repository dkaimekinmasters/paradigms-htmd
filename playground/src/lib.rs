//! A browser playground for htmd (HTML → Markdown) and pulldown-cmark
//! (Markdown → HTML), specified in `pulldown-cmark-htmd-demo-2/htmddemo2-spec.md`.

mod app;
mod browser;
pub mod convert;
pub mod permalink;
pub mod presets;
pub mod sanitize;
pub mod settings;

/// Mount the playground into the page's `<body>`.
pub fn start() {
    browser::install_panic_hook();
    leptos::mount::mount_to_body(app::App);
}
