//! The **Examples** menu presets (spec: Examples).

use crate::settings::{Direction, Extension};

/// One entry in the **Examples** menu.
pub struct Example {
    /// The menu label.
    pub name: &'static str,
    /// The text that replaces the input.
    pub text: &'static str,
    /// The extension a Markdown preset demonstrates; picking the preset checks
    /// it.
    pub extension: Option<Extension>,
}

const fn html(name: &'static str, text: &'static str) -> Example {
    Example {
        name,
        text,
        extension: None,
    }
}

const fn markdown(extension: Extension, text: &'static str) -> Example {
    Example {
        name: "",
        text,
        extension: Some(extension),
    }
}

impl Example {
    /// The menu label; a Markdown extension preset is named after its
    /// checkbox.
    pub fn label(&self) -> &'static str {
        match self.extension {
            Some(extension) => extension.label(),
            None => self.name,
        }
    }
}

/// The presets for `direction`, **Basic** first.
pub fn examples(direction: Direction) -> &'static [Example] {
    match direction {
        Direction::HtmlToMarkdown => &HTML_EXAMPLES,
        Direction::MarkdownToHtml => &MARKDOWN_EXAMPLES,
    }
}

/// The page opens with this when the address carries no saved state.
pub const DEFAULT_EXAMPLE: &str = HTML_EXAMPLES[0].text;

/// Whether `text` is one of the presets of either direction, which **Clear**
/// and **Examples** may replace without asking.
pub fn is_preset(text: &str) -> bool {
    HTML_EXAMPLES
        .iter()
        .chain(MARKDOWN_EXAMPLES.iter())
        .any(|example| example.text == text)
}

/// The HTML presets. **Basic** is written exactly as pulldown-cmark writes the
/// same paragraph, so that switching direction twice gives it back.
pub static HTML_EXAMPLES: [Example; 10] = [
    html(
        "Basic",
        "<p>Type <strong>HTML</strong> here to see it as <em>Markdown</em>.</p>",
    ),
    html(
        "Headings",
        "<h1>Main title</h1>
<p>An introductory paragraph.</p>
<h2>A section</h2>
<p>Text in the section.</p>
<h3>A subsection</h3>
<p>Text in the subsection.</p>
",
    ),
    html(
        "Lists",
        "<ul>
  <li>Fruit
    <ul>
      <li>Apple</li>
      <li>Pear
        <ul>
          <li>Conference</li>
        </ul>
      </li>
    </ul>
  </li>
  <li>Vegetables</li>
</ul>
<ol start=\"3\">
  <li>Third</li>
  <li>Fourth</li>
  <li>Fifth</li>
</ol>
",
    ),
    html(
        "Links and images",
        "<p>A link whose text is its URL: <a href=\"https://example.com/\">https://example.com/</a></p>
<p>A titled link: <a href=\"https://commonmark.org/\" title=\"CommonMark home\">CommonMark</a></p>
<p>An image: <img src=\"https://example.com/cat.png\" alt=\"A sleeping cat\"></p>
",
    ),
    html(
        "Code",
        "<p>Run <code> cargo test </code> to check your work.</p>
<pre><code class=\"language-rust\">fn main() {
    let greeting = \"Hello, world!\";
    println!(\"{greeting}\");
}
</code></pre>
",
    ),
    html(
        "Tables",
        "<table>
  <thead>
    <tr><th>Crate</th><th>Converts</th></tr>
  </thead>
  <tbody>
    <tr><td>htmd</td><td>HTML to Markdown</td></tr>
    <tr><td>pulldown-cmark</td><td>Markdown to HTML</td></tr>
  </tbody>
</table>
",
    ),
    html(
        "Blockquotes",
        "<blockquote>
  <p>The outer quote.</p>
  <blockquote>
    <p>A quote inside the quote.</p>
    <blockquote>
      <p>And one level deeper.</p>
    </blockquote>
  </blockquote>
</blockquote>
",
    ),
    html(
        "Math",
        "<p>Inline math: <span class=\"math math-inline\">E = mc^2</span>.</p>
<p>Display math:</p>
<p><span class=\"math math-display\">\\int_0^1 x^2 \\, dx = \\frac{1}{3}</span></p>
",
    ),
    // Original text written for the playground, distributed under the
    // repository's license.
    html("Article", ARTICLE),
    html(
        "Edge cases",
        "<p>Empty elements: <b></b><em></em><span></span><a></a> end.</p>
<p></p>
<div></div>
<p>Entities: &amp; &lt; &gt; &quot; &copy; &mdash; &#169; &#x1F600; a&nbsp;b</p>
<p>Runs    of     spaces,
\t\ttabs,

   and newlines.</p>
<!-- A comment, which does not appear in the output. -->
<p>Before the script.</p>
<script>document.title = \"This never runs\";</script>
<style>p { color: red; }</style>
<noscript><p>Shown <b>only</b> without scripts.</p></noscript>
",
    ),
];

