//! Autocomplete widget: a text input with a suggestion menu.
//!
//! Unlike [`iced_widget::combo_box`], the text value is the single source of
//! truth: the option list only serves as hints for quick filling, and content
//! typed outside of the option list is submitted as a regular value.
//!
//! The widget is built on top of `iced_widget` primitives — it reuses
//! [`iced_widget::text_input::TextInput`] and [`iced_widget::overlay::menu::Menu`]
//! and does not patch iced itself.

use iced::widget::overlay::menu;
use iced::widget::text_input::{self, TextInput};

use iced_core::clipboard::Clipboard;
use iced_core::event::Event;
use iced_core::keyboard;
use iced_core::keyboard::key;
use iced_core::layout::{self, Layout};
use iced_core::mouse;
use iced_core::overlay::Element as OverlayElement;
use iced_core::renderer;
use iced_core::text;
use iced_core::widget::{self, Widget};
use iced_core::{Element, Length, Padding, Pixels, Rectangle, Shell, Size, Theme, Vector};

use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
use std::time::Instant;

/// Creates an [`AutoComplete`] widget.
///
/// - `state`: the source of truth for the option pool and the current text,
///   held by the caller.
/// - `placeholder`: the placeholder shown when the text is empty.
/// - `on_input`: called on every text change. Handle it in your application's
///   `update`; the widget performs no I/O and no debouncing.
/// - `on_submit`: called when the current text is submitted (either free text
///   or the value filled in after selecting an option).
/// - `on_option_selected`: called when a specific option is selected (optional).
///
/// If the options come from a remote source, start the request in `update` from
/// `on_input` and hand the results back with [`State::set_options`]. See the
/// crate docs for the full shape, and the `remote` example for debouncing and
/// for discarding stale responses.
pub fn auto_complete<'a, T, Message, Theme, Renderer>(
    state: &'a State<T>,
    placeholder: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
    on_submit: impl Fn(String) -> Message + 'a,
    on_option_selected: Option<impl Fn(T) -> Message + 'a>,
) -> AutoComplete<'a, T, Message, Theme, Renderer>
where
    T: Display + Clone + 'static,
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    AutoComplete {
        state,
        text_input: TextInput::new(placeholder, &state.value())
            .on_input(TextInputEvent::TextChanged)
            .class(Theme::default_input())
            .padding(text_input::DEFAULT_PADDING),
        font: None,
        padding: text_input::DEFAULT_PADDING,
        size: None,
        on_input: Box::new(on_input),
        on_submit: Box::new(on_submit),
        on_option_selected: on_option_selected
            .map(|f| Box::new(f) as Box<dyn Fn(T) -> Message + 'a>),
        to_value: Box::new(|option: &T| option.to_string()),
        on_open: None,
        on_close: None,
        text_shaping: text::Shaping::default(),
        menu_class: <Theme as Catalog>::default_menu(),
        menu_height: Length::Shrink,
    }
}

/// A text input with a suggestion menu.
pub struct AutoComplete<'a, T, Message, Theme = iced_core::Theme, Renderer = iced::widget::Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    state: &'a State<T>,
    text_input: TextInput<'a, TextInputEvent, Theme>,
    font: Option<iced::Font>,
    padding: Padding,
    size: Option<f32>,
    on_input: Box<dyn Fn(String) -> Message + 'a>,
    on_submit: Box<dyn Fn(String) -> Message + 'a>,
    on_option_selected: Option<Box<dyn Fn(T) -> Message + 'a>>,
    to_value: Box<dyn Fn(&T) -> String + 'a>,
    on_open: Option<Message>,
    on_close: Option<Message>,
    text_shaping: text::Shaping,
    menu_class: <Theme as menu::Catalog>::Class<'a>,
    menu_height: Length,
}

