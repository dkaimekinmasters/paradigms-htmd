//! The playground page (spec: Page layout and everything below it).

use crate::browser;
use crate::convert::{self, Output, parse_skip_tags};
use crate::permalink::{self, Fragment, SavedState};
use crate::presets::{self, DEFAULT_EXAMPLE, Preset};
use crate::settings::*;
use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::time::Duration;

/// How long the user must pause typing before the output updates.
const TYPING_PAUSE: Duration = Duration::from_millis(100);

/// Input above this many UTF-8 bytes pauses conversion (spec: Large input).
const LARGE_INPUT_BYTES: usize = 1024 * 1024;

const REPOSITORY: &str = "https://github.com/dkaimekinmasters/paradigms-htmd";
const HTMD_VERSION: &str = env!("HTMD_VERSION");
const PULLDOWN_CMARK_VERSION: &str = env!("PULLDOWN_CMARK_VERSION");

const SOURCE_ID: &str = "output-source";
const INPUT_ID: &str = "input";
const SHARE_ID: &str = "share-address";

/// An output pane tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Source,
    Rendered,
    Original,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Rendered => "Rendered",
            Self::Original => "Original",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Source => "tab-source",
            Self::Rendered => "tab-rendered",
            Self::Original => "tab-original",
        }
    }

    /// The tabs shown for a direction, in order.
    fn shown(direction: Direction) -> &'static [Tab] {
        match direction {
            Direction::HtmlToMarkdown => &[Self::Source, Self::Rendered, Self::Original],
            Direction::MarkdownToHtml => &[Self::Source, Self::Rendered],
        }
    }
}

/// Whether the output matches the input.
#[derive(Clone, Debug, PartialEq)]
enum Condition {
    Current,
    /// Large input is waiting for "Convert anyway".
    Paused,
    /// A converter crashed; nothing converts until the page reloads.
    Crashed(String),
}

/// The page's state, and the actions the controls perform on it.
#[derive(Clone, Copy)]
struct Playground {
    direction: RwSignal<Direction>,
    input: RwSignal<String>,
    htmd: RwSignal<HtmdSettings>,
    md: RwSignal<MdSettings>,
    /// The last result; empty after a large input arrives by direction switch
    /// or address.
    output: RwSignal<Output>,
    condition: RwSignal<Condition>,
    /// "Convert anyway" was pressed for the current large input.
    convert_anyway: RwSignal<bool>,
    tab: RwSignal<Tab>,
    share_too_large: RwSignal<bool>,
    invalid_link: RwSignal<bool>,
    /// A short message about the last Copy or Share.
    copy_notice: RwSignal<Option<String>>,
    /// The address, shown selected when the browser refuses to copy it.
    manual_share: RwSignal<Option<String>>,
    pending: StoredValue<Option<TimeoutHandle>>,
    /// The user changed something the address does not record yet.
    address_behind: StoredValue<bool>,
}

impl Playground {
    fn new() -> Self {
        Self {
            direction: RwSignal::new(Direction::default()),
            input: RwSignal::new(String::new()),
            htmd: RwSignal::new(HtmdSettings::default()),
            md: RwSignal::new(MdSettings::default()),
            output: RwSignal::new(Output::default()),
            condition: RwSignal::new(Condition::Current),
            convert_anyway: RwSignal::new(false),
            tab: RwSignal::new(Tab::Source),
            share_too_large: RwSignal::new(false),
            invalid_link: RwSignal::new(false),
            copy_notice: RwSignal::new(None),
            manual_share: RwSignal::new(None),
            pending: StoredValue::new(None),
            address_behind: StoredValue::new(false),
        }
    }

    fn is_stale(&self) -> bool {
        self.condition.get() != Condition::Current
    }

    fn cancel_typing_pause(&self) {
        if let Some(handle) = self.pending.get_value() {
            handle.clear();
        }
        self.pending.set_value(None);
    }

    /// Convert keystrokes still inside the typing pause right away.
    fn flush(&self) {
        if self.pending.get_value().is_some() {
            self.cancel_typing_pause();
            self.convert();
        }
    }

