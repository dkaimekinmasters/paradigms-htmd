//! Turning untrusted HTML into a document for the Rendered and Original tabs.
//!
//! The spec's Safety constraint is met in two layers. The page shows the
//! document in an `<iframe sandbox>` without `allow-scripts`, which stops
//! scripts, forms, popups and top-level navigation. This module adds what the
//! sandbox does not cover: a Content-Security-Policy that blocks every network
//! load (images, styles, fonts, frames), and removal of whatever can still
//! navigate the frame or reach the network outside CSP: link targets,
//! `<meta http-equiv=refresh>`, `<base>`, and `<link>` (whose `dns-prefetch`
//! CSP does not govern).

use html5ever::serialize::{SerializeOpts, TraversalScope};
use html5ever::tendril::TendrilSink;
use html5ever::{local_name, parse_document, serialize};
use markup5ever_rcdom::{Handle, NodeData, RcDom, SerializableHandle};

const CONTENT_SECURITY_POLICY: &str =
    "default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:; media-src data:";

/// Light and dark styling for rendered output; the user's own CSS follows it
/// and may override it, but only inside the frame.
const FRAME_STYLE: &str = ":root{color-scheme:light dark}\
body{margin:1rem;font:16px/1.5 system-ui,-apple-system,\"Segoe UI\",Roboto,sans-serif;overflow-wrap:anywhere}\
a{color:LinkText;text-decoration:underline;cursor:default}\
img{max-width:100%}\
pre{overflow:auto;padding:.5rem;border:1px solid GrayText;border-radius:4px}\
code{font-family:ui-monospace,SFMono-Regular,Consolas,monospace}\
table{border-collapse:collapse}th,td{border:1px solid GrayText;padding:.25rem .5rem}\
blockquote{margin-left:0;padding-left:1rem;border-left:3px solid GrayText}";

/// Elements dropped together with their content.
const DROPPED_ELEMENTS: &[&str] = &["meta", "base", "link"];

/// Attributes that navigate or contact a server outside CSP's control.
const DROPPED_ATTRIBUTES: &[&str] = &[
    "href",
    "xlink:href",
    "action",
    "formaction",
    "ping",
    "target",
    "srcdoc",
    "http-equiv",
];

/// Wrap untrusted HTML in a complete document that is safe to show in a
/// sandboxed frame. `title` names the frame's document for screen readers.
pub fn sandboxed_document(html: &str, title: &str) -> String {
    let dom = parse_document(RcDom::default(), Default::default()).one(html);
    clean(&dom.document);
    let (head, body) = head_and_body(&dom.document);
    let mut document = format!(
        "<!DOCTYPE html><html><head>\
         <meta http-equiv=\"Content-Security-Policy\" content=\"{CONTENT_SECURITY_POLICY}\">\
         <meta charset=\"utf-8\"><meta name=\"color-scheme\" content=\"light dark\">\
         <title>{}</title><style>{FRAME_STYLE}</style>",
        escape_text(title)
    );
    if let Some(head) = head {
        document.push_str(&children_html(&head));
    }
    document.push_str("</head><body>");
    if let Some(body) = body {
        document.push_str(&children_html(&body));
    }
    document.push_str("</body></html>");
    document
}

/// Remove dangerous elements and attributes from the whole tree, without
/// recursion so that deeply nested input cannot overflow the stack.
fn clean(root: &Handle) {
    let mut pending = vec![root.clone()];
    while let Some(node) = pending.pop() {
        node.children
            .borrow_mut()
            .retain(|child| match &child.data {
                NodeData::Element { name, .. } => !DROPPED_ELEMENTS.contains(&&*name.local),
                NodeData::ProcessingInstruction { .. } => false,
                _ => true,
            });
        if let NodeData::Element {
            attrs,
            template_contents,
            ..
        } = &node.data
        {
            attrs.borrow_mut().retain(|attribute| {
                let name = attribute_name(attribute);
                !DROPPED_ATTRIBUTES.contains(&name.as_str()) && !name.starts_with("on")
            });
            if let Some(contents) = template_contents.borrow().as_ref() {
                pending.push(contents.clone());
            }
        }
        pending.extend(node.children.borrow().iter().cloned());
    }
}

fn attribute_name(attribute: &html5ever::Attribute) -> String {
    let local = attribute.name.local.to_ascii_lowercase().to_string();
    match &attribute.name.prefix {
        Some(prefix) => format!("{prefix}:{local}"),
        None => local,
    }
}

/// html5ever always builds `<html>` with a `<head>` and a `<body>`, or a
/// `<frameset>` in place of the body.
fn head_and_body(document: &Handle) -> (Option<Handle>, Option<Handle>) {
    let html = document
        .children
        .borrow()
        .iter()
        .find(|child| is_element(child, &local_name!("html")))
        .cloned();
    let Some(html) = html else {
        return (None, None);
    };
    let children = html.children.borrow();
    let head = children
        .iter()
        .find(|child| is_element(child, &local_name!("head")))
        .cloned();
    let body = children
        .iter()
        .find(|child| {
            is_element(child, &local_name!("body")) || is_element(child, &local_name!("frameset"))
        })
        .cloned();
    (head, body)
}

fn is_element(node: &Handle, local: &html5ever::LocalName) -> bool {
    matches!(&node.data, NodeData::Element { name, .. } if name.local == *local)
}

fn children_html(node: &Handle) -> String {
    let mut bytes = Vec::new();
    let options = SerializeOpts {
        traversal_scope: TraversalScope::ChildrenOnly(None),
        ..Default::default()
    };
    serialize(&mut bytes, &SerializableHandle::from(node.clone()), options)
        .expect("writing to a Vec cannot fail");
    String::from_utf8(bytes).expect("html5ever serializes UTF-8")
}

fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_stay_but_cannot_navigate_or_load() {
        let document = sandboxed_document(
            "<meta http-equiv=refresh content='0;url=https://e.com'><base href='https://e.com/'>\
             <link rel=dns-prefetch href='//e.com'><p onclick='x()'><a href='https://e.com' target=_top>t</a>\
             <img src='https://example.com/x.png'></p><script>alert(1)</script>",
            "Rendered",
        );
        assert!(document.starts_with(
            "<!DOCTYPE html><html><head><meta http-equiv=\"Content-Security-Policy\""
        ));
        assert!(!document.contains("refresh"));
        assert!(!document.contains("<base"));
        assert!(!document.contains("dns-prefetch"));
        assert!(!document.contains("href"));
        assert!(!document.contains("onclick"));
        assert!(!document.contains("_top"));
        assert!(document.contains("<a>t</a>"));
        assert!(document.contains("<img src=\"https://example.com/x.png\">"));
        assert!(document.contains("<script>alert(1)</script>"));
    }

    #[test]
    fn user_head_follows_the_policy() {
        let document = sandboxed_document(
            "<html><head><style>p{color:red}</style></head><body><p>a</p></body></html>",
            "Original",
        );
        let policy = document.find("Content-Security-Policy").unwrap();
        let user_style = document.find("p{color:red}").unwrap();
        assert!(policy < user_style);
        assert!(document.contains("<title>Original</title>"));
    }

    #[test]
    fn deep_nesting_does_not_recurse() {
        let html = "<div>".repeat(5_000);
        let document = std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(move || sandboxed_document(&html, "t").len())
            .unwrap()
            .join();
        assert!(document.is_ok());
    }
}