impl<'a, T, Message, Theme, Renderer> AutoComplete<'a, T, Message, Theme, Renderer>
where
    T: Display + Clone + 'static,
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    /// Sets the text currently displayed/edited by the input (usually pass
    /// `state.value()` so it persists across views).
    ///
    /// Note: `TextInput` has no runtime value-setting interface, so the text is
    /// written back into the [`State`] here; since every `view` rebuilds the
    /// widget from `state.value()`, it takes effect on the next frame.
    /// Callers are expected to pass `state.value()`, which matches the initial
    /// value used at construction and causes no visual jump.
    pub fn value(self, value: impl Into<String>) -> Self {
        self.state.set_value(value);
        self
    }

    /// Sets how an option is turned into the text filled into the input
    /// (defaults to `T::to_string`).
    ///
    /// For example, the menu may display `id (name)` while only `id` should be
    /// filled into the field.
    pub fn to_value(mut self, to_value: impl Fn(&T) -> String + 'a) -> Self {
        self.to_value = Box::new(to_value);
        self
    }

    /// Replaces the filtering rule and immediately re-filters the pool.
    ///
    /// Equivalent to setting it when the [`State`] is built, but usable later —
    /// for instance when the user switches between "search by name" and
    /// "search by id".
    pub fn filter(self, filter: Filter<T>) -> Self {
        self.state.set_filter(filter);
        self
    }

    /// Sets the message produced on every text change.
    pub fn on_input(mut self, on_input: impl Fn(String) -> Message + 'a) -> Self {
        self.on_input = Box::new(on_input);
        self
    }

    /// Sets the message produced when the menu opens.
    pub fn on_open(mut self, message: Message) -> Self {
        self.on_open = Some(message);
        self
    }

    /// Sets the message produced when the menu closes: on blur, `Escape`, or
    /// after selecting an option (by keyboard or click).
    pub fn on_close(mut self, message: Message) -> Self {
        self.on_close = Some(message);
        self
    }

    /// Sets the [`Padding`] of the input.
    #[must_use]
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self.text_input = self.text_input.padding(self.padding);
        self
    }

    /// Sets the font.
    #[must_use]
    pub fn font(mut self, font: iced::Font) -> Self {
        self.font = Some(font);
        self.text_input = self.text_input.font(font);
        self
    }

    /// Sets the [`text_input::Icon`] of the input.
    // #[must_use]
    // pub fn icon(mut self, icon: text_input::Icon<Renderer::Font>) -> Self {
    //     self.text_input = self.text_input.icon(icon);
    //     self
    // }

    /// Sets the text size.
    #[must_use]
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        let size = size.into();
        self.size = Some(size.0);
        self.text_input = self.text_input.size(size);
        self
    }

    /// Sets the line height.
    #[must_use]
    pub fn line_height(mut self, line_height: impl Into<text::LineHeight>) -> Self {
        self.text_input = self.text_input.line_height(line_height);
        self
    }

    /// Sets the width.
    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.text_input = self.text_input.width(width);
        self
    }

    /// Sets the text shaping strategy.
    #[must_use]
    pub fn text_shaping(mut self, shaping: text::Shaping) -> Self {
        self.text_shaping = shaping;
        self
    }

    /// Sets the style of the input.
    #[must_use]
    pub fn input_style(
        mut self,
        style: impl Fn(&Theme, text_input::Status) -> text_input::Style + 'a,
    ) -> Self
    where
        <Theme as text_input::Catalog>::Class<'a>: From<text_input::StyleFn<'a, Theme>>,
    {
        self.text_input = self.text_input.style(style);
        self
    }

    /// Sets the style of the suggestion menu.
    #[must_use]
    pub fn menu_style(mut self, style: impl Fn(&Theme) -> menu::Style + 'a) -> Self
    where
        <Theme as menu::Catalog>::Class<'a>: From<menu::StyleFn<'a, Theme>>,
    {
        self.menu_class = (Box::new(style) as menu::StyleFn<'a, Theme>).into();
        self
    }
}

/// The local state of an [`AutoComplete`] widget.
#[derive(Clone)]
pub struct State<T> {
    options: Vec<T>,
    inner: RefCell<Inner<T>>,
}

/// The candidate filter closure: given an option and the current query text,
/// returns whether the option should be kept.
type MatchFn<T> = dyn Fn(&T, &str) -> bool + 'static;

/// How the option pool is narrowed as the user types.
///
/// The widget ships **no default matching rule**: what counts as a match
/// depends entirely on your data and intent, so any built-in guess would be
/// silently wrong for some callers. State the rule yourself, once, at
/// [`State::new`].
pub enum Filter<T> {
    /// Your own predicate: receives the option and the current text, and
    /// returns whether the option should remain visible.
    ///
    /// ```no_run
    /// use iced_autocomplete::{Filter, State};
    ///
    /// #[derive(Clone)]
    /// struct Person {
    ///     username: String,
    ///     initial: char,
    /// }
    ///
    /// // Match on the username, case-insensitively, plus the initial.
    /// let state: State<Person> = State::new(
    ///     Vec::<Person>::new(),
    ///     Filter::Custom(Box::new(|person: &Person, query: &str| {
    ///         let query = query.to_lowercase();
    ///         person.username.to_lowercase().contains(&query)
    ///             || person.initial.to_lowercase().to_string() == query
    ///     })),
    /// );
    /// ```
    Custom(Box<MatchFn<T>>),

    /// No filtering: the pool is displayed exactly as provided.
    ///
    /// Use this when the options are already the exact result set — most
    /// importantly for remote search, where the server applied the rules.
    None,
}

impl<T> Debug for Filter<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Custom(_) => f.write_str("Custom(..)"),
            Self::None => f.write_str("None"),
        }
    }
}

