//! The playground page (spec: Page layout).

use std::sync::Arc;
use std::time::Duration;

use htmd::options::{CodeBlockStyle, LinkStyle, TranslationMode};
use htmd_playground::LARGE_INPUT_BYTES;
use htmd_playground::convert::{convert, markdown_to_html, parse_skip_tags};
use htmd_playground::examples::{DEFAULT_EXAMPLE, Example, examples, is_preset};
use htmd_playground::permalink::{self, PageState};
use htmd_playground::preview::preview_document;
use htmd_playground::settings::{
    BR_STYLES, BULLET_MARKERS, CODE_BLOCK_STYLES, CODE_FENCES, Choice, CmarkSettings, Direction,
    Extension, HEADING_STYLES, HR_STYLES, LINK_STYLES, REFERENCE_STYLES, Settings,
    TRANSLATION_MODES, clamp_spacing,
};
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::browser;

/// The pause in typing after which the output updates (spec: Conversion).
const TYPING_PAUSE: Duration = Duration::from_millis(100);

const INVALID_LINK: &str = "This link was invalid, so the default example is shown.";
const TOO_LARGE_TO_SHARE: &str =
    "The input is too large to share; the address keeps only the direction and options.";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Source,
    Rendered,
    Original,
}

/// The last conversion, which every tab shows until the next one.
#[derive(Clone, Default)]
struct Output {
    direction: Direction,
    input: Arc<str>,
    source: Arc<str>,
    cmark: CmarkSettings,
    /// Absent for the empty output left when large input arrives at once.
    stats: Option<(f64, usize)>,
}

impl Output {
    fn empty(direction: Direction) -> Self {
        Self {
            direction,
            ..Self::default()
        }
    }
}

/// The page state, shared by every handler.
#[derive(Clone, Copy)]
struct Playground {
    settings: RwSignal<Settings>,
    input: RwSignal<String>,
    output: RwSignal<Output>,
    /// "Convert anyway" was pressed since the input last exceeded 1 MiB.
    convert_large_input: RwSignal<bool>,
    crashed: RwSignal<bool>,
    tab: RwSignal<Tab>,
    link_notice: RwSignal<Option<&'static str>>,
    copy_notice: RwSignal<Option<&'static str>>,
    /// The address shown for copying by hand when Share cannot copy.
    share_fallback: RwSignal<Option<String>>,
    pending: StoredValue<Option<TimeoutHandle>>,
    editor: NodeRef<html::Textarea>,
}

impl Playground {
    fn input_is_large(&self) -> bool {
        self.input.with(|input| input.len() > LARGE_INPUT_BYTES)
    }

    fn paused(&self) -> bool {
        self.input_is_large() && !self.convert_large_input.get()
    }

    /// The output may not match the input (spec: Stale output).
    fn stale(&self) -> bool {
        self.paused() || self.crashed.get()
    }

    fn halted(&self) -> bool {
        if browser::crash_reported() {
            self.crashed.set(true);
        }
        self.crashed.get_untracked()
    }

    fn cancel_pending(&self) {
        if let Some(handle) = self.pending.get_value() {
            handle.clear();
        }
        self.pending.set_value(None);
    }

    /// Convert the input into the output, unless paused or crashed.
    fn convert(&self) {
        if self.halted() || untrack(|| self.paused()) {
            return;
        }
        let settings = self.settings.get_untracked();
        let input: Arc<str> = self.input.with_untracked(|input| Arc::from(input.as_str()));
        let start = browser::now();
        let result = convert(&input, &settings);
        let millis = browser::now() - start;
        match result {
            Ok(source) => {
                let characters = source.chars().count();
                self.output.set(Output {
                    direction: settings.direction,
                    input,
                    source: source.into(),
                    cmark: settings.cmark,
                    stats: Some((millis, characters)),
                });
            }
            Err(error) => {
                self.crashed.set(true);
                browser::show_crash(&error.to_string());
            }
        }
    }

    /// Record the state in the address (spec: Permalinks).
    fn update_address(&self) {
        let base_len = browser::address_base().len();
        let fragment = self.settings.with_untracked(|settings| {
            self.input
                .with_untracked(|input| permalink::encode(settings, input, base_len))
        });
        browser::replace_fragment(&fragment.text);
        self.link_notice
            .set(fragment.input_dropped.then_some(TOO_LARGE_TO_SHARE));
    }

