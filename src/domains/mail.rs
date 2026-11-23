//! Mail: triage list, read one, send.

use serde_json::Value;

use super::args;
use crate::json::{find_array, first_str};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MailSummary {
    pub message_id: String,
    pub date: String,
    pub from: String,
    pub subject: String,
    pub unread: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MailMessage {
    pub message_id: String,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub date: String,
    pub body: String,
}

pub fn triage_args(max: u32, query: Option<&str>) -> Vec<String> {
    let mut a = args(&["mail", "+triage", "--max"]);
    a.push(max.clamp(1, 400).to_string());
    if let Some(q) = query.map(str::trim).filter(|q| !q.is_empty()) {
        a.extend(["--query".into(), q.to_string()]);
    }
    a.extend(args(&["--format", "json"]));
    a
}

pub fn message_args(message_id: &str) -> Vec<String> {
    args(&["mail", "+message", "--message-id", message_id, "--html=false", "--format", "json"])
}

/// `--confirm-send` is included because the user pressed Send in the UI;
/// without it lark-cli only saves a draft.
pub fn send_args(to: &[String], subject: &str, body: &str) -> Vec<String> {
    let mut a = args(&["mail", "+send"]);
    for r in to.iter().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        a.extend(["--to".into(), r.to_string()]);
    }
    a.extend(args(&["--subject", subject, "--body", body, "--plain-text", "--confirm-send", "--format", "json"]));
    a
}

/// Split a free-form recipient field on commas/semicolons.
pub fn split_recipients(s: &str) -> Vec<String> {
    s.split([',', ';']).map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}

pub fn parse_triage(v: &Value) -> Vec<MailSummary> {
    find_array(v, &["messages", "items"])
        .iter()
        .map(|m| MailSummary {
            message_id: first_str(m, &["message_id", "id"]),
            date: first_str(m, &["date", "internal_date", "time"]),
            from: first_str(m, &["from", "from.address", "sender"]),
            subject: {
                let s = first_str(m, &["subject"]);
                if s.is_empty() { "(No subject)".into() } else { s }
            },
            unread: first_str(m, &["labels"]).to_uppercase().contains("UNREAD"),
        })
        .collect()
}

fn addr_list(v: &Value, key: &str) -> String {
    match crate::json::path(v, key) {
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| x.as_str().map(String::from).unwrap_or_else(|| first_str(x, &["address", "mail_address", "name"])))
            .collect::<Vec<_>>()
            .join(", "),
        Some(x) => crate::json::scalar(x).unwrap_or_else(|| first_str(x, &["address", "name"])),
        None => String::new(),
    }
}

pub fn parse_message(v: &Value) -> MailMessage {
    let m = find_array(v, &["messages"]).first().unwrap_or(v);
    let m = m.get("message").unwrap_or(m);
    MailMessage {
        message_id: first_str(m, &["message_id", "id"]),
        subject: first_str(m, &["subject"]),
        from: {
            let f = first_str(m, &["from"]);
            if f.is_empty() { addr_list(m, "head_from") } else { f }
        },
        to: addr_list(m, "to"),
        date: first_str(m, &["date", "internal_date"]),
        body: first_str(m, &["body_plain_text", "body_text", "plain_text", "body", "snippet"]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_triage_and_message() {
        let v = serde_json::json!({"messages":[{"message_id":"m1","date":"Fri, 21 Mar 2026 11:40:00 +0800","from":"Alice <a@x.com>","subject":"Weekly","labels":"INBOX,UNREAD"}],"has_more":false});
        let t = parse_triage(&v);
        assert_eq!(t[0].from, "Alice <a@x.com>");
        assert!(t[0].unread);
        let m = parse_message(&serde_json::json!({"message_id":"m1","subject":"Weekly","to":["b@x.com","c@x.com"],"body_plain_text":"hello"}));
        assert_eq!(m.body, "hello");
        assert_eq!(m.to, "b@x.com, c@x.com");
    }

    #[test]
    fn send_args_have_recipients_and_confirm() {
        let a = send_args(&split_recipients("a@x.com; b@x.com,"), "S", "B");
        assert_eq!(a.iter().filter(|x| *x == "--to").count(), 2);
        assert!(a.contains(&"--confirm-send".to_string()));
        assert!(a.contains(&"--plain-text".to_string()));
        assert!(!triage_args(10, Some(" ")).contains(&"--query".to_string()));
        assert!(triage_args(10, Some("x")).contains(&"--query".to_string()));
    }
}