#[derive(Clone)]
struct Inner<T> {
    value: String,
    filtered_options: Filtered<T>,
    filter: Rc<Filter<T>>,
}

impl<T: Debug> Debug for Inner<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inner")
            .field("value", &self.value)
            .field("filtered_options", &self.filtered_options)
            .field("filter", &self.filter)
            .finish()
    }
}

impl<T: Debug> Debug for State<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("options", &self.options)
            .field("inner", &self.inner)
            .finish()
    }
}

#[derive(Debug, Clone)]
struct Filtered<T> {
    options: Vec<T>,
    updated: Instant,
}

impl<T> State<T>
where
    T: Clone,
{
    /// Creates new state with the given option pool and filtering rule.
    ///
    /// The `filter` is required: the widget does not assume how your options
    /// should match, so the rule must be stated explicitly. See [`Filter`].
    ///
    /// ```no_run
    /// use iced_autocomplete::{Filter, State};
    ///
    /// // A remote source: results are already exact, so nothing is filtered.
    /// let state: State<String> = State::new(Vec::new(), Filter::None);
    ///
    /// // An in-memory pool with an explicit, case-sensitive rule.
    /// let state: State<String> = State::new(
    ///     vec!["Rust".to_string()],
    ///     Filter::Custom(Box::new(|option: &String, query: &str| {
    ///         option.contains(query)
    ///     })),
    /// );
    /// ```
    pub fn new(options: impl IntoIterator<Item = T>, filter: Filter<T>) -> Self {
        Self::with_value(options, String::new(), filter)
    }

    /// Creates new state with the given option pool, initial text, and
    /// filtering rule.
    pub fn with_value(
        options: impl IntoIterator<Item = T>,
        value: impl Into<String>,
        filter: Filter<T>,
    ) -> Self {
        let value = value.into();
        let options: Vec<T> = options.into_iter().collect();
        let filter = Rc::new(filter);
        let filtered_options = Filtered::new(apply_filter(&filter, &options, &value));

        Self {
            options,
            inner: RefCell::new(Inner {
                value,
                filtered_options,
                filter,
            }),
        }
    }

    /// Returns the option pool.
    pub fn options(&self) -> &[T] {
        &self.options
    }

    /// Returns the current text (the source of truth).
    pub fn value(&self) -> String {
        self.inner.borrow().value.clone()
    }

    /// Writes the text back (e.g. after taking it out of a message).
    pub fn set_value(&self, value: impl Into<String>) {
        let value = value.into();
        self.inner.borrow_mut().value = value.clone();
        self.recompute_filtered(&value);
    }

    /// Replaces the option pool **exactly as given**, without re-filtering it.
    ///
    /// Use this when the caller owns the matching logic and the new options
    /// already represent the exact set to display — most notably when feeding
    /// back the results of an asynchronous remote search. Because no local
    /// filtering is applied, the pool is shown verbatim.
    ///
    /// If you are swapping an in-memory pool and want the component to keep
    /// narrowing it against the current text, use
    /// [`set_options_filtered`](Self::set_options_filtered) instead.
    pub fn set_options(&mut self, options: impl IntoIterator<Item = T>) {
        self.options = options.into_iter().collect();
        // Deliberately skips the filter: the pool is stored verbatim, so the
        // caller's result set is shown exactly as given.
        self.inner
            .borrow_mut()
            .filtered_options
            .update(self.options.clone());
    }

    /// Replaces the option pool and re-applies the configured [`Filter`]
    /// against the current text.
    ///
    /// Use this when swapping an in-memory pool that should stay searchable —
    /// for example when a sibling control narrows the available choices.
    pub fn set_options_filtered(&mut self, options: impl IntoIterator<Item = T>) {
        self.options = options.into_iter().collect();
        let value = self.inner.borrow().value.clone();
        self.recompute_filtered(&value);
    }

    /// Replaces the filtering rule and immediately re-filters the pool.
    pub fn set_filter(&self, filter: Filter<T>) {
        self.inner.borrow_mut().filter = Rc::new(filter);
        let value = self.inner.borrow().value.clone();
        self.recompute_filtered(&value);
    }

    fn recompute_filtered(&self, value: &str) {
        let filtered = {
            let inner = self.inner.borrow();
            apply_filter(&inner.filter, &self.options, value)
        };
        self.inner.borrow_mut().filtered_options.update(filtered);
    }

    fn with_inner<O>(&self, f: impl FnOnce(&Inner<T>) -> O) -> O {
        f(&self.inner.borrow())
    }

    fn with_inner_mut(&self, f: impl FnOnce(&mut Inner<T>)) {
        f(&mut self.inner.borrow_mut());
    }

    fn sync_filtered_options(&self, options: &mut Filtered<T>) {
        let inner = self.inner.borrow();
        inner.filtered_options.sync(options);
    }
}