    fn run_pending(&self) {
        self.pending.set_value(None);
        self.convert();
        self.update_address();
    }

    /// Apply keystrokes still inside the typing pause.
    fn flush(&self) {
        if self.pending.get_value().is_some() {
            self.cancel_pending();
            self.run_pending();
        }
    }

    fn on_typing(&self, text: String) {
        if self.halted() {
            return;
        }
        self.input.set(text);
        if !untrack(|| self.input_is_large()) {
            self.convert_large_input.set(false);
        }
        self.copy_notice.set(None);
        self.cancel_pending();
        let playground = *self;
        self.pending.set_value(
            set_timeout_with_handle(move || playground.run_pending(), TYPING_PAUSE).ok(),
        );
    }

    /// Change options, then convert at once (spec: Conversion).
    fn change(&self, change: impl FnOnce(&mut Settings)) {
        self.settings.update(change);
        self.cancel_pending();
        self.convert();
        self.update_address();
    }

    /// Put text into the input without typing. Input above 1 MiB pauses with
    /// an empty output; otherwise the caller converts.
    fn replace_input(&self, text: String) {
        self.cancel_pending();
        if let Some(editor) = self.editor.get_untracked() {
            editor.set_value(&text);
        }
        self.input.set(text);
        self.convert_large_input.set(false);
        self.copy_notice.set(None);
        if untrack(|| self.input_is_large()) {
            let direction = self.settings.with_untracked(|settings| settings.direction);
            self.output.set(Output::empty(direction));
        }
    }

    /// Load the state an address describes, without rewriting the address.
    fn load(&self, state: Result<Option<PageState>, permalink::InvalidLink>) {
        let (state, notice) = page_state_or_default(state);
        self.settings.set(state.settings);
        self.fix_tab();
        self.replace_input(state.input);
        self.convert();
        self.link_notice.set(notice);
    }

    /// Select **Source** when the selected tab no longer exists.
    fn fix_tab(&self) {
        let direction = self.settings.with_untracked(|settings| settings.direction);
        if direction == Direction::MarkdownToHtml && self.tab.get_untracked() == Tab::Original {
            self.tab.set(Tab::Source);
        }
    }

    fn switch_direction(&self) {
        if self.halted() {
            return;
        }
        self.flush();
        if untrack(|| self.stale()) {
            return;
        }
        let output = self.output.get_untracked().source.to_string();
        self.settings
            .update(|settings| settings.direction = settings.direction.flipped());
        self.fix_tab();
        self.replace_input(output);
        self.convert();
        self.update_address();
    }

    fn may_replace_input(&self) -> bool {
        let keep = self
            .input
            .with_untracked(|input| input.is_empty() || is_preset(input));
        keep || browser::confirm("Replace your input? It is not one of the examples.")
    }

    fn pick_example(&self, example: &'static Example) {
        if !self.may_replace_input() {
            return;
        }
        self.replace_input(example.text.to_string());
        if let Some(extension) = example.extension {
            self.settings
                .update(|settings| settings.cmark.set(extension, true));
        }
        self.convert();
        self.update_address();
    }

    fn clear(&self) {
        if !self.may_replace_input() {
            return;
        }
        self.replace_input(String::new());
        self.convert();
        self.update_address();
    }

    fn convert_anyway(&self) {
        self.convert_large_input.set(true);
        self.cancel_pending();
        self.convert();
    }

    fn copy_output(&self, source: NodeRef<html::Pre>) {
        self.flush();
        if untrack(|| self.stale()) {
            return;
        }
        let text = self.output.get_untracked().source.to_string();
        let copy_notice = self.copy_notice;
        copy_notice.set(None);
        spawn_local(async move {
            if browser::copy_text(&text).await {
                copy_notice.set(Some("Copied the output."));
            } else {
                if let Some(source) = source.get_untracked() {
                    browser::select_contents(&source);
                }
                copy_notice.set(Some(
                    "Copying is blocked; the output is selected, so press Ctrl+C (⌘C on a Mac).",
                ));
            }
        });
    }

