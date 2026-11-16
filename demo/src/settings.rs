//! The direction and every option in the options panel. The defaults are the
//! spec's tables, not htmd's `Options::default()`, so that a later htmd which
//! changes a default does not change the playground.

use htmd::options::{
    BrStyle, BulletListMarker, CodeBlockFence, CodeBlockStyle, HeadingStyle, HrStyle,
    LinkReferenceStyle, LinkStyle, Options, TranslationMode,
};

/// Which converter runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    /// htmd.
    #[default]
    HtmlToMarkdown,
    /// pulldown-cmark.
    MarkdownToHtml,
}

impl Direction {
    /// The other direction.
    pub fn flipped(self) -> Self {
        match self {
            Self::HtmlToMarkdown => Self::MarkdownToHtml,
            Self::MarkdownToHtml => Self::HtmlToMarkdown,
        }
    }

    /// The input pane's label.
    pub fn input_language(self) -> &'static str {
        match self {
            Self::HtmlToMarkdown => "HTML",
            Self::MarkdownToHtml => "Markdown",
        }
    }

    /// The output language, shown by the **Source** tab.
    pub fn output_language(self) -> &'static str {
        self.flipped().input_language()
    }
}

/// htmd's options plus the two builder settings the panel exposes.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmdSettings {
    /// Passed to `HtmlToMarkdownBuilder::options`.
    pub options: Options,
    /// Passed to `HtmlToMarkdownBuilder::scripting_enabled`.
    pub noscript_as_raw_text: bool,
    /// The **Skip tags** field as typed; see [`crate::convert::parse_skip_tags`].
    pub skip_tags: String,
}

impl Default for HtmdSettings {
    fn default() -> Self {
        Self {
            options: Options {
                heading_style: HeadingStyle::Atx,
                hr_style: HrStyle::Asterisks,
                br_style: BrStyle::TwoSpaces,
                link_style: LinkStyle::Inlined,
                link_reference_style: LinkReferenceStyle::Full,
                code_block_style: CodeBlockStyle::Fenced,
                code_block_fence: CodeBlockFence::Backticks,
                bullet_list_marker: BulletListMarker::Asterisk,
                ul_bullet_spacing: 3,
                ol_number_spacing: 2,
                preformatted_code: false,
                translation_mode: TranslationMode::Pure,
            },
            noscript_as_raw_text: true,
            skip_tags: String::new(),
        }
    }
}

/// Clamp a list spacing to the 1–4 range the panel allows.
pub fn clamp_spacing(value: i64) -> u8 {
    value.clamp(1, 4) as u8
}

/// A pulldown-cmark extension, in the order of the spec's extension table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extension {
    Tables,
    Math,
    Footnotes,
    OldFootnotes,
    Strikethrough,
    TaskLists,
    SmartPunctuation,
    HeadingAttributes,
    YamlMetadata,
    PlusesMetadata,
    Gfm,
    DefinitionLists,
    Superscript,
    Subscript,
    Wikilinks,
}

impl Extension {
    /// Every extension, in table order.
    pub const ALL: [Extension; 15] = [
        Self::Tables,
        Self::Math,
        Self::Footnotes,
        Self::OldFootnotes,
        Self::Strikethrough,
        Self::TaskLists,
        Self::SmartPunctuation,
        Self::HeadingAttributes,
        Self::YamlMetadata,
        Self::PlusesMetadata,
        Self::Gfm,
        Self::DefinitionLists,
        Self::Superscript,
        Self::Subscript,
        Self::Wikilinks,
    ];