/// Computes the filtered option list for the given query text.
///
/// A free function so it can run while an `inner` borrow is held; calling back
/// into `Self` would panic with `RefCell already mutably borrowed`.
fn apply_filter<T>(filter: &Filter<T>, options: &[T], value: &str) -> Vec<T>
where
    T: Clone,
{
    match filter {
        Filter::Custom(match_fn) => options
            .iter()
            .filter(|option| match_fn(option, value))
            .cloned()
            .collect(),
        Filter::None => options.to_vec(),
    }
}

impl<T> Default for State<T>
where
    T: Clone,
{
    /// Creates an empty state with [`Filter::None`].
    ///
    /// Prefer [`State::new`] and state your rule explicitly; this exists for
    /// `#[derive(Default)]`-style construction. An empty pool has nothing to
    /// filter, so `None` is the least surprising choice.
    fn default() -> Self {
        Self::new(Vec::new(), Filter::None)
    }
}

impl<T> Filtered<T>
where
    T: Clone,
{
    fn new(options: Vec<T>) -> Self {
        Self {
            options,
            updated: Instant::now(),
        }
    }

    fn empty() -> Self {
        Self {
            options: vec![],
            updated: Instant::now(),
        }
    }

    fn update(&mut self, options: Vec<T>) {
        self.options = options;
        self.updated = Instant::now();
    }

    fn sync(&self, other: &mut Filtered<T>) {
        if other.updated != self.updated {
            *other = self.clone();
        }
    }
}

