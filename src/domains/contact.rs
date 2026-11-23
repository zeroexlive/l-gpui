//! Contacts search.

use serde_json::Value;

use super::args;
use crate::json::{find_array, first_str};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct User {
    pub open_id: String,
    pub name: String,
    pub email: String,
    pub department: String,
    pub title: String,
}

pub fn search_args(query: &str) -> Vec<String> {
    args(&["contact", "+search-user", "--query", query, "--format", "json"])
}

pub fn parse_users(v: &Value) -> Vec<User> {
    find_array(v, &["users", "items"])
        .iter()
        .map(|u| User {
            open_id: first_str(u, &["open_id", "user_id", "id"]),
            name: first_str(u, &["name", "en_name", "display_name"]),
            email: first_str(u, &["email", "enterprise_email"]),
            department: first_str(u, &["department_name", "department", "departments.0.name", "department_path"]),
            title: first_str(u, &["job_title", "title", "description"]),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_users() {
        let v = serde_json::json!({"users":[{"open_id":"ou_1","name":"Ada","email":"ada@x.com","department_name":"R&D"}]});
        let u = super::parse_users(&v);
        assert_eq!(u[0].name, "Ada");
        assert_eq!(u[0].department, "R&D");
    }
}
