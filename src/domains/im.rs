//! Messenger: chats, messages, send, search.

use serde_json::{Value, json};

use super::args;
use crate::json::{find_array, first_str, path, scalar};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Chat {
    pub chat_id: String,
    pub name: String,
    pub mode: String,
    pub description: String,
}

impl Chat {
    pub fn display_name(&self) -> &str {
        if self.name.trim().is_empty() { &self.chat_id } else { &self.name }
    }
    pub fn initials(&self) -> String {
        initials(self.display_name())
    }
}

/// Up to two leading characters for an avatar.
pub fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let s: String = match words.as_slice() {
        [a, b, ..] => a.chars().take(1).chain(b.chars().take(1)).collect(),
        [a] => a.chars().take(2).collect(),
        [] => "?".into(),
    };
    s.to_uppercase()
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Message {
    pub message_id: String,
    pub chat_id: String,
    pub chat_name: String,
    pub sender: String,
    pub text: String,
    pub create_time: String,
    pub msg_type: String,
}

pub fn chat_list_args(page_size: u32) -> Vec<String> {
    let mut a = args(&["im", "+chat-list", "--types", "p2p,group", "--sort", "active_time", "--page-size"]);
    a.push(page_size.clamp(1, 100).to_string());
    a.extend(args(&["--format", "json"]));
    a
}

pub fn messages_args(chat_id: &str, page_size: u32) -> Vec<String> {
    let mut a = args(&["im", "+chat-messages-list", "--chat-id", chat_id, "--order", "desc", "--no-reactions"]);
    a.extend(["--page-size".into(), page_size.clamp(1, 50).to_string()]);
    a.extend(args(&["--format", "json"]));
    a
}

/// Uses `--content` JSON (not `--text`) so text starting with `@` is never
/// treated as a file reference by lark-cli.
pub fn send_text_args(chat_id: &str, text: &str, idempotency_key: &str) -> Vec<String> {
    let content = json!({ "text": text }).to_string();
    args(&[
        "im", "+messages-send", "--chat-id", chat_id, "--msg-type", "text", "--content", &content,
        "--idempotency-key", idempotency_key, "--format", "json",
    ])
}

pub fn search_args(query: &str, page_size: u32) -> Vec<String> {
    let mut a = args(&["im", "+messages-search", "--query", query, "--page-size"]);
    a.push(page_size.clamp(1, 50).to_string());
    a.extend(args(&["--format", "json"]));
    a
}

pub fn parse_chats(v: &Value) -> Vec<Chat> {
    find_array(v, &["chats", "items"])
        .iter()
        .map(|c| Chat {
            chat_id: first_str(c, &["chat_id", "id"]),
            name: first_str(c, &["name", "chat_name", "i18n_names.en_us"]),
            mode: first_str(c, &["chat_mode", "chat_type", "mode"]),
            description: first_str(c, &["description"]),
        })
        .filter(|c| !c.chat_id.is_empty())
        .collect()
}

/// Extract readable text from a message `content`, which may be plain text,
/// a JSON string like `{"text":"hi"}`, or an object.
pub fn content_text(content: &Value) -> String {
    match content {
        Value::String(s) => {
            let t = s.trim();
            if t.starts_with('{') {
                if let Ok(inner) = serde_json::from_str::<Value>(t) {
                    let x = content_text(&inner);
                    if !x.is_empty() {
                        return x;
                    }
                }
            }
            s.clone()
        }
        Value::Object(_) => {
            let x = first_str(content, &["text", "title", "content"]);
            if !x.is_empty() { x } else { content.to_string() }
        }
        other => scalar(other).unwrap_or_default(),
    }
}

pub fn parse_messages(v: &Value) -> Vec<Message> {
    find_array(v, &["messages", "items"])
        .iter()
        .map(|m| Message {
            message_id: first_str(m, &["message_id", "id"]),
            chat_id: first_str(m, &["chat_id", "chat.chat_id"]),
            chat_name: first_str(m, &["chat_name", "chat.name"]),
            sender: first_str(m, &["sender.name", "sender_name", "sender.id", "sender.sender_id.open_id"]),
            text: path(m, "content").or_else(|| path(m, "body.content")).map(content_text).unwrap_or_default(),
            create_time: first_str(m, &["create_time", "created_at", "time"]),
            msg_type: first_str(m, &["msg_type", "message_type"]),
        })
        .collect()
}

/// Messages in chronological order (lark-cli is queried newest-first).
pub fn chronological(mut msgs: Vec<Message>) -> Vec<Message> {
    msgs.sort_by_key(|m| crate::time::sort_key(&m.create_time));
    msgs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_args() {
        let a = chat_list_args(30);
        assert_eq!(a[..2], ["im", "+chat-list"]);
        assert!(a.windows(2).any(|w| w == ["--types", "p2p,group"]));
        assert!(a.windows(2).any(|w| w == ["--page-size", "30"]));
        let s = send_text_args("oc_1", "@hello \"q\"", "k1");
        let i = s.iter().position(|x| x == "--content").unwrap();
        let c: Value = serde_json::from_str(&s[i + 1]).unwrap();
        assert_eq!(c["text"], "@hello \"q\"");
        assert!(messages_args("oc_1", 500).windows(2).any(|w| w == ["--page-size", "50"]));
    }

    #[test]
    fn parses_chats_and_messages() {
        let chats = serde_json::json!({"chats":[{"chat_id":"oc_1","name":"Team","chat_mode":"group"},{"chat_id":"oc_2","name":""},{"name":"no id"}]});
        let c = parse_chats(&chats);
        assert_eq!(c.len(), 2);
        assert_eq!(c[1].display_name(), "oc_2");
        assert_eq!(c[0].initials(), "TE");

        let msgs = serde_json::json!({"messages":[
            {"message_id":"om_2","content":"second","sender":{"name":"Bo"},"create_time":"1700000060000"},
            {"message_id":"om_1","content":"{\"text\":\"first\"}","sender":{"id":"ou_1"},"create_time":"1700000000000"}
        ]});
        let m = chronological(parse_messages(&msgs));
        assert_eq!(m[0].text, "first");
        assert_eq!(m[0].sender, "ou_1");
        assert_eq!(m[1].sender, "Bo");
    }

    #[test]
    fn initials_work() {
        assert_eq!(initials("ada lovelace"), "AL");
        assert_eq!(initials(""), "?");
    }
}