const ARTICLE: &str = "<article>
<h1>How a document changes format</h1>
<p>Text rarely stays in one format for long. A note starts as Markdown in an
editor, is published as HTML on a web page, and is later copied back into
Markdown by someone who wants to quote it. Each step is a
<em>translation</em>, and like any translation it can lose something on the
way.</p>

<h2>Two languages for the same ideas</h2>
<p>HTML and Markdown describe many of the same things: headings, paragraphs,
lists, links, emphasis, and code. Markdown writes them with punctuation that
reads naturally as plain text, while HTML writes them with tags. The
<a href=\"https://spec.commonmark.org/\">CommonMark specification</a> defines
exactly how Markdown maps onto HTML, which makes the trip from Markdown to HTML
predictable.</p>
<p>The trip back is harder, because HTML can say more than Markdown can. A
converter must decide what to do with:</p>
<ul>
  <li>attributes, such as classes and inline styles;</li>
  <li>elements with no Markdown form, such as <code>&lt;sup&gt;</code>;</li>
  <li>whitespace, which HTML mostly ignores but Markdown treats as meaningful.</li>
</ul>

<h2>Choosing what to keep</h2>
<p>There are two reasonable answers. A <strong>pure</strong> translation always
writes plain Markdown and drops whatever Markdown cannot express. A
<strong>faithful</strong> translation keeps that content as raw HTML inside the
Markdown, so that converting back gives nearly the original page.</p>
<p>Neither answer is always right. Pure output is easier to read and edit;
faithful output loses less.<br>
The choice depends on whether people or programs will read the result.</p>

<h2>Checking a converter</h2>
<p>A simple way to test a converter is a round trip:</p>
<ol>
  <li>Convert a page from HTML to Markdown.</li>
  <li>Convert the Markdown back to HTML with a trusted parser.</li>
  <li>Compare the two pages side by side.</li>
</ol>
<p>Differences show what was lost. For example, this small function turns a
title into a heading in both formats:</p>
<pre><code class=\"language-rust\">fn heading(title: &amp;str) -&gt; (String, String) {
    let html = format!(\"&lt;h1&gt;{title}&lt;/h1&gt;\");
    let markdown = format!(\"# {title}\");
    (html, markdown)
}
</code></pre>

<hr>

<h2>Further reading</h2>
<p>The <a href=\"https://commonmark.org/help/\" title=\"Learn Markdown in 60 seconds\">CommonMark
help page</a> is a short introduction to the syntax, and the
<a href=\"https://html.spec.whatwg.org/\">HTML Living Standard</a> describes
every element a converter might meet.</p>
</article>
";

/// The Markdown presets: **Basic**, then one per extension in table order.
pub static MARKDOWN_EXAMPLES: [Example; 16] = [
    Example {
        name: "Basic",
        text: "# A heading

A paragraph with *emphasis*, **strong emphasis**, `code`, and a
[link](https://commonmark.org/).

* A bullet list
* with two items

> A blockquote.
",
        extension: None,
    },
    markdown(
        Extension::Tables,
        "| Crate | Converts |
| --- | :---: |
| htmd | HTML to Markdown |
| pulldown-cmark | Markdown to HTML |
",
    ),
    markdown(
        Extension::Math,
        "Inline math: $E = mc^2$.

Display math: $$\\int_0^1 x^2 \\, dx = \\frac{1}{3}$$
",
    ),
    markdown(
        Extension::Footnotes,
        "A sentence with a footnote.[^note]

[^note]: The footnote's text.
",
    ),
    markdown(
        Extension::OldFootnotes,
        "Two footnotes[^1] and an undefined one.[^9]

[^1]: In the old syntax, these two lines
[^2]: are a single footnote definition.
",
    ),
    markdown(
        Extension::Strikethrough,
        "This is ~~not~~ the right answer.\n",
    ),
    markdown(
        Extension::TaskLists,
        "- [x] Write the spec
- [ ] Build the playground
",
    ),
    markdown(
        Extension::SmartPunctuation,
        "\"Double\" and 'single' quotes -- an en dash --- an em dash... and an ellipsis.\n",
    ),
    markdown(
        Extension::HeadingAttributes,
        "# A heading with an id and a class {#intro .title}\n",
    ),
    markdown(
        Extension::YamlMetadata,
        "---
title: A document with YAML metadata
---

Only this paragraph is shown.
",
    ),
    markdown(
        Extension::PlusesMetadata,
        "+++
title = \"A document with TOML metadata\"
+++

Only this paragraph is shown.
",
    ),
    markdown(
        Extension::Gfm,
        "> [!NOTE]
> A GitHub-style note.
",
    ),
    markdown(
        Extension::DefinitionLists,
        "Markdown
: A lightweight markup language.
",
    ),
    markdown(Extension::Superscript, "Text with ^superscript^ in it.\n"),
    markdown(Extension::Subscript, "Text with ~subscript~ in it.\n"),
    markdown(
        Extension::Wikilinks,
        "See [[Main Page]] or [[Main Page|the start]].\n",
    ),
];