    fn share(&self) {
        self.flush();
        self.share_fallback.set(None);
        let base_len = browser::address_base().len();
        let input_dropped = self.settings.with_untracked(|settings| {
            self.input
                .with_untracked(|input| permalink::encode(settings, input, base_len).input_dropped)
        });
        if input_dropped {
            self.link_notice.set(Some(TOO_LARGE_TO_SHARE));
            return;
        }
        let address = browser::address();
        let (copy_notice, share_fallback) = (self.copy_notice, self.share_fallback);
        spawn_local(async move {
            if browser::copy_text(&address).await {
                copy_notice.set(Some("Copied the link."));
            } else {
                share_fallback.set(Some(address));
                copy_notice.set(Some(
                    "Copying is blocked; the link is selected, so press Ctrl+C (⌘C on a Mac).",
                ));
            }
        });
    }
}

fn page_state_or_default(
    state: Result<Option<PageState>, permalink::InvalidLink>,
) -> (PageState, Option<&'static str>) {
    let default = || PageState {
        settings: Settings::default(),
        input: DEFAULT_EXAMPLE.to_string(),
    };
    match state {
        Ok(Some(state)) => (state, None),
        Ok(None) => (default(), None),
        Err(_) => (default(), Some(INVALID_LINK)),
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (initial, initial_notice) =
        page_state_or_default(permalink::decode(&browser::address_fragment()));
    let initial_input = initial.input.clone();
    let playground = Playground {
        settings: RwSignal::new(initial.settings),
        input: RwSignal::new(initial.input),
        output: RwSignal::new(Output::default()),
        convert_large_input: RwSignal::new(false),
        crashed: RwSignal::new(false),
        tab: RwSignal::new(Tab::Source),
        link_notice: RwSignal::new(initial_notice),
        copy_notice: RwSignal::new(None),
        share_fallback: RwSignal::new(None),
        pending: StoredValue::new(None),
        editor: NodeRef::new(),
    };
    // Convert after mounting, so that if a saved input crashes the converter,
    // the editor holding it is already on the page.
    request_animation_frame(move || playground.convert());

    // Hand edits and Back/Forward fire `hashchange`; `replaceState` does not.
    let _ = window_event_listener(ev::hashchange, move |_| {
        if !playground.halted() {
            playground.load(permalink::decode(&browser::address_fragment()));
        }
    });

    let direction = move || playground.settings.with(|settings| settings.direction);

    view! {
        <header class="site-header">
            <h1>"htmd playground"</h1>
            <p class="versions">
                "Built from htmd " {env!("HTMD_VERSION")} " and pulldown-cmark "
                {env!("PULLDOWN_CMARK_VERSION")} "."
            </p>
            <nav aria-label="Project links">
                <a href="https://github.com/dkaimekinmasters/paradigms-htmd">"Source"</a>
                <a href=concat!("https://docs.rs/htmd/", env!("HTMD_VERSION"))>"docs.rs"</a>
                <a href="https://crates.io/crates/htmd">"crates.io"</a>
            </nav>
        </header>
        <div class="toolbar">
            <DirectionSwitch playground />
            <div class="share">
                <button type="button" on:click=move |_| playground.share()>"Share"</button>
                <ShareFallback playground />
            </div>
        </div>
        <OptionsPanel playground />
        <main class="panes">
            <section class="pane" aria-labelledby="input-label">
                <div class="pane-bar">
                    <label id="input-label" for="input">
                        {move || direction().input_language()}
                    </label>
                    <ExamplesMenu playground />
                    <button type="button" on:click=move |_| playground.clear()>"Clear"</button>
                </div>
                <textarea
                    id="input"
                    class="editor"
                    spellcheck="false"
                    autocomplete="off"
                    node_ref=playground.editor
                    prop:value=initial_input
                    on:input=move |event| playground.on_typing(event_target_value(&event))
                ></textarea>
            </section>
            <section class="pane" aria-label="Output">
                <OutputPane playground />
                <StatusLine playground />
            </section>
        </main>
    }
}

#[component]
fn DirectionSwitch(playground: Playground) -> impl IntoView {
    let position = move |value: Direction, label: &'static str| {
        view! {
            <label class="segment">
                <input
                    type="radio"
                    name="direction"
                    data-stale-disable=""
                    prop:checked=move || {
                        playground.settings.with(|settings| settings.direction == value)
                    }
                    prop:disabled=move || playground.stale()
                    on:change=move |_| playground.switch_direction()
                />
                <span>{label}</span>
            </label>
        }
    };
    view! {
        <fieldset class="direction">
            <legend class="visually-hidden">"Direction"</legend>
            {position(Direction::HtmlToMarkdown, "HTML → Markdown")}
            {position(Direction::MarkdownToHtml, "Markdown → HTML")}
        </fieldset>
    }
}

