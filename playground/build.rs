//! Expose the htmd and pulldown-cmark versions this build resolved, for the
//! page header.

fn main() {
    println!("cargo::rerun-if-changed=Cargo.lock");
    println!("cargo::rerun-if-changed=../Cargo.toml");
    let lock = std::fs::read_to_string("Cargo.lock").unwrap_or_default();
    for (package, variable) in [
        ("htmd", "HTMD_VERSION"),
        ("pulldown-cmark", "PULLDOWN_CMARK_VERSION"),
    ] {
        let version = locked_version(&lock, package).unwrap_or("unknown");
        println!("cargo::rustc-env={variable}={version}");
    }
}

/// The `version` line following `name = "<package>"` in a Cargo.lock.
fn locked_version<'a>(lock: &'a str, package: &str) -> Option<&'a str> {
    let name_line = format!("name = \"{package}\"");
    let mut lines = lock.lines().skip_while(|line| *line != name_line);
    lines.next()?;
    lines
        .next()?
        .strip_prefix("version = \"")?
        .strip_suffix('"')
}