    fn edit_input(&self, text: String) {
        self.input.set(text);
        self.address_behind.set_value(true);
        self.copy_notice.set(None);
        self.cancel_typing_pause();
        let playground = *self;
        let handle = set_timeout_with_handle(
            move || {
                playground.pending.set_value(None);
                playground.convert();
            },
            TYPING_PAUSE,
        )
        .ok();
        self.pending.set_value(handle);
    }

    /// Record a change made through a control and convert immediately.
    fn changed_by_user(&self) {
        self.address_behind.set_value(true);
        self.copy_notice.set(None);
        self.cancel_typing_pause();
        self.convert();
    }

    fn change_htmd(&self, change: impl FnOnce(&mut HtmdSettings)) {
        self.htmd.update(change);
        self.changed_by_user();
    }

    fn change_extension(&self, extension: Extension, on: bool) {
        self.md.update(|settings| settings.set(extension, on));
        self.changed_by_user();
    }

    fn reset_options(&self) {
        self.htmd.set(HtmdSettings::default());
        self.md.set(MdSettings::default());
        self.changed_by_user();
    }

    fn convert(&self) {
        if matches!(self.condition.get_untracked(), Condition::Crashed(_)) {
            self.update_address();
            return;
        }
        let input = self.input.get_untracked();
        if input.len() > LARGE_INPUT_BYTES {
            if !self.convert_anyway.get_untracked() {
                self.condition.set(Condition::Paused);
                self.update_address();
                return;
            }
        } else {
            self.convert_anyway.set(false);
        }
        let direction = self.direction.get_untracked();
        let htmd_settings = self.htmd.get_untracked();
        let md_settings = self.md.get_untracked();
        let result = browser::guarded(move || {
            convert::run(
                direction,
                &input,
                &htmd_settings,
                &md_settings,
                browser::now,
            )
        });
        match result {
            Ok(Ok(output)) => {
                self.output.set(output);
                self.condition.set(Condition::Current);
            }
            Ok(Err(error)) => self.condition.set(Condition::Crashed(error.to_string())),
            Err(message) => self.condition.set(Condition::Crashed(message)),
        }
        self.update_address();
    }

    fn saved_state(&self) -> SavedState {
        SavedState {
            dir: self.direction.get_untracked(),
            input: Some(self.input.get_untracked()),
            htmd: self.htmd.get_untracked(),
            md: self.md.get_untracked(),
        }
    }

    /// Write the state into the address if the user changed it.
    fn update_address(&self) {
        if !self.address_behind.get_value() {
            return;
        }
        self.address_behind.set_value(false);
        let base = browser::address_without_fragment();
        let (fragment, too_large) = permalink::fragment_for(&base, &self.saved_state());
        browser::replace_address(&format!("{base}{fragment}"));
        self.share_too_large.set(too_large);
        self.invalid_link.set(false);
    }

    /// Load the state the address describes (spec: Permalinks).
    fn load_address(&self) {
        match permalink::decode(&browser::fragment()) {
            Fragment::State(state) => {
                self.invalid_link.set(false);
                self.apply(state);
            }
            Fragment::Empty => {
                self.invalid_link.set(false);
                self.apply(default_state());
            }
            Fragment::Invalid => {
                self.invalid_link.set(true);
                self.apply(default_state());
            }
        }
    }

    fn apply(&self, state: SavedState) {
        self.cancel_typing_pause();
        self.address_behind.set_value(false);
        self.share_too_large.set(false);
        self.manual_share.set(None);
        self.copy_notice.set(None);
        self.set_direction(state.dir);
        let input = state.input.unwrap_or_default();
        self.start_fresh_output(&input);
        self.input.set(input);
        self.htmd.set(state.htmd);
        self.md.set(state.md);
        self.convert();
    }

    fn set_direction(&self, direction: Direction) {
        self.direction.set(direction);
        if !Tab::shown(direction).contains(&self.tab.get_untracked()) {
            self.tab.set(Tab::Source);
        }
    }

    /// Text that replaced the input wholesale starts without "Convert anyway",
    /// and with an empty output if it is too large to convert.
    fn start_fresh_output(&self, input: &str) {
        self.convert_anyway.set(false);
        if input.len() > LARGE_INPUT_BYTES {
            self.output.set(Output::default());
        }
    }

    fn switch_direction(&self, direction: Direction) {
        if direction == self.direction.get_untracked() {
            return;
        }
        self.flush();
        if self.condition.get_untracked() != Condition::Current {
            return;
        }
        let text = self.output.get_untracked().source;
        self.set_direction(direction);
        self.start_fresh_output(&text);
        self.input.set(text);
        self.changed_by_user();
    }

