//! Documents for the **Rendered** and **Original** tabs, which show untrusted
//! HTML in an `<iframe sandbox srcdoc>`.
//!
//! The sandbox (no `allow-*` tokens) stops scripts, forms, popups and top-level
//! navigation, and the CSP below stops every network load. Neither stops a
//! link from navigating the frame itself, so every `href` is renamed to
//! `data-href`, which also keeps `<base>` and `<link>` inert. Meta refresh is
//! removed for the same reason.

use std::rc::Rc;

use html5ever::serialize::{SerializeOpts, TraversalScope, serialize};
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{LocalName, ParseOpts, parse_document};
use markup5ever_rcdom::{Handle, NodeData, RcDom, SerializableHandle};

/// Prepended to the serialized input. The CSP must be the first element
/// parsed so that it covers everything after it; the parser merges the
/// input's own `<html>` and `<head>` into the elements this creates.
const HEAD: &str = concat!(
    "<!DOCTYPE html>",
    r#"<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'">"#,
    r#"<meta name="color-scheme" content="light dark">"#,
    "<style>",
    ":root{color-scheme:light dark}",
    "body{margin:1rem;font:16px/1.5 system-ui,sans-serif;overflow-wrap:break-word}",
    "a[data-href]{color:#0b57d0;text-decoration:underline;cursor:default}",
    "@media (prefers-color-scheme:dark){a[data-href]{color:#8ab4f8}}",
    "pre{overflow-x:auto}",
    "table{border-collapse:collapse}th,td{border:1px solid #8888;padding:.25rem .5rem}",
    "img{max-width:100%}",
    "</style>",
);

/// Wrap `html` in a self-contained document that cannot run scripts, load
/// anything, or navigate.
pub fn preview_document(html: &str) -> String {
    // Parse as the sandboxed frame will, with scripting disabled, so that
    // `<noscript>` content serializes as markup rather than escaped text.
    let parse_options = ParseOpts {
        tree_builder: TreeBuilderOpts {
            scripting_enabled: false,
            ..TreeBuilderOpts::default()
        },
        ..ParseOpts::default()
    };
    let dom = parse_document(RcDom::default(), parse_options).one(html);
    neutralize(&dom.document);
    let mut serialized = Vec::with_capacity(html.len() + HEAD.len());
    serialized.extend_from_slice(HEAD.as_bytes());
    serialize(
        &mut serialized,
        &SerializableHandle::from(Rc::clone(&dom.document)),
        SerializeOpts {
            scripting_enabled: false,
            traversal_scope: TraversalScope::ChildrenOnly(None),
            ..SerializeOpts::default()
        },
    )
    .expect("writing to a Vec cannot fail");
    String::from_utf8(serialized).expect("html5ever serializes valid UTF-8")
}

/// Remove the doctype, which `HEAD` supplies, and every way left to navigate.
/// Iterative, since deeply nested input would overflow a recursive walk.
fn neutralize(document: &Handle) {
    document
        .children
        .borrow_mut()
        .retain(|child| !matches!(child.data, NodeData::Doctype { .. }));
    let mut stack: Vec<Handle> = vec![Rc::clone(document)];
    while let Some(node) = stack.pop() {
        if let NodeData::Element { name, attrs, .. } = &node.data {
            let mut attrs = attrs.borrow_mut();
            if &*name.local == "meta" {
                attrs.retain(|attr| &*attr.name.local != "http-equiv");
            }
            for attr in attrs.iter_mut() {
                if &*attr.name.local == "href" {
                    attr.name.local = LocalName::from("data-href");
                }
            }
        }
        stack.extend(node.children.borrow().iter().cloned());
    }
}
