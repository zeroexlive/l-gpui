//! Tolerant JSON helpers. lark-cli output shapes differ slightly between
//! shortcuts and versions (`{ok,data:{..}}` envelopes, bare objects, nested
//! `data.data`), so parsers look values up by several candidate paths.

use serde_json::Value;

/// Look up a dotted path such as `sender.name` or `start_time.datetime`.
pub fn path<'a>(v: &'a Value, p: &str) -> Option<&'a Value> {
    let mut cur = v;
    for key in p.split('.') {
        cur = match cur {
            Value::Object(m) => m.get(key)?,
            Value::Array(a) => a.get(key.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(cur)
}

/// Render a scalar as a string (strings verbatim, numbers/bools formatted).
pub fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// First non-empty scalar found at any of `paths`.
pub fn first_str(v: &Value, paths: &[&str]) -> String {
    paths
        .iter()
        .filter_map(|p| path(v, p).and_then(scalar))
        .find(|s| !s.trim().is_empty())
        .unwrap_or_default()
}

/// First boolean found at any of `paths` (accepts `"true"` strings too).
pub fn first_bool(v: &Value, paths: &[&str]) -> Option<bool> {
    paths.iter().find_map(|p| match path(v, p)? {
        Value::Bool(b) => Some(*b),
        Value::String(s) if s == "true" => Some(true),
        Value::String(s) if s == "false" => Some(false),
        _ => None,
    })
}

/// Containers in which a result array may live, outermost first.
fn containers(v: &Value) -> Vec<&Value> {
    let mut out = vec![v];
    for k in ["data", "result"] {
        if let Some(inner) = v.get(k) {
            out.push(inner);
            if let Some(inner2) = inner.get("data") {
                out.push(inner2);
            }
        }
    }
    out
}

/// Find the result list: the first array under one of `keys` in the value or
/// its `data`/`result` wrappers, or the value itself if it is an array.
pub fn find_array<'a>(v: &'a Value, keys: &[&str]) -> &'a [Value] {
    for c in containers(v) {
        if let Value::Array(a) = c {
            return a;
        }
        for k in keys {
            if let Some(Value::Array(a)) = c.get(*k) {
                return a;
            }
        }
    }
    &[]
}

/// Remove simple HTML tags (e.g. `<em>` search highlights) and decode the
/// handful of entities lark-cli emits.
pub fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn finds_arrays_in_envelopes() {
        let a = json!({"ok": true, "data": {"chats": [1, 2]}});
        assert_eq!(find_array(&a, &["chats"]).len(), 2);
        let b = json!({"chats": [1]});
        assert_eq!(find_array(&b, &["chats"]).len(), 1);
        let c = json!([1, 2, 3]);
        assert_eq!(find_array(&c, &["x"]).len(), 3);
        let d = json!({"data": {"data": {"items": [1]}}});
        assert_eq!(find_array(&d, &["items"]).len(), 1);
        assert!(find_array(&json!({}), &["items"]).is_empty());
    }

    #[test]
    fn first_str_and_paths() {
        let v = json!({"a": {"b": ""}, "c": 5, "d": {"e": "x"}});
        assert_eq!(first_str(&v, &["a.b", "c"]), "5");
        assert_eq!(first_str(&v, &["d.e"]), "x");
        assert_eq!(first_str(&v, &["zz"]), "");
    }

    #[test]
    fn strips_highlight_tags() {
        assert_eq!(strip_tags("Q3 <em>plan</em> &amp; notes"), "Q3 plan & notes");
    }
}