    /// Whether the input may be replaced, asking first if that would lose
    /// the user's own text.
    fn may_replace_input(&self, action: &str) -> bool {
        let input = self.input.get_untracked();
        input.is_empty()
            || presets::is_preset(&input)
            || browser::confirm(&format!("{action} will replace your input. Continue?"))
    }

    fn choose_preset(&self, preset: &Preset) {
        if !self.may_replace_input("Loading this example") {
            return;
        }
        if let Some(extension) = preset.extension {
            self.md.update(|settings| settings.set(extension, true));
        }
        self.start_fresh_output(preset.text);
        self.input.set(preset.text.to_string());
        self.changed_by_user();
    }

    fn clear(&self) {
        if self.may_replace_input("Clearing") {
            self.input.set(String::new());
            self.changed_by_user();
        }
    }

    fn convert_large_input(&self) {
        self.convert_anyway.set(true);
        self.convert();
    }

    fn copy_output(&self) {
        self.flush();
        if self.condition.get_untracked() != Condition::Current {
            return;
        }
        let text = self.output.get_untracked().source;
        self.copy(text, SOURCE_ID, "Copied the output.");
    }

    fn copy_input(&self) {
        self.copy(self.input.get_untracked(), INPUT_ID, "Copied the input.");
    }

    /// Copy `text`, or select it in the element `fallback_id` and ask the
    /// user to copy it.
    fn copy(&self, text: String, fallback_id: &'static str, done: &'static str) {
        let notice = self.copy_notice;
        spawn_local(async move {
            match browser::copy_to_clipboard(&text).await {
                Ok(()) => notice.set(Some(done.to_string())),
                Err(()) => {
                    browser::focus_and_select(fallback_id);
                    notice.set(Some(manual_copy_message()));
                }
            }
        });
    }

    fn share(&self) {
        self.flush();
        self.update_address();
        if self.share_too_large.get_untracked() {
            self.copy_notice.set(Some(
                "The input is too large to share; the link keeps only the direction and options."
                    .to_string(),
            ));
            return;
        }
        let address = browser::address();
        let notice = self.copy_notice;
        let manual_share = self.manual_share;
        manual_share.set(None);
        spawn_local(async move {
            match browser::copy_to_clipboard(&address).await {
                Ok(()) => notice.set(Some("Copied a link to this page.".to_string())),
                Err(()) => {
                    manual_share.set(Some(address));
                    request_animation_frame(|| browser::focus_and_select(SHARE_ID));
                    notice.set(Some(manual_copy_message()));
                }
            }
        });
    }
}

fn manual_copy_message() -> String {
    "The browser did not allow copying. The text is selected: press Ctrl+C (⌘C on a Mac) to copy it."
        .to_string()
}

fn default_state() -> SavedState {
    SavedState {
        input: Some(DEFAULT_EXAMPLE.to_string()),
        ..Default::default()
    }
}

/// The whole page.
#[component]
pub fn App() -> impl IntoView {
    let playground = Playground::new();
    playground.load_address();
    window_event_listener(ev::hashchange, move |_| playground.load_address());

    view! {
        <header class="page-header">
            <h1>"htmd playground"</h1>
            <p class="versions">
                {format!("htmd {HTMD_VERSION} · pulldown-cmark {PULLDOWN_CMARK_VERSION}")}
            </p>
            <nav aria-label="Project links">
                <a href=REPOSITORY>"Source repository"</a>
                <a href="https://docs.rs/htmd">"htmd on docs.rs"</a>
                <a href="https://crates.io/crates/htmd">"htmd on crates.io"</a>
            </nav>
        </header>
        <main>
            <DirectionSwitch playground />
            <OptionsPanel playground />
            <div class="panes">
                <InputPane playground />
                <OutputPane playground />
            </div>
            <StatusLine playground />
        </main>
    }
}

#[component]
fn DirectionSwitch(playground: Playground) -> impl IntoView {
    let button = move |direction: Direction| {
        view! {
            <button
                type="button"
                aria-pressed=move || (playground.direction.get() == direction).to_string()
                disabled=move || playground.is_stale()
                on:click=move |_| playground.switch_direction(direction)
            >
                {direction.label()}
            </button>
        }
    };
    view! {
        <div class="direction" role="group" aria-label="Direction">
            {button(Direction::HtmlToMarkdown)}
            {button(Direction::MarkdownToHtml)}
        </div>
    }
}

