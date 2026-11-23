//! Typed facade over [`Runner`]. All methods block; the UI calls them on a
//! background executor.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::auth::{self, SignInState};
use crate::domains::{approval, calendar, contact, docs, im, mail, minutes, task};
use crate::runner::{CliError, Runner};

pub type Result<T> = std::result::Result<T, CliError>;

#[derive(Debug, Clone, Default)]
pub struct Lark {
    pub runner: Runner,
}

static SEQ: AtomicU64 = AtomicU64::new(0);

/// A unique idempotency key (≤ 50 chars) for message sends.
pub fn idempotency_key() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("lgpui-{:x}-{:x}-{}", nanos, std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed))
}

impl Lark {
    pub fn new(runner: Runner) -> Self {
        Self { runner }
    }

    pub fn from_env() -> Self {
        Self::new(Runner::from_env())
    }

    pub fn sign_in_state(&self) -> SignInState {
        auth::check(&self.runner)
    }

    pub fn chats(&self, page_size: u32) -> Result<Vec<im::Chat>> {
        Ok(im::parse_chats(&self.runner.run(&im::chat_list_args(page_size))?))
    }

    pub fn messages(&self, chat_id: &str, page_size: u32) -> Result<Vec<im::Message>> {
        let v = self.runner.run(&im::messages_args(chat_id, page_size))?;
        Ok(im::chronological(im::parse_messages(&v)))
    }

    pub fn send_text(&self, chat_id: &str, text: &str) -> Result<()> {
        self.runner.run(&im::send_text_args(chat_id, text, &idempotency_key())).map(|_| ())
    }

    pub fn search_messages(&self, query: &str) -> Result<Vec<im::Message>> {
        Ok(im::parse_messages(&self.runner.run(&im::search_args(query, 30))?))
    }

    /// Agenda from the start of today for `days` days.
    pub fn agenda(&self, days: i64) -> Result<Vec<calendar::Event>> {
        let start = crate::time::day_start_rfc3339(0);
        let end = crate::time::day_start_rfc3339(days.max(1));
        Ok(calendar::parse_events(&self.runner.run(&calendar::agenda_args(&start, &end))?))
    }

    pub fn my_tasks(&self) -> Result<Vec<task::Task>> {
        Ok(task::parse_tasks(&self.runner.run(&task::my_tasks_args())?))
    }

    pub fn complete_task(&self, guid: &str) -> Result<()> {
        self.runner.run(&task::complete_args(guid)).map(|_| ())
    }

    pub fn create_task(&self, summary: &str) -> Result<()> {
        self.runner.run(&task::create_args(summary)).map(|_| ())
    }

    /// `docs +search`, falling back to `drive +search` if the former fails
    /// for a reason other than sign-in.
    pub fn search_docs(&self, query: &str) -> Result<Vec<docs::DocHit>> {
        match self.runner.run(&docs::docs_search_args(query)) {
            Ok(v) => Ok(docs::parse_hits(&v)),
            Err(e) if e.needs_sign_in() => Err(e),
            Err(_) => Ok(docs::parse_hits(&self.runner.run(&docs::drive_search_args(query))?)),
        }
    }

    pub fn wiki_spaces(&self) -> Result<Vec<docs::WikiSpace>> {
        Ok(docs::parse_spaces(&self.runner.run(&docs::wiki_spaces_args())?))
    }

    pub fn mail_triage(&self, max: u32, query: Option<&str>) -> Result<Vec<mail::MailSummary>> {
        Ok(mail::parse_triage(&self.runner.run(&mail::triage_args(max, query))?))
    }

    pub fn mail_message(&self, id: &str) -> Result<mail::MailMessage> {
        Ok(mail::parse_message(&self.runner.run(&mail::message_args(id))?))
    }

    pub fn mail_send(&self, to: &str, subject: &str, body: &str) -> Result<()> {
        let rcpts = mail::split_recipients(to);
        if rcpts.is_empty() {
            return Err(CliError::Api { kind: "input".into(), message: "Add at least one recipient".into(), hint: String::new() });
        }
        self.runner.run(&mail::send_args(&rcpts, subject, body)).map(|_| ())
    }

    pub fn search_users(&self, query: &str) -> Result<Vec<contact::User>> {
        Ok(contact::parse_users(&self.runner.run(&contact::search_args(query))?))
    }

    pub fn approvals(&self, topic: approval::Topic) -> Result<Vec<approval::ApprovalTask>> {
        Ok(approval::parse_tasks(&self.runner.run(&approval::query_args(topic))?))
    }

    pub fn search_minutes(&self, query: &str) -> Result<Vec<minutes::Minute>> {
        Ok(minutes::parse_minutes(&self.runner.run(&minutes::search_args(query))?))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn idempotency_keys_are_unique_and_short() {
        let a = super::idempotency_key();
        let b = super::idempotency_key();
        assert_ne!(a, b);
        assert!(a.len() <= 50);
    }
}