/// Private overlay state of the widget.
struct Menu<T> {
    menu: menu::State,
    hovered_option: Option<usize>,
    new_selection: Option<T>,
    filtered_options: Filtered<T>,
    /// Whether the menu is visible (independent of focus).
    is_open: bool,
    /// Set by the overlay when a click closes the menu.
    ///
    /// The overlay cannot publish `on_close` itself — the click is handled
    /// inside `menu`'s own callback, which returns a single message — so it
    /// records the event here and the base `update` publishes it on the next
    /// pass.
    closed_by_click: bool,
}

#[derive(Debug, Clone)]
enum TextInputEvent {
    TextChanged(String),
}

impl<T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for AutoComplete<'_, T, Message, Theme, Renderer>
where
    T: Display + Clone + 'static,
    Message: Clone,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn size(&self) -> Size<Length> {
        Widget::<TextInputEvent, Theme, Renderer>::size(&self.text_input)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
        // ) -> layout::Node {
    ) {
        self.text_input.layout(
            &mut tree.children[0],
            renderer,
            limits,
            // The autocomplete always displays the text value; unlike
            // combo_box, no hidden selection overlay is needed.
            // None,
        )
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<Menu<T>>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(Menu::<T> {
            menu: menu::State::new(),
            filtered_options: Filtered::empty(),
            hovered_option: Some(0),
            new_selection: None,
            is_open: false,
            closed_by_click: false,
        })
    }

    // fn children(&self) -> Vec<widget::Tree> {
    //     vec![widget::Tree::new(&self.text_input as &dyn Widget<_, _, _>)]
    // }

    fn diff(&mut self, _tree: &mut widget::Tree) {
        // Keep the child tree intact so its state is not dropped.
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        // clipboard: &mut Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let menu = tree.state.downcast_mut::<Menu<T>>();

        let started_focused = {
            let text_input_state = tree.children[0]
                .state
                .downcast_ref::<text_input::State<Renderer::Paragraph>>();
            text_input_state.is_focused()
        };

        // Let the text input handle input events first.
        let mut local_messages = Vec::new();
        let mut local_shell = Shell::new(&mut local_messages);

        self.text_input.update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut local_shell,
            viewport,
        );

        if local_shell.is_event_captured() {
            shell.capture_event();
        }
        shell.request_redraw_at(local_shell.redraw_request());
        shell.request_input_method(local_shell.input_method());

        // React to the text input's local messages (text changed).
        for message in local_messages {
            let TextInputEvent::TextChanged(new_value) = message;

            shell.publish((self.on_input)(new_value.clone()));

            self.state.with_inner_mut(|state| {
                menu.hovered_option = Some(0);
                menu.is_open = true;
                state.value = new_value;
                let filtered = apply_filter(&state.filter, &self.state.options, &state.value);
                state.filtered_options.update(filtered);
            });
            shell.invalidate_layout();
            shell.request_redraw();
        }

        let is_focused = {
            let text_input_state = tree.children[0]
                .state
                .downcast_ref::<text_input::State<Renderer::Paragraph>>();
            text_input_state.is_focused()
        };

        // Set when `Escape` closes the menu. Collected rather than published
        // inline so every close is reported from one place, exactly once.
        let mut escaped = false;

        if is_focused {
            self.state.with_inner(|state| {
                if let Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(named_key),
                    modifiers,
                    ..
                }) = event
                {
                    match named_key {
                        // Enter: accept the hovered option if any, otherwise
                        // submit the free text.
                        key::Named::Enter => {
                            if menu.is_open
                                && let Some(index) = menu.hovered_option
                                && let Some(option) = state.filtered_options.options.get(index)
                            {
                                menu.new_selection = Some(option.clone());
                            } else {
                                shell.publish((self.on_submit)(self.state.value()));
                            }
                            shell.capture_event();
                            shell.request_redraw();
                        }
                        // Tab: accept the hovered option (default); without a
                        // hover, let native focus navigation proceed.
                        key::Named::Tab if !modifiers.shift() => {
                            if menu.is_open
                                && let Some(index) = menu.hovered_option
                                && let Some(option) = state.filtered_options.options.get(index)
                            {
                                menu.new_selection = Some(option.clone());
                                shell.capture_event();
                                shell.request_redraw();
                            }
                        }
                        // Arrow up / Shift+Tab: move up in the options.
                        key::Named::ArrowUp | key::Named::Tab if modifiers.shift() => {
                            if menu.is_open {
                                if let Some(index) = &mut menu.hovered_option {
                                    if *index == 0 {
                                        *index =
                                            state.filtered_options.options.len().saturating_sub(1);
                                    } else {
                                        *index = index.saturating_sub(1);
                                    }
                                } else {
                                    menu.hovered_option = Some(0);
                                }
                                shell.capture_event();
                                shell.request_redraw();
                            }
                        }
                        // Arrow down: move down in the options.
                        key::Named::ArrowDown => {
                            if menu.is_open {
                                if let Some(index) = &mut menu.hovered_option {
                                    if *index
                                        >= state.filtered_options.options.len().saturating_sub(1)
                                    {
                                        *index = 0;
                                    } else {
                                        *index = index.saturating_add(1).min(
                                            state.filtered_options.options.len().saturating_sub(1),
                                        );
                                    }
                                } else {
                                    menu.hovered_option = Some(0);
                                }
                                shell.capture_event();
                                shell.request_redraw();
                            }
                        }
                        // Escape: close the menu, keep the current text.
                        key::Named::Escape => {
                            escaped = menu.is_open;
                            menu.is_open = false;
                            shell.capture_event();
                            shell.request_redraw();
                        }
                        _ => {}
                    }
                }
            });
        }

        // Handle options selected via keyboard (Enter/Tab): options picked by
        // mouse clicks are already emitted directly in the overlay's
        // `on_selected` closure; here we only consume `new_selection` written
        // by the keyboard path.
        let mut selection_closed = false;
        if let Some(selection) = menu.new_selection.take() {
            let value = self.state.value();
            // After selecting an option, keep the input filled and focused.
            if let Some(on_option_selected) = &self.on_option_selected {
                shell.publish(on_option_selected(selection));
            }
            shell.publish((self.on_submit)(value));
            menu.is_open = false;
            menu.hovered_option = Some(0);
            selection_closed = true;
            shell.request_redraw();
        }

        let is_focused = {
            let text_input_state = tree.children[0]
                .state
                .downcast_ref::<text_input::State<Renderer::Paragraph>>();
            text_input_state.is_focused()
        };

        let mut focus_closed = false;
        if started_focused != is_focused {
            // Focus changed; force a widget tree rebuild to trigger a new view.
            shell.invalidate_widgets();

            if is_focused {
                // Opening the menu on focus matches combo_box behavior.
                menu.is_open = true;
                menu.hovered_option = Some(0);
                if let Some(on_open) = &self.on_open {
                    shell.publish(on_open.clone());
                }
            } else {
                // Only report a close if the menu was actually open.
                focus_closed = menu.is_open;
                menu.is_open = false;
            }
        }

        // A click closes the menu inside the overlay, which cannot publish
        // `on_close` itself; it records the event instead.
        let click_closed = std::mem::take(&mut menu.closed_by_click);

        // Emit `on_close` at most once per pass. The reasons can overlap — a
        // click both selects and closes, and losing focus closes the menu too —
        // so they are collapsed rather than published individually. Follows the
        // emission of `on_option_selected` / `on_submit` above.
        if (escaped || selection_closed || focus_closed || click_closed)
            && let Some(on_close) = &self.on_close
        {
            shell.publish(on_close.clone());
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.text_input
            .mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.text_input.draw(
            &tree.children[0],
            renderer,
            theme,
            layout,
            cursor,
            None,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<OverlayElement<'b, Message, Theme, Renderer>> {
        let is_focused = {
            let text_input_state = tree.children[0]
                .state
                .downcast_ref::<text_input::State<Renderer::Paragraph>>();
            text_input_state.is_focused()
        };

        let is_open = tree.state.downcast_ref::<Menu<T>>().is_open;

        if is_focused && is_open {
            let Menu {
                menu,
                filtered_options,
                hovered_option,
                is_open: menu_open,
                closed_by_click,
                ..
            } = tree.state.downcast_mut::<Menu<T>>();

            self.state.sync_filtered_options(filtered_options);

            if filtered_options.options.is_empty() {
                None
            } else {
                let bounds = layout.bounds();

                let to_value = &self.to_value;
                let state = self.state;
                let on_submit = &self.on_submit;
                let on_option_selected_cb = self.on_option_selected.as_deref();

                let mut menu_widget = menu::Menu::new(
                    menu,
                    &filtered_options.options,
                    hovered_option,
                    // A click emits the selection message here rather than
                    // deferring to `new_selection`: the overlay captures the
                    // click, so the base `update` does not run in this event
                    // pass, and a deferred message would lag one frame — making
                    // `on_submit` observe a stale text value.
                    move |selection: T| {
                        let value = to_value(&selection);
                        state.set_value(value.clone());
                        *menu_open = false;
                        *closed_by_click = true;
                        match on_option_selected_cb {
                            Some(f) => f(selection),
                            None => on_submit(value),
                        }
                    },
                    // Deliberately `None`: hover only moves the highlight and
                    // must not emit a selection event.
                    None,
                    &self.menu_class,
                )
                .width(bounds.width)
                .padding(self.padding)
                .text_shaping(self.text_shaping);

                if let Some(font) = self.font {
                    menu_widget = menu_widget.font(font);
                }
                if let Some(size) = self.size {
                    menu_widget = menu_widget.text_size(size);
                }

                Some(menu_widget.overlay(
                    layout.position() + translation,
                    *viewport,
                    bounds.height,
                    self.menu_height,
                ))
            }
        } else {
            None
        }
    }
}