#[component]
fn InputPane(playground: Playground) -> impl IntoView {
    let language = move || playground.direction.get().input_language();
    view! {
        <section class="pane" aria-label="Input">
            <div class="pane-bar">
                <label for=INPUT_ID class="pane-title">{language}</label>
                <div class="pane-actions">
                    <select
                        aria-label="Examples"
                        on:change=move |ev| {
                            let select = event_target::<web_sys::HtmlSelectElement>(&ev);
                            let chosen = select.value().parse::<usize>().ok();
                            select.set_value("");
                            let direction = playground.direction.get_untracked();
                            if let Some(preset) = chosen.and_then(|i| presets::presets(direction).get(i)) {
                                playground.choose_preset(preset);
                            }
                        }
                    >
                        <option value="" selected>"Examples…"</option>
                        {move || {
                            presets::presets(playground.direction.get())
                                .iter()
                                .enumerate()
                                .map(|(index, preset)| {
                                    view! { <option value=index.to_string()>{preset.label()}</option> }
                                })
                                .collect_view()
                        }}
                    </select>
                    <button type="button" on:click=move |_| playground.clear()>"Clear"</button>
                </div>
            </div>
            <textarea
                id=INPUT_ID
                class="editor"
                spellcheck="false"
                autocapitalize="off"
                autocomplete="off"
                prop:value=move || playground.input.get()
                on:input=move |ev| playground.edit_input(event_target_value(&ev))
            ></textarea>
        </section>
    }
}

#[component]
fn OutputPane(playground: Playground) -> impl IntoView {
    let crashed = move || matches!(playground.condition.get(), Condition::Crashed(_));
    view! {
        <section class="pane" aria-label="Output">
            <Show when=crashed fallback=move || view! { <OutputTabs playground /> }>
                <CrashMessage playground />
            </Show>
        </section>
    }
}

#[component]
fn OutputTabs(playground: Playground) -> impl IntoView {
    let select_neighbour = move |ev: ev::KeyboardEvent| {
        let shown = Tab::shown(playground.direction.get_untracked());
        let current = shown
            .iter()
            .position(|tab| *tab == playground.tab.get_untracked())
            .unwrap_or(0);
        let next = match ev.key().as_str() {
            "ArrowRight" => (current + 1) % shown.len(),
            "ArrowLeft" => (current + shown.len() - 1) % shown.len(),
            "Home" => 0,
            "End" => shown.len() - 1,
            _ => return,
        };
        ev.prevent_default();
        playground.tab.set(shown[next]);
        browser::focus(shown[next].id());
    };
    let tab_button = move |tab: Tab| {
        let selected = move || playground.tab.get() == tab;
        view! {
            <button
                type="button"
                role="tab"
                id=tab.id()
                aria-controls="output-panel"
                aria-selected=move || selected().to_string()
                tabindex=move || if selected() { "0" } else { "-1" }
                on:click=move |_| playground.tab.set(tab)
            >
                {tab.label()}
            </button>
        }
    };
    let shows = move |tab: Tab| move || playground.tab.get() == tab;
    view! {
        <div class="pane-bar">
            <div role="tablist" aria-label="Output view" class="tabs" on:keydown=select_neighbour>
                {move || {
                    Tab::shown(playground.direction.get())
                        .iter()
                        .map(|tab| tab_button(*tab))
                        .collect_view()
                }}
            </div>
            <Show when=shows(Tab::Source)>
                <button
                    type="button"
                    disabled=move || playground.is_stale()
                    on:click=move |_| playground.copy_output()
                >
                    "Copy"
                </button>
            </Show>
        </div>
        <div
            id="output-panel"
            role="tabpanel"
            class="output-panel"
            aria-labelledby=move || playground.tab.get().id()
        >
            <textarea
                id=SOURCE_ID
                class="editor"
                readonly
                spellcheck="false"
                aria-label="Output source"
                hidden=move || !shows(Tab::Source)()
                prop:value=move || playground.output.with(|output| output.source.clone())
            ></textarea>
            <iframe
                class="rendered"
                sandbox=""
                title="Rendered output"
                hidden=move || !shows(Tab::Rendered)()
                srcdoc=move || playground.output.with(|output| output.rendered.clone())
            ></iframe>
            <Show when=move || playground.direction.get() == Direction::HtmlToMarkdown>
                <iframe
                    class="rendered"
                    sandbox=""
                    title="Original HTML"
                    hidden=move || !shows(Tab::Original)()
                    srcdoc=move || {
                        playground.output.with(|output| output.original.clone().unwrap_or_default())
                    }
                ></iframe>
            </Show>
        </div>
    }
}

