htmd playground
===============

A web page for trying htmd (HTML → Markdown) and pulldown-cmark (Markdown →
HTML) in the browser, as specified in
[htmddemo2-spec.md](../pulldown-cmark-htmd-demo-2/htmddemo2-spec.md). It is a
separate crate with its own workspace, so htmd's build, lint and tests are
unchanged.

Running locally
---------------

```
rustup target add wasm32-unknown-unknown
cargo install trunk --locked     # or a release binary from github.com/trunk-rs/trunk
cd playground
trunk serve --open
```

Checks
------

```
cargo clippy --target wasm32-unknown-unknown --all-targets -- --deny warnings
cargo test
```

`tests/expected_results.rs` checks every row of the spec's Expected results
tables and the acceptance criteria that need no browser.

Layout
------

* `src/settings.rs` – the options and their defaults from the spec.
* `src/convert.rs` – running the converters; the nesting limit.
* `src/sanitize.rs` – making HTML safe for the Rendered and Original tabs.
* `src/permalink.rs` – saving state in the address fragment.
* `src/presets.rs` – the Examples menu.
* `src/app.rs` – the page (Leptos 0.8).
* `src/browser.rs` – browser APIs, and turning crashes into messages.

`.github/workflows/playground.yaml` builds it on every pull request and
publishes it to GitHub Pages on every push to `main`.