impl<T, Message, Theme, Renderer> Debug for AutoComplete<'_, T, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoComplete")
            .field("padding", &self.padding)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

impl<'a, T, Message, Theme, Renderer> From<AutoComplete<'a, T, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    T: Display + Clone + 'static,
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(auto_complete: AutoComplete<'a, T, Message, Theme, Renderer>) -> Self {
        Self::new(auto_complete)
    }
}

/// The theme catalog of the [`AutoComplete`] widget: reuses the styles of
/// `text_input` and `menu`.
pub trait Catalog: text_input::Catalog + menu::Catalog {
    /// The default style class of the input.
    fn default_input<'a>() -> <Self as text_input::Catalog>::Class<'a> {
        <Self as text_input::Catalog>::default()
    }

    /// The default style class of the menu.
    fn default_menu<'a>() -> <Self as menu::Catalog>::Class<'a> {
        <Self as menu::Catalog>::default()
    }
}

impl Catalog for Theme {}

#[cfg(test)]
mod tests {
    use super::*;

    /// A case-insensitive substring rule, used by the local-pool tests. The
    /// widget ships no such rule itself — each caller states their own.
    fn contains_ignore_case(option: &&str, query: &str) -> bool {
        option.to_lowercase().contains(&query.to_lowercase())
    }