#[component]
fn CrashMessage(playground: Playground) -> impl IntoView {
    let message = move || match playground.condition.get() {
        Condition::Crashed(message) => message,
        _ => String::new(),
    };
    view! {
        <div class="crash" role="alert">
            <h2>"The converter crashed"</h2>
            <p>{message}</p>
            <p>
                "Your input is still editable, but nothing converts again until the page is reloaded. "
                "If the saved input itself causes the crash, start over."
            </p>
            <div class="crash-actions">
                <button type="button" on:click=move |_| playground.copy_input()>
                    "Copy input"
                </button>
                <a
                    href=browser::address()
                    on:click=|ev| {
                        ev.prevent_default();
                        browser::reload();
                    }
                >
                    "Reload"
                </a>
                <a href=browser::address_without_state()>"Start over"</a>
            </div>
        </div>
    }
}

#[component]
fn StatusLine(playground: Playground) -> impl IntoView {
    let summary = move || match playground.condition.get() {
        Condition::Current => playground.output.with(|output| {
            format!(
                "Converted in {:.1} ms · {} characters",
                output.millis, output.chars
            )
        }),
        Condition::Paused => "The input is larger than 1 MiB, so conversion is paused.".to_string(),
        Condition::Crashed(_) => "Conversion stopped after a crash.".to_string(),
    };
    view! {
        <div class="status-bar">
            <div class="status" role="status" aria-live="polite">
                <span>{summary}</span>
                <Show when=move || playground.invalid_link.get()>
                    <span class="notice">
                        "The link was invalid, so the default example is shown."
                    </span>
                </Show>
                <Show when=move || playground.share_too_large.get()>
                    <span class="notice">
                        "The input is too large to share; the address keeps only the direction and options."
                    </span>
                </Show>
                {move || playground.copy_notice.get().map(|notice| view! { <span class="notice">{notice}</span> })}
            </div>
            <Show when=move || playground.condition.get() == Condition::Paused>
                <button type="button" on:click=move |_| playground.convert_large_input()>
                    "Convert anyway"
                </button>
            </Show>
            <button type="button" on:click=move |_| playground.share()>"Share"</button>
            {move || {
                playground
                    .manual_share
                    .get()
                    .map(|address| {
                        view! {
                            <input
                                id=SHARE_ID
                                class="share-address"
                                readonly
                                aria-label="Link to this page"
                                prop:value=address
                            />
                        }
                    })
            }}
        </div>
    }
}

#[component]
fn OptionsPanel(playground: Playground) -> impl IntoView {
    let html_to_markdown = move || playground.direction.get() == Direction::HtmlToMarkdown;
    view! {
        <details class="options" open=browser::is_wide_screen()>
            <summary>"Options"</summary>
            <div class="options-body">
                <Show when=html_to_markdown>
                    <HtmdOptions playground />
                </Show>
                <fieldset>
                    <legend>"pulldown-cmark extensions"</legend>
                    <div class="controls">
                        {Extension::ALL
                            .into_iter()
                            .map(|extension| extension_checkbox(playground, extension))
                            .collect_view()}
                    </div>
                </fieldset>
                <button type="button" on:click=move |_| playground.reset_options()>
                    "Reset to defaults"
                </button>
            </div>
        </details>
    }
}

