//! The playground's state as the user sees it: direction and the options of
//! both converters. Defaults follow the spec's options tables, not the
//! libraries' own defaults (see "htmd options" in `htmddemo2-spec.md`).

use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A dropdown choice: every value, its permalink key, and its visible label.
pub trait Choice: Copy + PartialEq + Send + Sync + 'static {
    /// Every value, in menu order.
    const ALL: &'static [Self];
    /// The stable name used in `<option value>` and in permalinks.
    fn key(self) -> &'static str;
    /// The label shown in the menu.
    fn label(self) -> &'static str;
}

macro_rules! choice {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $key:literal, $label:literal;)+ }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub enum $name {
            $(#[serde(rename = $key)] $variant,)+
        }

        /// The first value is the default.
        impl Default for $name {
            fn default() -> Self {
                Self::ALL[0]
            }
        }

        impl Choice for $name {
            const ALL: &'static [Self] = &[$(Self::$variant),+];
            fn key(self) -> &'static str {
                match self { $(Self::$variant => $key),+ }
            }
            fn label(self) -> &'static str {
                match self { $(Self::$variant => $label),+ }
            }
        }
    };
}

choice! {
    /// Which converter runs.
    Direction {
        HtmlToMarkdown = "html", "HTML → Markdown";
        MarkdownToHtml = "md", "Markdown → HTML";
    }
}

choice! {
    /// htmd's `TranslationMode`.
    TranslationMode {
        Pure = "pure", "Pure";
        Faithful = "faithful", "Faithful";
    }
}

choice! {
    /// htmd's `HeadingStyle`.
    HeadingStyle {
        Atx = "atx", "ATX (# Heading)";
        Setext = "setext", "Setext (underlined)";
    }
}

choice! {
    /// htmd's `HrStyle`.
    HrStyle {
        Asterisks = "asterisks", "* * *";
        Dashes = "dashes", "- - -";
        Underscores = "underscores", "_ _ _";
    }
}

choice! {
    /// htmd's `BrStyle`.
    BrStyle {
        TwoSpaces = "two-spaces", "two spaces";
        Backslash = "backslash", "backslash";
    }
}

choice! {
    /// htmd's `BulletListMarker`.
    BulletMarker {
        Asterisk = "asterisk", "*";
        Dash = "dash", "-";
    }
}

choice! {
    /// htmd's `CodeBlockStyle`.
    CodeBlockStyle {
        Fenced = "fenced", "fenced";
        Indented = "indented", "indented";
    }
}

choice! {
    /// htmd's `CodeBlockFence`.
    CodeFence {
        Backticks = "backticks", "```";
        Tildes = "tildes", "~~~";
    }
}

choice! {
    /// htmd's `LinkStyle`.
    LinkStyle {
        Inline = "inline", "inline";
        InlineAutolinks = "autolinks", "inline with autolinks";
        Referenced = "referenced", "referenced";
    }
}

choice! {
    /// htmd's `LinkReferenceStyle`.
    ReferenceStyle {
        Full = "full", "full ([t][1])";
        Collapsed = "collapsed", "collapsed ([t][])";
        Shortcut = "shortcut", "shortcut ([t])";
    }
}

impl Direction {
    /// The other direction.
    pub fn flipped(self) -> Self {
        match self {
            Self::HtmlToMarkdown => Self::MarkdownToHtml,
            Self::MarkdownToHtml => Self::HtmlToMarkdown,
        }
    }

    /// The name of the input language, used as the input pane's label.
    pub fn input_language(self) -> &'static str {
        match self {
            Self::HtmlToMarkdown => "HTML",
            Self::MarkdownToHtml => "Markdown",
        }
    }
}

/// The smallest and largest list spacing htmd accepts.
pub const SPACING_RANGE: std::ops::RangeInclusive<u8> = 1..=4;

/// htmd's options as shown in the options panel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct HtmdSettings {
    pub translation_mode: TranslationMode,
    pub heading_style: HeadingStyle,
    pub horizontal_rule: HrStyle,
    pub line_break: BrStyle,
    pub bullet_marker: BulletMarker,
    pub bullet_spacing: u8,
    pub number_spacing: u8,
    pub code_block_style: CodeBlockStyle,
    pub code_fence: CodeFence,
    pub link_style: LinkStyle,
    pub reference_style: ReferenceStyle,
    pub preserve_code_whitespace: bool,
    pub noscript_raw_text: bool,
    /// The Skip tags field as typed; see [`crate::convert::parse_skip_tags`].
    pub skip_tags: String,
}