    fn names() -> Vec<&'static str> {
        vec!["rust-lang", "rustdoc", "cargo", "iced-rs", "Iced Widgets"]
    }

    fn local_state(options: Vec<&'static str>) -> State<&'static str> {
        State::new(options, Filter::Custom(Box::new(contains_ignore_case)))
    }

    #[test]
    fn custom_filter_narrows_the_pool() {
        let state = local_state(names());

        state.set_value("rust");
        let filtered = state.with_inner(|inner| inner.filtered_options.options.clone());
        assert_eq!(filtered, vec!["rust-lang", "rustdoc"]);
    }

    #[test]
    fn custom_filter_is_case_insensitive_when_the_rule_says_so() {
        let state = local_state(vec!["HelloWorld"]);

        state.set_value("helloworld");
        let filtered = state.with_inner(|inner| inner.filtered_options.options.clone());
        assert_eq!(filtered, vec!["HelloWorld"]);
    }

    #[test]
    fn empty_query_keeps_everything() {
        let state = local_state(names());

        // `set_value("")` re-filters; a rule that matches empty queries keeps
        // the whole pool.
        state.set_value("");
        let filtered = state.with_inner(|inner| inner.filtered_options.options.len());
        assert_eq!(filtered, names().len());
    }

    #[test]
    fn filter_none_never_narrows() {
        let state: State<&str> = State::new(names(), Filter::None);

        // Even with a non-empty query, nothing is filtered out. This is the
        // behaviour remote sources rely on: the results are already exact.
        state.set_value("nothing-matches-this");
        let filtered = state.with_inner(|inner| inner.filtered_options.options.len());
        assert_eq!(filtered, names().len());
    }

    #[test]
    fn custom_filter_can_key_on_a_field_other_than_display() {
        #[derive(Clone, Debug)]
        struct Person {
            username: &'static str,
            initial: char,
        }

        impl Display for Person {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{} ({})", self.username, self.initial)
            }
        }

        let people = vec![
            Person {
                username: "alice",
                initial: 'A',
            },
            Person {
                username: "bob",
                initial: 'B',
            },
        ];

        // The rule inspects `initial`, which never appears in `Display` terms
        // that would match; this is impossible with a display-text matcher.
        let state: State<Person> = State::new(
            people,
            Filter::Custom(Box::new(|person: &Person, query: &str| {
                query.is_empty() || person.initial.to_lowercase().to_string() == query
            })),
        );

        state.set_value("b");
        let filtered = state.with_inner(|inner| inner.filtered_options.options.clone());
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].username, "bob");
    }

    #[test]
    fn set_options_replaces_without_filtering() {
        let mut state = State::with_value(
            vec!["a", "b"],
            "hello",
            Filter::Custom(Box::new(contains_ignore_case)),
        );

        // `set_options` stores the pool verbatim: even though "hello" matches
        // neither new option, both remain visible. Remote responses rely on
        // this — a second local filter would drop valid server matches.
        state.set_options(vec!["c", "d"]);
        assert_eq!(state.value(), "hello");
        assert_eq!(state.options(), &["c", "d"]);

        let filtered = state.with_inner(|inner| inner.filtered_options.options.len());
        assert_eq!(filtered, 2);
    }

    #[test]
    fn set_options_filtered_reapplies_the_rule() {
        let mut state = State::with_value(
            vec!["a", "b"],
            "rust",
            Filter::Custom(Box::new(contains_ignore_case)),
        );

        // Unlike `set_options`, this narrows the new pool against the current
        // text — useful when a sibling control swaps a searchable pool.
        state.set_options_filtered(vec!["rust-lang", "cargo"]);
        let filtered = state.with_inner(|inner| inner.filtered_options.options.clone());
        assert_eq!(filtered, vec!["rust-lang"]);
    }

    #[test]
    fn set_options_accepts_any_into_iterator() {
        let mut state: State<i32> = State::new(Vec::new(), Filter::None);

        // No `.collect()` required at the call site.
        state.set_options(1..4);
        assert_eq!(state.options(), &[1, 2, 3]);
    }

    #[test]
    fn set_filter_recomputes_immediately() {
        let state: State<&str> = State::new(names(), Filter::None);
        state.set_value("rust");

        // Nothing filtered yet.
        let before = state.with_inner(|inner| inner.filtered_options.options.len());
        assert_eq!(before, names().len());

        state.set_filter(Filter::Custom(Box::new(contains_ignore_case)));
        let after = state.with_inner(|inner| inner.filtered_options.options.clone());
        assert_eq!(after, vec!["rust-lang", "rustdoc"]);
    }

    #[test]
    fn default_state_is_empty_and_unfiltered() {
        let state: State<&str> = State::default();
        assert!(state.options().is_empty());
        assert_eq!(state.value(), "");
    }
}
