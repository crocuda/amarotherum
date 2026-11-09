//! # iced-autocomplete
//!
//! An autocomplete text input widget for [Iced](https://iced.rs): a
//! `text_input`-like field with a suggestion menu.
//!
//! Unlike `combo_box`, the text value is the single source of truth — the
//! option list only serves as hints for quick filling, and content typed
//! outside of the option list is submitted as a regular value. This makes the
//! widget suitable for both "pick from a known list" and "type anything" use
//! cases.
//!
//! # Filtering
//!
//! [`State::new`] and [`State::with_value`] require a [`Filter`]. There is no
//! default matching rule, because what counts as a match depends on your data
//! and your users' intent — case sensitivity, which fields to search, whether
//! initials or pinyin should count, and how non-ASCII text is handled. Any
//! built-in guess would be silently wrong for some callers, so the rule is
//! yours to declare:
//!
//! - [`Filter::Custom`] — your own predicate, for in-memory pools.
//! - [`Filter::None`] — no filtering; the pool is shown exactly as provided.
//!
//! # Asynchronous (remote) sources
//!
//! The widget performs no I/O: it publishes messages, and only your `update`
//! can spawn work. A remote search therefore looks like this:
//!
//! 1. `on_input` publishes your own message carrying the typed text;
//! 2. your `update` starts the request (e.g. with `Task::perform`);
//! 3. on response, hand the results to [`State::set_options`], which stores
//!    them verbatim.
//!
//! Build the state with [`Filter::None`] for this case — the server already
//! applied the matching rules, so a second local filter would wrongly narrow
//! the results.
//!
//! Debouncing and out-of-order responses are also the application's job, since
//! only it can see the requests. The `remote` example shows both.
//!
//! # Example
//!
//! ```no_run
//! use iced_autocomplete::{Filter, State, auto_complete};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Changed(String),
//!     Submitted(String),
//!     Selected(String),
//! }
//!
//! fn view(state: &State<String>) -> iced::Element<'_, Message> {
//!     auto_complete(
//!         state,
//!         "Type to search…",
//!         |text| Message::Changed(text),
//!         |text| Message::Submitted(text),
//!         Some(|option| Message::Selected(option)),
//!     )
//!     .width(300)
//!     .into()
//! }
//!
//! // The filtering rule is stated explicitly — the widget does not guess how
//! // your options should match.
//! fn make_state() -> State<String> {
//!     State::new(
//!         vec!["Rust".to_string(), "Ruby".to_string()],
//!         Filter::Custom(Box::new(|option: &String, query: &str| {
//!             option.to_lowercase().contains(&query.to_lowercase())
//!         })),
//!     )
//! }
//! ```
//!
//! See the `examples/` directory for complete, runnable applications.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

mod auto_complete;

pub use auto_complete::{AutoComplete, Catalog, Filter, State, auto_complete};
