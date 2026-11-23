//! Per-domain argument builders and tolerant parsers.
//!
//! Each domain exposes `*_args(...) -> Vec<String>` (pure, testable) and
//! `parse_*(&Value) -> Vec<T>` that accepts the unwrapped lark-cli payload.

pub mod approval;
pub mod calendar;
pub mod contact;
pub mod docs;
pub mod im;
pub mod mail;
pub mod minutes;
pub mod task;

pub(crate) fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}