#[component]
fn HtmdOptions(playground: Playground) -> impl IntoView {
    let htmd = playground.htmd;
    let faithful = move || htmd.with(|s| s.translation_mode == TranslationMode::Faithful);
    let fenced = move || htmd.with(|s| s.code_block_style == CodeBlockStyle::Fenced);
    let referenced = move || htmd.with(|s| s.link_style == LinkStyle::Referenced);
    let ignored_tags = move || {
        let (_, ignored) = htmd.with(|s| parse_skip_tags(&s.skip_tags));
        (!ignored.is_empty()).then(|| format!("Ignored: {}", ignored.join(", ")))
    };
    view! {
        <fieldset>
            <legend>"htmd"</legend>
            <div class="controls">
                {select_control(
                    playground,
                    "translation-mode",
                    "Translation mode",
                    || htmd_help::TRANSLATION_MODE,
                    |s| s.translation_mode,
                    |s, v| s.translation_mode = v,
                    || false,
                )}
                {select_control(
                    playground,
                    "heading-style",
                    "Heading style",
                    || htmd_help::HEADING_STYLE,
                    |s| s.heading_style,
                    |s, v| s.heading_style = v,
                    || false,
                )}
                {select_control(
                    playground,
                    "horizontal-rule",
                    "Horizontal rule",
                    || htmd_help::HORIZONTAL_RULE,
                    |s| s.horizontal_rule,
                    |s, v| s.horizontal_rule = v,
                    || false,
                )}
                {select_control(
                    playground,
                    "line-break",
                    "Line break",
                    move || if faithful() { htmd_help::LINE_BREAK_FAITHFUL } else { htmd_help::LINE_BREAK },
                    |s| s.line_break,
                    |s, v| s.line_break = v,
                    faithful,
                )}
                {select_control(
                    playground,
                    "bullet-marker",
                    "Bullet marker",
                    || htmd_help::BULLET_MARKER,
                    |s| s.bullet_marker,
                    |s, v| s.bullet_marker = v,
                    || false,
                )}
                {spacing_control(
                    playground,
                    "bullet-spacing",
                    "Spaces after bullet",
                    htmd_help::BULLET_SPACING,
                    |s| s.bullet_spacing,
                    |s, v| s.bullet_spacing = v,
                )}
                {spacing_control(
                    playground,
                    "number-spacing",
                    "Spaces after list number",
                    htmd_help::NUMBER_SPACING,
                    |s| s.number_spacing,
                    |s, v| s.number_spacing = v,
                )}
                {select_control(
                    playground,
                    "code-block-style",
                    "Code block style",
                    || htmd_help::CODE_BLOCK_STYLE,
                    |s| s.code_block_style,
                    |s, v| s.code_block_style = v,
                    || false,
                )}
                <Show when=fenced>
                    {select_control(
                        playground,
                        "code-fence",
                        "Code fence",
                        || htmd_help::CODE_FENCE,
                        |s| s.code_fence,
                        |s, v| s.code_fence = v,
                        || false,
                    )}
                </Show>
                {select_control(
                    playground,
                    "link-style",
                    "Link style",
                    || htmd_help::LINK_STYLE,
                    |s| s.link_style,
                    |s, v| s.link_style = v,
                    || false,
                )}
                <Show when=referenced>
                    {select_control(
                        playground,
                        "reference-style",
                        "Reference style",
                        || htmd_help::REFERENCE_STYLE,
                        |s| s.reference_style,
                        |s, v| s.reference_style = v,
                        || false,
                    )}
                </Show>
                {htmd_checkbox(
                    playground,
                    "preserve-code-whitespace",
                    "Preserve code whitespace",
                    htmd_help::PRESERVE_CODE_WHITESPACE,
                    |s| s.preserve_code_whitespace,
                    |s, v| s.preserve_code_whitespace = v,
                )}
                {htmd_checkbox(
                    playground,
                    "noscript-raw-text",
                    "Treat <noscript> content as raw text",
                    htmd_help::NOSCRIPT_RAW_TEXT,
                    |s| s.noscript_raw_text,
                    |s, v| s.noscript_raw_text = v,
                )}
                <div class="control control-wide">
                    <label for="skip-tags">"Skip tags"</label>
                    <input
                        id="skip-tags"
                        type="text"
                        spellcheck="false"
                        autocapitalize="off"
                        placeholder="e.g. script, style"
                        aria-describedby="skip-tags-help skip-tags-ignored"
                        prop:value=move || htmd.with(|s| s.skip_tags.clone())
                        on:input=move |ev| {
                            let text = event_target_value(&ev);
                            playground.change_htmd(move |s| s.skip_tags = text);
                        }
                    />
                    <small id="skip-tags-help" class="help">{htmd_help::SKIP_TAGS}</small>
                    <small id="skip-tags-ignored" class="help warning">{ignored_tags}</small>
                </div>
            </div>
        </fieldset>
    }
}