    /// The checkbox label, also the name of the extension's Markdown preset.
    pub fn label(self) -> &'static str {
        match self {
            Self::Tables => "Tables",
            Self::Math => "Math",
            Self::Footnotes => "Footnotes",
            Self::OldFootnotes => "Old-style footnotes",
            Self::Strikethrough => "Strikethrough",
            Self::TaskLists => "Task lists",
            Self::SmartPunctuation => "Smart punctuation",
            Self::HeadingAttributes => "Heading attributes",
            Self::YamlMetadata => "YAML metadata blocks",
            Self::PlusesMetadata => "+++ metadata blocks",
            Self::Gfm => "GFM",
            Self::DefinitionLists => "Definition lists",
            Self::Superscript => "Superscript",
            Self::Subscript => "Subscript",
            Self::Wikilinks => "Wikilinks",
        }
    }

    /// One-line help, condensed from pulldown-cmark's `Options` docs.
    pub fn help(self) -> &'static str {
        match self {
            Self::Tables => "GitHub-style pipe tables.",
            Self::Math => "$inline$ and $$display$$ math, output as its formula text.",
            Self::Footnotes => "Footnote references [^1] and definitions [^1]: text.",
            Self::OldFootnotes => {
                "Footnotes as parsed before pulldown-cmark 0.10; implies footnotes."
            }
            Self::Strikethrough => "~~Strikethrough~~ text.",
            Self::TaskLists => "List items starting with [ ] or [x] become checkboxes.",
            Self::SmartPunctuation => "Curly quotes, en and em dashes, and ellipses.",
            Self::HeadingAttributes => "Heading ids and classes: # Title {#id .class}.",
            Self::YamlMetadata => "A YAML block between --- lines at the start is metadata.",
            Self::PlusesMetadata => "A block between +++ lines at the start is metadata.",
            Self::Gfm => "GitHub blockquote alerts such as > [!NOTE].",
            Self::DefinitionLists => "A term followed by lines starting with : definitions.",
            Self::Superscript => "^Superscript^ text.",
            Self::Subscript => "~Subscript~ text.",
            Self::Wikilinks => "[[Page]] and [[Page|label]] links.",
        }
    }

    /// The pulldown-cmark flag; old-style footnotes' flag includes footnotes'.
    pub fn flag(self) -> pulldown_cmark::Options {
        use pulldown_cmark::Options as O;
        match self {
            Self::Tables => O::ENABLE_TABLES,
            Self::Math => O::ENABLE_MATH,
            Self::Footnotes => O::ENABLE_FOOTNOTES,
            Self::OldFootnotes => O::ENABLE_OLD_FOOTNOTES,
            Self::Strikethrough => O::ENABLE_STRIKETHROUGH,
            Self::TaskLists => O::ENABLE_TASKLISTS,
            Self::SmartPunctuation => O::ENABLE_SMART_PUNCTUATION,
            Self::HeadingAttributes => O::ENABLE_HEADING_ATTRIBUTES,
            Self::YamlMetadata => O::ENABLE_YAML_STYLE_METADATA_BLOCKS,
            Self::PlusesMetadata => O::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS,
            Self::Gfm => O::ENABLE_GFM,
            Self::DefinitionLists => O::ENABLE_DEFINITION_LIST,
            Self::Superscript => O::ENABLE_SUPERSCRIPT,
            Self::Subscript => O::ENABLE_SUBSCRIPT,
            Self::Wikilinks => O::ENABLE_WIKILINKS,
        }
    }

    /// On by default: htmd writes tables and math (spec: pulldown-cmark
    /// options).
    pub fn default_enabled(self) -> bool {
        matches!(self, Self::Tables | Self::Math)
    }
}

/// Which pulldown-cmark extensions are checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CmarkSettings {
    enabled: [bool; Extension::ALL.len()],
}

impl Default for CmarkSettings {
    fn default() -> Self {
        Self {
            enabled: Extension::ALL.map(Extension::default_enabled),
        }
    }
}

impl CmarkSettings {
    /// Whether `extension` is checked.
    pub fn is_enabled(&self, extension: Extension) -> bool {
        self.enabled[extension as usize]
    }

    /// Check or uncheck `extension`. Old-style footnotes requires footnotes,
    /// so checking it checks footnotes and unchecking footnotes unchecks it.
    pub fn set(&mut self, extension: Extension, on: bool) {
        self.enabled[extension as usize] = on;
        match (extension, on) {
            (Extension::OldFootnotes, true) => self.enabled[Extension::Footnotes as usize] = true,
            (Extension::Footnotes, false) => self.enabled[Extension::OldFootnotes as usize] = false,
            _ => {}
        }
    }

    /// Check or uncheck `extension` alone, without the footnotes rule.
    pub fn set_raw(&mut self, extension: Extension, on: bool) {
        self.enabled[extension as usize] = on;
    }

