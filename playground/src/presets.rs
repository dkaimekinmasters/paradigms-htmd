//! The Examples menu (spec: Examples).

use crate::settings::{Direction, Extension};

/// One entry of the Examples menu.
#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub name: &'static str,
    pub text: &'static str,
    /// The extension a Markdown preset demonstrates; choosing the preset
    /// turns it on.
    pub extension: Option<Extension>,
}

const fn html(name: &'static str, text: &'static str) -> Preset {
    Preset {
        name,
        text,
        extension: None,
    }
}

const fn markdown(extension: Extension, text: &'static str) -> Preset {
    Preset {
        name: "",
        text,
        extension: Some(extension),
    }
}

impl Preset {
    /// The menu label: an extension preset is named after its checkbox.
    pub fn label(&self) -> &'static str {
        match self.extension {
            Some(extension) => extension.label(),
            None => self.name,
        }
    }
}

/// The page opens with this when the address carries no saved state.
pub const DEFAULT_EXAMPLE: &str = HTML_PRESETS[0].text;

/// The presets for a direction, Basic first.
pub fn presets(direction: Direction) -> &'static [Preset] {
    match direction {
        Direction::HtmlToMarkdown => HTML_PRESETS,
        Direction::MarkdownToHtml => MARKDOWN_PRESETS,
    }
}

/// Whether `text` is exactly one of the presets of either direction.
pub fn is_preset(text: &str) -> bool {
    HTML_PRESETS
        .iter()
        .chain(MARKDOWN_PRESETS)
        .any(|preset| preset.text == text)
}

const HTML_PRESETS: &[Preset] = &[
    html(
        "Basic",
        "<p>Type <strong>HTML</strong> here and see it as <em>Markdown</em> on the right.</p>\n",
    ),
    html(
        "Headings",
        "<h1>Main title</h1>\n<p>An introduction.</p>\n<h2>Section</h2>\n<p>Some text in the section.</p>\n<h3>Subsection</h3>\n<p>More detail.</p>\n",
    ),
    html(
        "Lists",
        "<ul>\n  <li>Fruit\n    <ul>\n      <li>Apple</li>\n      <li>Pear</li>\n    </ul>\n  </li>\n  <li>Vegetables\n    <ul>\n      <li>Carrot</li>\n    </ul>\n  </li>\n</ul>\n<ol start=\"3\">\n  <li>Third</li>\n  <li>Fourth</li>\n  <li>Fifth</li>\n</ol>\n",
    ),
    html(
        "Links and images",
        "<p>Bare: <a href=\"https://commonmark.org\">https://commonmark.org</a></p>\n<p>Titled: <a href=\"https://docs.rs/htmd\" title=\"htmd on docs.rs\">the htmd docs</a></p>\n<p><img src=\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='48' height='32'%3E%3Crect width='48' height='32' rx='6' fill='%23e8a33d'/%3E%3C/svg%3E\" alt=\"an orange cat, abstractly\"></p>\n",
    ),
    html(
        "Code",
        "<p>Call <code> convert(html) </code> to get Markdown.</p>\n<pre><code class=\"language-rust\">fn main() {\n    let md = htmd::convert(\"&lt;h1&gt;Hi&lt;/h1&gt;\").unwrap();\n    println!(\"{md}\");\n}</code></pre>\n",
    ),
    html(
        "Tables",
        "<table>\n  <thead>\n    <tr><th>Crate</th><th>Converts</th></tr>\n  </thead>\n  <tbody>\n    <tr><td>htmd</td><td>HTML to Markdown</td></tr>\n    <tr><td>pulldown-cmark</td><td>Markdown to HTML</td></tr>\n  </tbody>\n</table>\n",
    ),
    html(
        "Blockquotes",
        "<blockquote>\n  <p>The outer quote.</p>\n  <blockquote>\n    <p>A quote inside the quote.</p>\n  </blockquote>\n  <p>Back to the outer quote.</p>\n</blockquote>\n",
    ),
    html(
        "Math",
        "<p>Inline: <span class=\"math math-inline\">e^{i\\pi} + 1 = 0</span>.</p>\n<p>Display:</p>\n<p><span class=\"math math-display\">\\sum_{k=1}^{n} k = \\frac{n(n+1)}{2}</span></p>\n",
    ),
    html("Article", ARTICLE),
    html(
        "Edge cases",
        "<p></p>\n<div><span></span></div>\n<p>Entities: &amp; &lt; &gt; &quot; &copy; &#x1F600; &nbsp;|</p>\n<p>Runs   of\n\n   whitespace\tand\ttabs.</p>\n<!-- a comment -->\n<p class=\"note\" id=\"edge\">A paragraph with attributes.</p>\n<script>document.title = \"script ran\";</script>\n<style>p { color: red; }</style>\n<noscript><p><b>Shown</b> when scripts are off.</p></noscript>\n",
    ),
];