impl Default for HtmdSettings {
    fn default() -> Self {
        Self {
            translation_mode: TranslationMode::Pure,
            heading_style: HeadingStyle::Atx,
            horizontal_rule: HrStyle::Asterisks,
            line_break: BrStyle::TwoSpaces,
            bullet_marker: BulletMarker::Asterisk,
            bullet_spacing: 3,
            number_spacing: 2,
            code_block_style: CodeBlockStyle::Fenced,
            code_fence: CodeFence::Backticks,
            link_style: LinkStyle::Inline,
            reference_style: ReferenceStyle::Full,
            preserve_code_whitespace: false,
            noscript_raw_text: true,
            skip_tags: String::new(),
        }
    }
}

impl HtmdSettings {
    /// Whether every value is one the options panel can produce.
    pub fn is_valid(&self) -> bool {
        SPACING_RANGE.contains(&self.bullet_spacing) && SPACING_RANGE.contains(&self.number_spacing)
    }

    /// Translate to htmd's own options.
    pub fn to_htmd(&self) -> htmd::options::Options {
        use htmd::options as o;
        o::Options {
            heading_style: match self.heading_style {
                HeadingStyle::Atx => o::HeadingStyle::Atx,
                HeadingStyle::Setext => o::HeadingStyle::Setex,
            },
            hr_style: match self.horizontal_rule {
                HrStyle::Asterisks => o::HrStyle::Asterisks,
                HrStyle::Dashes => o::HrStyle::Dashes,
                HrStyle::Underscores => o::HrStyle::Underscores,
            },
            br_style: match self.line_break {
                BrStyle::TwoSpaces => o::BrStyle::TwoSpaces,
                BrStyle::Backslash => o::BrStyle::Backslash,
            },
            link_style: match self.link_style {
                LinkStyle::Inline => o::LinkStyle::Inlined,
                LinkStyle::InlineAutolinks => o::LinkStyle::InlinedPreferAutolinks,
                LinkStyle::Referenced => o::LinkStyle::Referenced,
            },
            link_reference_style: match self.reference_style {
                ReferenceStyle::Full => o::LinkReferenceStyle::Full,
                ReferenceStyle::Collapsed => o::LinkReferenceStyle::Collapsed,
                ReferenceStyle::Shortcut => o::LinkReferenceStyle::Shortcut,
            },
            code_block_style: match self.code_block_style {
                CodeBlockStyle::Fenced => o::CodeBlockStyle::Fenced,
                CodeBlockStyle::Indented => o::CodeBlockStyle::Indented,
            },
            code_block_fence: match self.code_fence {
                CodeFence::Backticks => o::CodeBlockFence::Backticks,
                CodeFence::Tildes => o::CodeBlockFence::Tildes,
            },
            bullet_list_marker: match self.bullet_marker {
                BulletMarker::Asterisk => o::BulletListMarker::Asterisk,
                BulletMarker::Dash => o::BulletListMarker::Dash,
            },
            ul_bullet_spacing: self.bullet_spacing,
            ol_number_spacing: self.number_spacing,
            preformatted_code: self.preserve_code_whitespace,
            translation_mode: match self.translation_mode {
                TranslationMode::Pure => o::TranslationMode::Pure,
                TranslationMode::Faithful => o::TranslationMode::Faithful,
            },
        }
    }
}

/// A pulldown-cmark extension, one checkbox each, in the spec's table order.
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
    /// Every extension, in checkbox order.
    pub const ALL: [Self; 15] = [
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

    fn index(self) -> usize {
        self as usize
    }

    /// The stable name used in permalinks.
    pub fn key(self) -> &'static str {
        match self {
            Self::Tables => "tables",
            Self::Math => "math",
            Self::Footnotes => "footnotes",
            Self::OldFootnotes => "old-footnotes",
            Self::Strikethrough => "strikethrough",
            Self::TaskLists => "tasklists",
            Self::SmartPunctuation => "smart-punctuation",
            Self::HeadingAttributes => "heading-attributes",
            Self::YamlMetadata => "yaml-metadata",
            Self::PlusesMetadata => "pluses-metadata",
            Self::Gfm => "gfm",
            Self::DefinitionLists => "definition-lists",
            Self::Superscript => "superscript",
            Self::Subscript => "subscript",
            Self::Wikilinks => "wikilinks",
        }
    }

    /// The checkbox label, also the name of the extension's preset.
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

    /// One line of help, based on pulldown-cmark's documentation.
    pub fn help(self) -> &'static str {
        match self {
            Self::Tables => "GitHub-style pipe tables.",
            Self::Math => "$inline$ and $$display$$ math, shown as formula text.",
            Self::Footnotes => "GitHub-compatible footnotes: [^1] and [^1]: text.",
            Self::OldFootnotes => {
                "The older footnote syntax (differs in indentation, nesting and undefined references); implies footnotes."
            }
            Self::Strikethrough => "~~deleted~~ text.",
            Self::TaskLists => "- [x] checkbox list items.",
            Self::SmartPunctuation => "Curly quotes, en and em dashes, and ellipses.",
            Self::HeadingAttributes => "# Heading {#id .class} attributes.",
            Self::YamlMetadata => "A --- delimited YAML block at the start, left out of the HTML.",
            Self::PlusesMetadata => {
                "A +++ delimited metadata block at the start, left out of the HTML."
            }
            Self::Gfm => "Misc GitHub features: > [!NOTE] style blockquote alerts.",
            Self::DefinitionLists => "Term lines followed by : definition lines.",
            Self::Superscript => "^superscript^ text.",
            Self::Subscript => "~subscript~ text (without it, ~text~ is strikethrough).",
            Self::Wikilinks => "Obsidian-style [[Page]] and [[Page|text]] links.",
        }
    }

    /// The default from the spec's extension table.
    pub fn default_enabled(self) -> bool {
        matches!(self, Self::Tables | Self::Math)
    }

    fn flag(self) -> pulldown_cmark::Options {
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
}

