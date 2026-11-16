//! Expose the htmd and pulldown-cmark versions in `Cargo.lock` to the header,
//! which shows the versions the page was built from.

use std::fs;

fn locked_version(lock: &str, package: &str) -> String {
    let marker = format!("name = \"{package}\"\nversion = \"");
    let normalized = lock.replace("\r\n", "\n");
    normalized
        .find(&marker)
        .map(|start| {
            let rest = &normalized[start + marker.len()..];
            rest[..rest.find('"').unwrap_or(0)].to_string()
        })
        .unwrap_or_else(|| String::from("unknown"))
}

fn main() {
    println!("cargo::rerun-if-changed=Cargo.lock");
    let lock = fs::read_to_string("Cargo.lock").unwrap_or_default();
    for (variable, package) in [
        ("HTMD_VERSION", "htmd"),
        ("PULLDOWN_CMARK_VERSION", "pulldown-cmark"),
    ] {
        println!(
            "cargo::rustc-env={variable}={}",
            locked_version(&lock, package)
        );
    }
}