#[component]
fn ShareFallback(playground: Playground) -> impl IntoView {
    let field: NodeRef<html::Input> = NodeRef::new();
    Effect::new(move |_| {
        if playground.share_fallback.with(Option::is_some) {
            request_animation_frame(move || {
                if let Some(field) = field.get_untracked() {
                    let _ = field.focus();
                    field.select();
                }
            });
        }
    });
    view! {
        <Show when=move || playground.share_fallback.with(Option::is_some)>
            <label class="visually-hidden" for="share-address">"Link to copy"</label>
            <input
                id="share-address"
                class="share-address"
                readonly
                node_ref=field
                prop:value=move || playground.share_fallback.get().unwrap_or_default()
            />
        </Show>
    }
}

#[component]
fn ExamplesMenu(playground: Playground) -> impl IntoView {
    let menu: NodeRef<html::Details> = NodeRef::new();
    let entries = move || {
        let direction = playground.settings.with(|settings| settings.direction);
        examples(direction)
            .iter()
            .map(|example| {
                view! {
                    <li>
                        <button
                            type="button"
                            on:click=move |_| {
                                if let Some(menu) = menu.get_untracked() {
                                    menu.set_open(false);
                                }
                                playground.pick_example(example);
                            }
                        >
                            {example.label()}
                        </button>
                    </li>
                }
            })
            .collect_view()
    };
    view! {
        <details class="menu" node_ref=menu>
            <summary>"Examples"</summary>
            <ul>{entries}</ul>
        </details>
    }
}

#[component]
fn OutputPane(playground: Playground) -> impl IntoView {
    let tab = playground.tab;
    let output = playground.output;
    let html_to_markdown = move || {
        playground.settings.with(|settings| settings.direction) == Direction::HtmlToMarkdown
    };
    // Memos are lazy, so a preview is only built while its tab is shown.
    let rendered = Memo::new(move |_| {
        output.with(|output| match output.direction {
            Direction::HtmlToMarkdown => {
                preview_document(&markdown_to_html(&output.source, &output.cmark))
            }
            Direction::MarkdownToHtml => preview_document(&output.source),
        })
    });
    let original = Memo::new(move |_| output.with(|output| preview_document(&output.input)));
    let source: NodeRef<html::Pre> = NodeRef::new();

    let tab_button = move |value: Tab, label: &'static str, id: &'static str| {
        view! {
            <button
                type="button"
                role="tab"
                id=id
                aria-controls="output-panel"
                aria-selected=move || if tab.get() == value { "true" } else { "false" }
                class:selected=move || tab.get() == value
                on:click=move |_| tab.set(value)
            >
                {label}
            </button>
        }
    };
    let output_language = move || output.with(|output| output.direction.output_language());

    view! {
        <div id="output-body">
            <div class="pane-bar" role="tablist" aria-label="Output view">
                {tab_button(Tab::Source, "Source", "tab-source")}
                {tab_button(Tab::Rendered, "Rendered", "tab-rendered")}
                <Show when=html_to_markdown>
                    {tab_button(Tab::Original, "Original", "tab-original")}
                </Show>
            </div>
            <div id="output-panel" class="output-panel" role="tabpanel">
                {move || match tab.get() {
                    Tab::Source => {
                        view! {
                            <div class="source-panel">
                                <button
                                    type="button"
                                    class="copy"
                                    data-stale-disable=""
                                    prop:disabled=move || playground.stale()
                                    on:click=move |_| playground.copy_output(source)
                                >
                                    "Copy"
                                </button>
                                <pre
                                    class="source"
                                    tabindex="0"
                                    aria-label=move || format!("{} output", output_language())
                                    node_ref=source
                                >
                                    {move || output.with(|output| output.source.to_string())}
                                </pre>
                            </div>
                        }
                            .into_any()
                    }
                    Tab::Rendered => {
                        view! {
                            <iframe
                                class="preview"
                                sandbox=""
                                title=move || format!("Rendered {}", output_language())
                                srcdoc=move || rendered.get()
                            ></iframe>
                        }
                            .into_any()
                    }
                    Tab::Original => {
                        view! {
                            <iframe
                                class="preview"
                                sandbox=""
                                title="Original HTML"
                                srcdoc=move || original.get()
                            ></iframe>
                        }
                            .into_any()
                    }
                }}
            </div>
        </div>
        // Filled and shown by `htmdShowCrash` in index.html.
        <div id="crash-message" class="crash" role="alert" hidden></div>
    }
}