/// About three kilobytes of prose written for this playground, released with
/// it under its license.
const ARTICLE: &str = r#"<article>
<h1>Plain text that formats itself</h1>
<p>Most writing on the web starts life as <em>something other than HTML</em>. Notes, README files, chat messages and documentation pages are typed by people who would rather not close tags. Lightweight markup languages exist for them: the source stays readable as plain text, and a converter produces the HTML.</p>
<h2>Where the conventions came from</h2>
<p>Long before any formal syntax, people writing plain-text email had settled on a handful of habits:</p>
<ul>
<li>wrapping a word in asterisks to stress it,</li>
<li>starting a line with a dash or an asterisk to make a list item,</li>
<li>quoting a previous message by prefixing each line with <code>&gt;</code>,</li>
<li>underlining a title with a row of equals signs.</li>
</ul>
<p>Markdown collected these habits into a language in 2004. Its original description was informal, and implementations soon disagreed about corner cases such as nested lists and emphasis next to punctuation.</p>
<h2>A specification</h2>
<p>The <a href="https://spec.commonmark.org/">CommonMark specification</a> resolved those disagreements with a precise grammar and several hundred examples, each pairing Markdown with the HTML it must produce. Converters can now be tested against the same examples, which is how this playground's two libraries check their work.</p>
<hr>
<h2>Going the other way</h2>
<p>Converting HTML <strong>back</strong> to Markdown is harder, because HTML can express things Markdown cannot: attributes, arbitrary nesting, and inline styling. A converter must choose between two goals:</p>
<ol>
<li>Write clean Markdown that a person would have typed, and drop what does not fit.</li>
<li>Keep every detail by leaving untranslatable parts as raw HTML.</li>
</ol>
<p>htmd offers both, as its <em>pure</em> and <em>faithful</em> translation modes.<br>Try each on this article and compare the <strong>Rendered</strong> tab with the <strong>Original</strong> one.</p>
<h3>A small example</h3>
<p>The whole conversion is a single call:</p>
<pre><code class="language-rust">let html = "&lt;p&gt;Hello, &lt;em&gt;world&lt;/em&gt;!&lt;/p&gt;";
let markdown = htmd::convert(html)?;
assert_eq!(markdown, "Hello, *world*!");</code></pre>
<p>Options control the details: which character marks a bullet, whether headings are underlined, how code blocks are fenced, and whether links are written inline or as references at the end of the document.</p>
<h2>Why round trips matter</h2>
<p>When text can travel from Markdown to HTML and back without losing meaning, tools can store whichever form suits them. An editor can save Markdown, a browser extension can capture a page as HTML, and either can be turned into the other when needed. The <a href="https://github.com/letmutex/htmd">htmd repository</a> and the <a href="https://pulldown-cmark.github.io/pulldown-cmark/">pulldown-cmark guide</a> describe the details of each direction.</p>
</article>
"#;

const MARKDOWN_PRESETS: &[Preset] = &[
    Preset {
        name: "Basic",
        text: "# Hello\n\nType **Markdown** here and see it as *HTML* on the right.\n\n* a list\n* with [a link](https://commonmark.org)\n\n> and a quote\n\n```\nsome code\n```\n",
        extension: None,
    },
    markdown(
        Extension::Tables,
        "| Crate | Converts |\n| :--- | ---: |\n| htmd | HTML to Markdown |\n| pulldown-cmark | Markdown to HTML |\n",
    ),
    markdown(
        Extension::Math,
        "Inline: $e^{i\\pi} + 1 = 0$.\n\nDisplay:\n\n$$\\sum_{k=1}^{n} k = \\frac{n(n+1)}{2}$$\n",
    ),
    markdown(
        Extension::Footnotes,
        "A claim that needs a source.[^source]\n\n[^source]: The source, in a footnote.\n",
    ),
    markdown(
        Extension::OldFootnotes,
        "In the old syntax, a reference with no definition still becomes a link: [^missing]\n\nA defined one works in both.[^1]\n\n[^1]: The footnote.\n",
    ),
    markdown(
        Extension::Strikethrough,
        "The meeting is on ~~Tuesday~~ Wednesday.\n",
    ),
    markdown(
        Extension::TaskLists,
        "- [x] Write the spec\n- [x] Review it\n- [ ] Build the playground\n",
    ),
    markdown(
        Extension::SmartPunctuation,
        "\"Double\" and 'single' quotes -- en dash --- em dash...\n",
    ),
    markdown(
        Extension::HeadingAttributes,
        "# Introduction {#intro .lead}\n\nThe heading above has an id and a class.\n",
    ),
    markdown(
        Extension::YamlMetadata,
        "---\ntitle: An example\ntags: [demo, yaml]\n---\n\nThe metadata block above is not part of the HTML.\n",
    ),
    markdown(
        Extension::PlusesMetadata,
        "+++\ntitle = \"An example\"\ntags = [\"demo\", \"toml\"]\n+++\n\nThe metadata block above is not part of the HTML.\n",
    ),
    markdown(
        Extension::Gfm,
        "> [!NOTE]\n> Alerts are blockquotes with a type.\n\n> [!WARNING]\n> This one is a warning.\n",
    ),
    markdown(
        Extension::DefinitionLists,
        "Markdown\n: A lightweight markup language.\n\nHTML\n: The language of web pages.\n",
    ),
    markdown(
        Extension::Superscript,
        "Superscript needs space around it: E = mc ^2^, and ^this^ too.\n",
    ),
    markdown(
        Extension::Subscript,
        "Subscript needs space around it: x ~i~, and ~this~ too.\n",
    ),
    markdown(
        Extension::Wikilinks,
        "See [[Main Page]] or [[Markdown|the Markdown page]].\n",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_presets_follow_the_extension_table() {
        assert_eq!(MARKDOWN_PRESETS[0].label(), "Basic");
        let extensions: Vec<_> = MARKDOWN_PRESETS[1..]
            .iter()
            .map(|preset| preset.extension.unwrap())
            .collect();
        assert_eq!(extensions, Extension::ALL);
    }

    #[test]
    fn article_is_a_few_kilobytes_without_scripts() {
        assert!((2_000..8_000).contains(&ARTICLE.len()));
        assert!(!ARTICLE.contains("<script") && !ARTICLE.contains("<style"));
        for needed in ["<h2", "<ul", "<ol", "<a ", "<hr>", "<br>", "<pre>"] {
            assert!(ARTICLE.contains(needed), "{needed}");
        }
    }
}