    /// The flags to pass to `pulldown_cmark::Parser::new_ext`.
    pub fn to_options(&self) -> pulldown_cmark::Options {
        Extension::ALL
            .into_iter()
            .filter(|extension| self.is_enabled(*extension))
            .fold(pulldown_cmark::Options::empty(), |flags, extension| {
                flags | extension.flag()
            })
    }
}

/// Everything a permalink records apart from the input.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    /// The direction switch.
    pub direction: Direction,
    /// The htmd group of the options panel.
    pub htmd: HtmdSettings,
    /// The pulldown-cmark group of the options panel.
    pub cmark: CmarkSettings,
}

/// A dropdown choice. `key` is both the `<option>` value and the permalink
/// value, so it must never change (spec: Permalinks).
pub struct Choice<T: 'static> {
    /// The option value.
    pub value: T,
    /// The stable identifier.
    pub key: &'static str,
    /// The text shown in the dropdown.
    pub label: &'static str,
}

impl<T: Copy + PartialEq> Choice<T> {
    /// The key of `value` in `choices`.
    pub fn key_of(choices: &[Choice<T>], value: T) -> &'static str {
        choices
            .iter()
            .find(|choice| choice.value == value)
            .map_or("", |choice| choice.key)
    }

    /// The value whose key is `key`.
    pub fn value_of(choices: &[Choice<T>], key: &str) -> Option<T> {
        choices
            .iter()
            .find(|choice| choice.key == key)
            .map(|choice| choice.value)
    }
}

const fn choice<T>(value: T, key: &'static str, label: &'static str) -> Choice<T> {
    Choice { value, key, label }
}

/// **Translation mode** choices.
pub static TRANSLATION_MODES: [Choice<TranslationMode>; 2] = [
    choice(TranslationMode::Pure, "pure", "Pure"),
    choice(TranslationMode::Faithful, "faithful", "Faithful"),
];

/// **Heading style** choices.
pub static HEADING_STYLES: [Choice<HeadingStyle>; 2] = [
    choice(HeadingStyle::Atx, "atx", "ATX (# Heading)"),
    choice(HeadingStyle::Setex, "setext", "Setext (underlined)"),
];

/// **Horizontal rule** choices.
pub static HR_STYLES: [Choice<HrStyle>; 3] = [
    choice(HrStyle::Asterisks, "asterisks", "* * *"),
    choice(HrStyle::Dashes, "dashes", "- - -"),
    choice(HrStyle::Underscores, "underscores", "_ _ _"),
];

/// **Line break** choices.
pub static BR_STYLES: [Choice<BrStyle>; 2] = [
    choice(BrStyle::TwoSpaces, "spaces", "Two spaces"),
    choice(BrStyle::Backslash, "backslash", "Backslash"),
];

/// **Bullet marker** choices.
pub static BULLET_MARKERS: [Choice<BulletListMarker>; 2] = [
    choice(BulletListMarker::Asterisk, "asterisk", "*"),
    choice(BulletListMarker::Dash, "dash", "-"),
];

/// **Code block style** choices.
pub static CODE_BLOCK_STYLES: [Choice<CodeBlockStyle>; 2] = [
    choice(CodeBlockStyle::Fenced, "fenced", "Fenced"),
    choice(CodeBlockStyle::Indented, "indented", "Indented"),
];

/// **Code fence** choices.
pub static CODE_FENCES: [Choice<CodeBlockFence>; 2] = [
    choice(CodeBlockFence::Backticks, "backticks", "```"),
    choice(CodeBlockFence::Tildes, "tildes", "~~~"),
];

/// **Link style** choices.
pub static LINK_STYLES: [Choice<LinkStyle>; 3] = [
    choice(LinkStyle::Inlined, "inline", "Inline"),
    choice(
        LinkStyle::InlinedPreferAutolinks,
        "autolinks",
        "Inline with autolinks",
    ),
    choice(LinkStyle::Referenced, "referenced", "Referenced"),
];

/// **Reference style** choices.
pub static REFERENCE_STYLES: [Choice<LinkReferenceStyle>; 3] = [
    choice(LinkReferenceStyle::Full, "full", "Full ([t][1])"),
    choice(
        LinkReferenceStyle::Collapsed,
        "collapsed",
        "Collapsed ([t][])",
    ),
    choice(LinkReferenceStyle::Shortcut, "shortcut", "Shortcut ([t])"),
];
