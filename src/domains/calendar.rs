//! Calendar agenda.

use serde_json::Value;

use super::args;
use crate::json::{find_array, first_str};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Event {
    pub event_id: String,
    pub summary: String,
    pub start: String,
    pub end: String,
    pub all_day: bool,
    pub location: String,
    pub organizer: String,
}

pub fn agenda_args(start: &str, end: &str) -> Vec<String> {
    args(&["calendar", "+agenda", "--start", start, "--end", end, "--format", "json"])
}

fn when(e: &Value, key: &str) -> (String, bool) {
    let dt = first_str(e, &[&format!("{key}_time.datetime"), &format!("{key}.datetime"), &format!("{key}_time.timestamp"), &format!("{key}.timestamp")]);
    if !dt.is_empty() {
        return (dt, false);
    }
    let d = first_str(e, &[&format!("{key}_time.date"), &format!("{key}.date")]);
    if !d.is_empty() {
        return (d, true);
    }
    (first_str(e, &[&format!("{key}_time"), key]), false)
}

pub fn parse_events(v: &Value) -> Vec<Event> {
    let mut out: Vec<Event> = find_array(v, &["events", "items", "agenda"])
        .iter()
        .map(|e| {
            let (start, all_day) = when(e, "start");
            let (end, _) = when(e, "end");
            Event {
                event_id: first_str(e, &["event_id", "id"]),
                summary: {
                    let s = first_str(e, &["summary", "title"]);
                    if s.is_empty() { "(No title)".into() } else { s }
                },
                start,
                end,
                all_day,
                location: first_str(e, &["location.name", "location", "vchat.meeting_url"]),
                organizer: first_str(e, &["event_organizer.display_name", "organizer.display_name", "organizer"]),
            }
        })
        .collect();
    out.sort_by_key(|e| crate::time::sort_key(&e.start));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_timed_and_all_day() {
        let v = serde_json::json!([
            {"event_id":"e2","summary":"Standup","start_time":{"datetime":"2026-03-21T10:00:00+08:00"},"end_time":{"datetime":"2026-03-21T10:15:00+08:00"}},
            {"event_id":"e1","summary":"","start_time":{"date":"2026-03-21"},"end_time":{"date":"2026-03-22"}}
        ]);
        let e = parse_events(&v);
        assert_eq!(e.len(), 2);
        let allday = e.iter().find(|x| x.event_id == "e1").unwrap();
        assert!(allday.all_day);
        assert_eq!(allday.summary, "(No title)");
        let s = e.iter().find(|x| x.event_id == "e2").unwrap();
        assert!(s.start.starts_with("2026-03-21T10"));
        assert!(agenda_args("a", "b").windows(2).any(|w| w == ["--start", "a"]));
    }
}