/// A dropdown for one htmd option.
fn select_control<T: Choice>(
    playground: Playground,
    id: &'static str,
    label: &'static str,
    help: impl Fn() -> &'static str + Send + Sync + 'static,
    get: fn(&HtmdSettings) -> T,
    set: fn(&mut HtmdSettings, T),
    disabled: impl Fn() -> bool + Send + Sync + 'static,
) -> impl IntoView {
    let help_id = format!("{id}-help");
    view! {
        <div class="control">
            <label for=id>{label}</label>
            <select
                id=id
                aria-describedby=help_id.clone()
                disabled=disabled
                on:change=move |ev| {
                    let key = event_target_value(&ev);
                    if let Some(&value) = T::ALL.iter().find(|value| value.key() == key) {
                        playground.change_htmd(move |s| set(s, value));
                    }
                }
            >
                {T::ALL
                    .iter()
                    .map(|&value| {
                        view! {
                            <option
                                value=value.key()
                                prop:selected=move || playground.htmd.with(|s| get(s) == value)
                            >
                                {value.label()}
                            </option>
                        }
                    })
                    .collect_view()}
            </select>
            <small id=help_id class="help">{help}</small>
        </div>
    }
}

/// A 1–4 number field: out-of-range numbers clamp, and a field left empty or
/// invalid shows the current value again when it loses focus.
fn spacing_control(
    playground: Playground,
    id: &'static str,
    label: &'static str,
    help: &'static str,
    get: fn(&HtmdSettings) -> u8,
    set: fn(&mut HtmdSettings, u8),
) -> impl IntoView {
    let help_id = format!("{id}-help");
    let current = move || playground.htmd.with_untracked(get);
    view! {
        <div class="control">
            <label for=id>{label}</label>
            <input
                id=id
                type="number"
                inputmode="numeric"
                min=*SPACING_RANGE.start()
                max=*SPACING_RANGE.end()
                step="1"
                aria-describedby=help_id.clone()
                prop:value=move || playground.htmd.with(get).to_string()
                on:input=move |ev| {
                    if let Ok(number) = event_target_value(&ev).trim().parse::<i64>() {
                        let clamped = number.clamp(
                            i64::from(*SPACING_RANGE.start()),
                            i64::from(*SPACING_RANGE.end()),
                        ) as u8;
                        if clamped != current() {
                            playground.change_htmd(move |s| set(s, clamped));
                        }
                    }
                }
                on:change=move |ev| {
                    event_target::<web_sys::HtmlInputElement>(&ev).set_value(&current().to_string());
                }
            />
            <small id=help_id class="help">{help}</small>
        </div>
    }
}

/// A checkbox for one htmd option.
fn htmd_checkbox(
    playground: Playground,
    id: &'static str,
    label: &'static str,
    help: &'static str,
    get: fn(&HtmdSettings) -> bool,
    set: fn(&mut HtmdSettings, bool),
) -> impl IntoView {
    checkbox(
        id.to_string(),
        label,
        help,
        move || playground.htmd.with(get),
        move |on| playground.change_htmd(move |s| set(s, on)),
    )
}

/// A checkbox for one pulldown-cmark extension.
fn extension_checkbox(playground: Playground, extension: Extension) -> impl IntoView {
    checkbox(
        format!("ext-{}", extension.key()),
        extension.label(),
        extension.help(),
        move || playground.md.with(|settings| settings.get(extension)),
        move |on| playground.change_extension(extension, on),
    )
}

fn checkbox(
    id: String,
    label: &'static str,
    help: &'static str,
    checked: impl Fn() -> bool + Send + Sync + 'static,
    toggle: impl Fn(bool) + 'static,
) -> impl IntoView {
    let help_id = format!("{id}-help");
    view! {
        <div class="control control-check">
            <input
                type="checkbox"
                id=id.clone()
                aria-describedby=help_id.clone()
                prop:checked=checked
                on:change=move |ev| toggle(event_target_checked(&ev))
            />
            <label for=id>{label}</label>
            <small id=help_id class="help">{help}</small>
        </div>
    }
}
