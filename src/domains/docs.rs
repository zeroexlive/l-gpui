//! Docs / Drive search and Wiki spaces.

use serde_json::Value;

use super::args;
use crate::json::{find_array, first_str, strip_tags};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocHit {
    pub title: String,
    pub url: String,
    pub kind: String,
    pub owner: String,
    pub updated: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WikiSpace {
    pub space_id: String,
    pub name: String,
    pub description: String,
}

pub fn docs_search_args(query: &str) -> Vec<String> {
    args(&["docs", "+search", "--query", query, "--format", "json"])
}

pub fn drive_search_args(query: &str) -> Vec<String> {
    args(&["drive", "+search", "--query", query, "--format", "json"])
}

pub fn wiki_spaces_args() -> Vec<String> {
    args(&["wiki", "+space-list", "--format", "json"])
}

pub fn parse_hits(v: &Value) -> Vec<DocHit> {
    find_array(v, &["results", "items", "docs", "files"])
        .iter()
        .map(|h| {
            let raw = first_str(h, &["title_highlighted", "title", "name", "result_meta.title"]);
            DocHit {
                title: if raw.is_empty() { "(Untitled)".into() } else { strip_tags(&raw) },
                url: first_str(h, &["result_meta.url", "url", "link"]),
                kind: first_str(h, &["result_meta.doc_types", "result_meta.type", "doc_type", "type", "entity_type"]),
                owner: first_str(h, &["result_meta.owner_name", "owner_name", "owner"]),
                updated: first_str(h, &["result_meta.update_time", "update_time", "edit_time"]),
            }
        })
        .collect()
}

pub fn parse_spaces(v: &Value) -> Vec<WikiSpace> {
    find_array(v, &["spaces", "items"])
        .iter()
        .map(|s| WikiSpace {
            space_id: first_str(s, &["space_id", "id"]),
            name: first_str(s, &["name", "title"]),
            description: first_str(s, &["description"]),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hits_and_spaces() {
        let v = serde_json::json!({"results":[{"title_highlighted":"Q3 <em>OKR</em>","result_meta":{"url":"https://x.larksuite.com/docx/abc","doc_types":"DOCX"}}]});
        let h = parse_hits(&v);
        assert_eq!(h[0].title, "Q3 OKR");
        assert_eq!(h[0].url, "https://x.larksuite.com/docx/abc");
        assert_eq!(h[0].kind, "DOCX");
        let s = parse_spaces(&serde_json::json!({"data":{"spaces":[{"space_id":"7","name":"Eng"}]}}));
        assert_eq!(s[0].name, "Eng");
    }
}