#[component]
fn StatusLine(playground: Playground) -> impl IntoView {
    let stats = move || {
        playground.output.with(|output| {
            output.stats.map(|(millis, characters)| {
                let unit = if characters == 1 {
                    "character"
                } else {
                    "characters"
                };
                format!("{millis:.1} ms · {characters} {unit}")
            })
        })
    };
    view! {
        <div class="status" role="status">
            <span>{stats}</span>
            <Show when=move || playground.paused()>
                <span class="notice">
                    "The input is over 1 MiB, so conversion is paused. "
                    <button type="button" on:click=move |_| playground.convert_anyway()>
                        "Convert anyway"
                    </button>
                </span>
            </Show>
            {move || playground.link_notice.get().map(|notice| view! { <span class="notice">{notice}</span> })}
            {move || playground.copy_notice.get().map(|notice| view! { <span class="notice">{notice}</span> })}
        </div>
    }
}

#[component]
fn OptionsPanel(playground: Playground) -> impl IntoView {
    let settings = playground.settings;
    let html_to_markdown =
        move || settings.with(|settings| settings.direction == Direction::HtmlToMarkdown);
    let options = move |read: fn(&Settings) -> bool| move || settings.with(read);
    let ignored_tags = move || {
        let (_, ignored) = settings.with(|settings| parse_skip_tags(&settings.htmd.skip_tags));
        (!ignored.is_empty()).then(|| format!("Ignored: {}", ignored.join(", ")))
    };

    view! {
        <details class="options" open=!browser::is_narrow_screen()>
            <summary>"Options"</summary>
            <div class="options-body">
                <fieldset class="option-group" hidden=move || !html_to_markdown()>
                    <legend>"htmd"</legend>
                    {choice_field(
                        playground,
                        "opt-mode",
                        "Translation mode",
                        "Pure writes plain Markdown and drops attributes; Faithful keeps such HTML as raw HTML.",
                        &TRANSLATION_MODES,
                        |s| s.htmd.options.translation_mode,
                        |s, v| s.htmd.options.translation_mode = v,
                        || true,
                        || false,
                    )}
                    {choice_field(
                        playground,
                        "opt-heading",
                        "Heading style",
                        "Setext is a preference: some headings can only be written in ATX.",
                        &HEADING_STYLES,
                        |s| s.htmd.options.heading_style,
                        |s, v| s.htmd.options.heading_style = v,
                        || true,
                        || false,
                    )}
                    {choice_field(
                        playground,
                        "opt-hr",
                        "Horizontal rule",
                        "How <hr> is written.",
                        &HR_STYLES,
                        |s| s.htmd.options.hr_style,
                        |s, v| s.htmd.options.hr_style = v,
                        || true,
                        || false,
                    )}
                    {choice_field(
                        playground,
                        "opt-br",
                        "Line break",
                        "How <br> is written; Faithful mode always writes <br>.",
                        &BR_STYLES,
                        |s| s.htmd.options.br_style,
                        |s, v| s.htmd.options.br_style = v,
                        || true,
                        options(|s| s.htmd.options.translation_mode == TranslationMode::Faithful),
                    )}
                    {choice_field(
                        playground,
                        "opt-bullet",
                        "Bullet marker",
                        "The character that starts each bullet list item.",
                        &BULLET_MARKERS,
                        |s| s.htmd.options.bullet_list_marker,
                        |s, v| s.htmd.options.bullet_list_marker = v,
                        || true,
                        || false,
                    )}
                    {number_field(
                        playground,
                        "opt-bullet-spaces",
                        "Spaces after bullet",
                        "Spaces between the bullet and the item's content, 1 to 4.",
                        |s| s.htmd.options.ul_bullet_spacing,
                        |s, v| s.htmd.options.ul_bullet_spacing = v,
                    )}
                    {number_field(
                        playground,
                        "opt-number-spaces",
                        "Spaces after list number",
                        "Spaces after the period of the widest-numbered item, 1 to 4.",
                        |s| s.htmd.options.ol_number_spacing,
                        |s, v| s.htmd.options.ol_number_spacing = v,
                    )}
                    {choice_field(
                        playground,
                        "opt-code",
                        "Code block style",
                        "Whether code blocks are fenced or indented by four spaces.",
                        &CODE_BLOCK_STYLES,
                        |s| s.htmd.options.code_block_style,
                        |s, v| s.htmd.options.code_block_style = v,
                        || true,
                        || false,
                    )}
                    {choice_field(
                        playground,
                        "opt-fence",
                        "Code fence",
                        "The fence around fenced code blocks.",
                        &CODE_FENCES,
                        |s| s.htmd.options.code_block_fence,
                        |s, v| s.htmd.options.code_block_fence = v,
                        options(|s| s.htmd.options.code_block_style == CodeBlockStyle::Fenced),
                        || false,
                    )}
                    {choice_field(
                        playground,
                        "opt-links",
                        "Link style",
                        "Autolinks write <url> for links whose text is their URL.",
                        &LINK_STYLES,
                        |s| s.htmd.options.link_style,
                        |s, v| s.htmd.options.link_style = v,
                        || true,
                        || false,
                    )}
                    {choice_field(
                        playground,
                        "opt-refs",
                        "Reference style",
                        "How referenced links name their definitions.",
                        &REFERENCE_STYLES,
                        |s| s.htmd.options.link_reference_style,
                        |s, v| s.htmd.options.link_reference_style = v,
                        options(|s| s.htmd.options.link_style == LinkStyle::Referenced),
                        || false,
                    )}
                    {checkbox_field(
                        playground,
                        "opt-precode",
                        "Preserve code whitespace",
                        "Keep the whitespace in inline <code> as written.",
                        |s| s.htmd.options.preformatted_code,
                        |s, v| s.htmd.options.preformatted_code = v,
                    )}
                    {checkbox_field(
                        playground,
                        "opt-noscript",
                        "Treat <noscript> content as raw text",
                        "Parse <noscript> as a browser with scripting enabled does.",
                        |s| s.htmd.noscript_as_raw_text,
                        |s, v| s.htmd.noscript_as_raw_text = v,
                    )}
                    <div class="field">
                        <label for="opt-skip">"Skip tags"</label>
                        <input
                            type="text"
                            id="opt-skip"
                            spellcheck="false"
                            aria-describedby="opt-skip-help opt-skip-ignored"
                            prop:value=move || settings.with(|s| s.htmd.skip_tags.clone())
                            on:input=move |event| {
                                let value = event_target_value(&event);
                                playground.change(move |s| s.htmd.skip_tags = value);
                            }
                        />
                        <small id="opt-skip-help">
                            "Elements removed with their content; names separated by commas or spaces."
                        </small>
                        <small id="opt-skip-ignored" class="ignored">{ignored_tags}</small>
                    </div>
                </fieldset>
                <fieldset class="option-group">
                    <legend>"pulldown-cmark"</legend>
                    {Extension::ALL
                        .map(|extension| {
                            checkbox_field(
                                playground,
                                extension_id(extension),
                                extension.label(),
                                extension.help(),
                                move |s| s.cmark.is_enabled(extension),
                                move |s, v| s.cmark.set(extension, v),
                            )
                        })
                        .collect_view()}
                </fieldset>
                <button
                    type="button"
                    class="reset"
                    on:click=move |_| {
                        playground
                            .change(|s| {
                                *s = Settings {
                                    direction: s.direction,
                                    ..Settings::default()
                                };
                            })
                    }
                >
                    "Reset to defaults"
                </button>
            </div>
        </details>
    }
}

