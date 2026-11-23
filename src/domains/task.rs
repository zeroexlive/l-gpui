//! Tasks: my open tasks, complete, create.

use serde_json::Value;

use super::args;
use crate::json::{find_array, first_bool, first_str};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Task {
    pub guid: String,
    pub summary: String,
    pub url: String,
    pub due: String,
    pub completed: bool,
}

pub fn my_tasks_args() -> Vec<String> {
    args(&["task", "+get-my-tasks", "--complete=false", "--format", "json"])
}

pub fn complete_args(guid: &str) -> Vec<String> {
    args(&["task", "+complete", "--task-id", guid, "--format", "json"])
}

pub fn create_args(summary: &str) -> Vec<String> {
    args(&["task", "+create", "--summary", summary, "--format", "json"])
}

pub fn parse_tasks(v: &Value) -> Vec<Task> {
    find_array(v, &["items", "tasks"])
        .iter()
        .map(|t| {
            let completed_at = first_str(t, &["completed_at"]);
            Task {
                guid: first_str(t, &["guid", "task_id", "id"]),
                summary: first_str(t, &["summary", "title"]),
                url: first_str(t, &["url", "applink"]),
                due: first_str(t, &["due_at", "due.timestamp", "due_time", "due"]),
                completed: first_bool(t, &["completed", "is_completed"])
                    .unwrap_or(!completed_at.is_empty() && completed_at != "0"),
            }
        })
        .filter(|t| !t.guid.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tasks() {
        let v = serde_json::json!({"items":[
            {"guid":"g1","summary":"Write report","url":"https://applink/x","due_at":"2026-03-27 18:00:00"},
            {"guid":"g2","summary":"Done one","completed_at":"1700000000000"},
            {"summary":"no guid"}
        ]});
        let t = parse_tasks(&v);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].due, "2026-03-27 18:00:00");
        assert!(!t[0].completed);
        assert!(t[1].completed);
        assert_eq!(complete_args("g1")[3], "g1");
        assert_eq!(create_args("Hi")[3], "Hi");
    }
}