/// pulldown-cmark's extensions as shown in the options panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MdSettings {
    enabled: [bool; 15],
}

impl Default for MdSettings {
    fn default() -> Self {
        Self {
            enabled: Extension::ALL.map(Extension::default_enabled),
        }
    }
}

impl MdSettings {
    /// Whether the extension's checkbox is checked.
    pub fn get(&self, extension: Extension) -> bool {
        self.enabled[extension.index()]
    }

    /// Check or uncheck a box, keeping old-style footnotes dependent on
    /// footnotes as the spec's Options section requires.
    pub fn set(&mut self, extension: Extension, on: bool) {
        self.enabled[extension.index()] = on;
        match (extension, on) {
            (Extension::OldFootnotes, true) => self.enabled[Extension::Footnotes.index()] = true,
            (Extension::Footnotes, false) => self.enabled[Extension::OldFootnotes.index()] = false,
            _ => {}
        }
    }

    /// Whether the checkboxes are in a state the panel can produce.
    pub fn is_valid(&self) -> bool {
        !self.get(Extension::OldFootnotes) || self.get(Extension::Footnotes)
    }

    /// Translate to pulldown-cmark's option flags.
    pub fn to_pulldown(&self) -> pulldown_cmark::Options {
        Extension::ALL
            .into_iter()
            .filter(|extension| self.get(*extension))
            .fold(pulldown_cmark::Options::empty(), |flags, extension| {
                flags | extension.flag()
            })
    }
}

impl Serialize for MdSettings {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(Extension::ALL.len()))?;
        for extension in Extension::ALL {
            map.serialize_entry(extension.key(), &self.get(extension))?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for MdSettings {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ExtensionsVisitor;

        impl<'de> Visitor<'de> for ExtensionsVisitor {
            type Value = MdSettings;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map from extension name to bool")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<MdSettings, A::Error> {
                let mut settings = MdSettings::default();
                while let Some(key) = map.next_key::<String>()? {
                    let extension = Extension::ALL
                        .into_iter()
                        .find(|extension| extension.key() == key)
                        .ok_or_else(|| de::Error::unknown_field(&key, &[]))?;
                    settings.enabled[extension.index()] = map.next_value()?;
                }
                Ok(settings)
            }
        }

        deserializer.deserialize_map(ExtensionsVisitor)
    }
}

/// One line of help for each htmd control, based on htmd's documentation.
pub mod htmd_help {
    pub const TRANSLATION_MODE: &str = "Pure writes plain Markdown, dropping what it cannot express; Faithful keeps such HTML as raw HTML.";
    pub const HEADING_STYLE: &str = "How to write headings; Setext applies only to levels 1 and 2.";
    pub const HORIZONTAL_RULE: &str = "How to write <hr>.";
    pub const LINE_BREAK: &str = "How to write <br>.";
    pub const LINE_BREAK_FAITHFUL: &str = "Faithful mode always writes <br>.";
    pub const BULLET_MARKER: &str = "The character that starts bullet list items.";
    pub const BULLET_SPACING: &str = "Spaces between the bullet and the item's content.";
    pub const NUMBER_SPACING: &str =
        "Spaces between the period and the content of the widest-numbered item.";
    pub const CODE_BLOCK_STYLE: &str = "How to write <pre> code blocks.";
    pub const CODE_FENCE: &str = "The fence around fenced code blocks.";
    pub const LINK_STYLE: &str =
        "Inline links, autolinks where the text is the URL, or reference links.";
    pub const REFERENCE_STYLE: &str = "How reference links name their definition.";
    pub const PRESERVE_CODE_WHITESPACE: &str = "Keep the whitespace in inline <code> as it is.";
    pub const NOSCRIPT_RAW_TEXT: &str =
        "Parse <noscript> content as text (as a browser with scripts on does), not as HTML.";
    pub const SKIP_TAGS: &str =
        "Elements to remove with their content, separated by commas or spaces.";
}