fn extension_id(extension: Extension) -> &'static str {
    match extension {
        Extension::Tables => "ext-tables",
        Extension::Math => "ext-math",
        Extension::Footnotes => "ext-footnotes",
        Extension::OldFootnotes => "ext-old-footnotes",
        Extension::Strikethrough => "ext-strikethrough",
        Extension::TaskLists => "ext-task-lists",
        Extension::SmartPunctuation => "ext-smart-punctuation",
        Extension::HeadingAttributes => "ext-heading-attributes",
        Extension::YamlMetadata => "ext-yaml",
        Extension::PlusesMetadata => "ext-pluses",
        Extension::Gfm => "ext-gfm",
        Extension::DefinitionLists => "ext-definition-lists",
        Extension::Superscript => "ext-superscript",
        Extension::Subscript => "ext-subscript",
        Extension::Wikilinks => "ext-wikilinks",
    }
}

/// A labelled dropdown. Hidden or disabled values are kept (spec: Options).
#[allow(clippy::too_many_arguments)]
fn choice_field<T: Copy + PartialEq + Send + Sync + 'static>(
    playground: Playground,
    id: &'static str,
    label: &'static str,
    help: &'static str,
    choices: &'static [Choice<T>],
    read: fn(&Settings) -> T,
    write: fn(&mut Settings, T),
    shown: impl Fn() -> bool + Send + Sync + 'static,
    disabled: impl Fn() -> bool + Send + Sync + 'static,
) -> impl IntoView {
    let settings = playground.settings;
    let help_id = format!("{id}-help");
    view! {
        <div class="field" hidden=move || !shown()>
            <label for=id>{label}</label>
            <select
                id=id
                aria-describedby=help_id.clone()
                prop:disabled=disabled
                on:change=move |event| {
                    if let Some(value) = Choice::value_of(choices, &event_target_value(&event)) {
                        playground.change(move |s| write(s, value));
                    }
                }
            >
                {choices
                    .iter()
                    .map(|choice| {
                        view! {
                            <option
                                value=choice.key
                                prop:selected=move || settings.with(|s| read(s) == choice.value)
                            >
                                {choice.label}
                            </option>
                        }
                    })
                    .collect_view()}
            </select>
            <small id=help_id>{help}</small>
        </div>
    }
}

