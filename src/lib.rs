//! `lgpui_core`: the UI-independent core of l-gpui.
//!
//! l-gpui never talks to Lark servers directly. Every operation is delegated
//! to the official `lark-cli` (npm package `@larksuite/cli`), which owns
//! configuration, OAuth sign-in and token storage. This crate:
//!
//! * spawns `lark-cli` and normalises its JSON envelope and errors ([`runner`]),
//! * detects the sign-in state ([`auth`]),
//! * builds per-domain arguments and tolerantly parses results ([`domains`]),
//! * exposes a small typed facade over all of it ([`client::Lark`]).

pub mod auth;
pub mod client;
pub mod domains;
pub mod json;
pub mod runner;
pub mod time;

pub use auth::SignInState;
pub use client::Lark;
pub use runner::{CliError, Runner};
