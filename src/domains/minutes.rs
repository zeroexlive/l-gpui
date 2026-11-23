//! Minutes search.

use serde_json::Value;

use super::args;
use crate::json::{find_array, first_str};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Minute {
    pub token: String,
    pub title: String,
    pub url: String,
    pub owner: String,
    pub time: String,
    pub duration: String,
}

pub fn search_args(query: &str) -> Vec<String> {
    let mut a = args(&["minutes", "+search"]);
    if !query.trim().is_empty() {
        a.extend(["--query".into(), query.trim().to_string()]);
    }
    a.extend(args(&["--format", "json"]));
    a
}

pub fn parse_minutes(v: &Value) -> Vec<Minute> {
    find_array(v, &["items", "minutes", "results"])
        .iter()
        .map(|m| Minute {
            token: first_str(m, &["minute_token", "token", "object_token"]),
            title: {
                let s = crate::json::strip_tags(&first_str(m, &["title", "topic", "name"]));
                if s.is_empty() { "(Untitled minute)".into() } else { s }
            },
            url: first_str(m, &["url", "link"]),
            owner: first_str(m, &["owner_name", "owner.name", "owner_id"]),
            time: first_str(m, &["create_time", "start_time", "time"]),
            duration: first_str(m, &["duration"]),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_minutes() {
        let v = serde_json::json!({"items":[{"minute_token":"obc","title":"Weekly sync","url":"https://x/minutes/obc"}]});
        let m = super::parse_minutes(&v);
        assert_eq!(m[0].title, "Weekly sync");
        assert!(!super::search_args("").contains(&"--query".to_string()));
    }
}