/// A 1–4 number field: out-of-range values clamp, and an emptied field
/// returns to its last value on blur (spec: Options).
fn number_field(
    playground: Playground,
    id: &'static str,
    label: &'static str,
    help: &'static str,
    read: fn(&Settings) -> u8,
    write: fn(&mut Settings, u8),
) -> impl IntoView {
    let settings = playground.settings;
    let help_id = format!("{id}-help");
    view! {
        <div class="field">
            <label for=id>{label}</label>
            <input
                type="number"
                id=id
                min="1"
                max="4"
                step="1"
                inputmode="numeric"
                aria-describedby=help_id.clone()
                prop:value=move || settings.with(read).to_string()
                on:input=move |event| {
                    let typed = event_target_value(&event);
                    let Ok(number) = typed.trim().parse::<f64>() else { return };
                    if !number.is_finite() {
                        return;
                    }
                    let value = clamp_spacing(number.round() as i64);
                    if value.to_string() != typed.trim() {
                        let field: web_sys::HtmlInputElement = event_target(&event);
                        field.set_value(&value.to_string());
                    }
                    if settings.with_untracked(read) != value {
                        playground.change(move |s| write(s, value));
                    }
                }
                on:blur=move |event| {
                    let field: web_sys::HtmlInputElement = event_target(&event);
                    field.set_value(&settings.with_untracked(read).to_string());
                }
            />
            <small id=help_id>{help}</small>
        </div>
    }
}

fn checkbox_field(
    playground: Playground,
    id: &'static str,
    label: &'static str,
    help: &'static str,
    read: impl Fn(&Settings) -> bool + Copy + Send + Sync + 'static,
    write: impl Fn(&mut Settings, bool) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let settings = playground.settings;
    let help_id = format!("{id}-help");
    view! {
        <div class="field checkbox">
            <input
                type="checkbox"
                id=id
                aria-describedby=help_id.clone()
                prop:checked=move || settings.with(read)
                on:change=move |event| {
                    let on = event_target_checked(&event);
                    playground.change(move |s| write(s, on));
                }
            />
            <label for=id>{label}</label>
            <small id=help_id>{help}</small>
        </div>
    }
}
