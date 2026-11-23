//! Approvals: pending (topic 1) and handled (topic 2) tasks.

use serde_json::Value;

use super::args;
use crate::json::{find_array, first_str};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    Pending,
    Handled,
}

impl Topic {
    pub fn code(self) -> &'static str {
        match self {
            Topic::Pending => "1",
            Topic::Handled => "2",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ApprovalTask {
    pub task_id: String,
    pub instance_code: String,
    pub title: String,
    pub initiator: String,
    pub status: String,
    pub url: String,
    pub time: String,
}

pub fn query_args(topic: Topic) -> Vec<String> {
    args(&["approval", "tasks", "query", "--topic", topic.code(), "--locale", "en-US", "--format", "json"])
}

pub fn parse_tasks(v: &Value) -> Vec<ApprovalTask> {
    find_array(v, &["tasks", "items", "task_list"])
        .iter()
        .map(|t| ApprovalTask {
            task_id: first_str(t, &["task_id", "id"]),
            instance_code: first_str(t, &["instance_code", "process_code", "process_external_id"]),
            title: {
                let s = first_str(t, &["title", "summary", "definition_name", "approval_name", "process_name"]);
                if s.is_empty() { "(Approval)".into() } else { s }
            },
            initiator: first_str(t, &["initiator_names.0", "initiator_name", "initiators.0.name", "initiator"]),
            status: first_str(t, &["status", "task_status", "process_status"]),
            url: first_str(t, &["urls.pc", "url", "urls.helpdesk", "urls.mobile"]),
            time: first_str(t, &["create_time", "start_time", "update_time"]),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_approvals() {
        let v = serde_json::json!({"data":{"tasks":[{"task_id":"t1","instance_code":"ic","title":"Leave request","initiator_names":["Bo"],"status":"PENDING","urls":{"pc":"https://x"}}]}});
        let t = parse_tasks(&v);
        assert_eq!(t[0].initiator, "Bo");
        assert_eq!(t[0].url, "https://x");
        assert!(query_args(Topic::Handled).windows(2).any(|w| w == ["--topic", "2"]));
    }
}
