//! Timestamp helpers. lark-cli returns times as epoch seconds, epoch
//! milliseconds (often as strings), RFC 3339, or already formatted text.

use chrono::{DateTime, Local, NaiveDate, TimeZone};

/// Parse an epoch (s or ms) or RFC 3339 / RFC 2822 string into local time.
pub fn parse(raw: &str) -> Option<DateTime<Local>> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(n) = s.parse::<i64>() {
        let ms = if n > 100_000_000_000 { n } else { n * 1000 };
        return Local.timestamp_millis_opt(ms).single();
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Local));
    }
    if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
        return Some(dt.with_timezone(&Local));
    }
    None
}

/// Human display: `HH:MM` for today, `YYYY-MM-DD HH:MM` otherwise. Unknown
/// formats are returned unchanged so nothing is lost.
pub fn display(raw: &str) -> String {
    match parse(raw) {
        Some(dt) if dt.date_naive() == Local::now().date_naive() => dt.format("%H:%M").to_string(),
        Some(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
        None => raw.trim().to_string(),
    }
}

/// Display for all-day dates (`YYYY-MM-DD`) or date-times.
pub fn display_date_or_time(raw: &str) -> String {
    if NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d").is_ok() {
        return raw.trim().to_string();
    }
    display(raw)
}

/// Sort key in epoch milliseconds (0 when unparseable).
pub fn sort_key(raw: &str) -> i64 {
    parse(raw).map(|d| d.timestamp_millis()).unwrap_or(0)
}

/// RFC 3339 for the start of the local day `offset_days` from today.
pub fn day_start_rfc3339(offset_days: i64) -> String {
    let date = Local::now().date_naive() + chrono::Duration::days(offset_days);
    let naive = date.and_hms_opt(0, 0, 0).expect("midnight exists");
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map(|d| d.to_rfc3339())
        .unwrap_or_else(|| format!("{date}T00:00:00Z"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_epochs() {
        let s = parse("1700000000").unwrap();
        let ms = parse("1700000000000").unwrap();
        assert_eq!(s, ms);
        assert!(parse("2026-03-21T11:40:00+08:00").is_some());
        assert!(parse("Sat, 21 Mar 2026 11:40:00 +0800").is_some());
        assert!(parse("garbage").is_none());
    }

    #[test]
    fn display_keeps_unknown() {
        assert_eq!(display("yesterday"), "yesterday");
        assert_eq!(display_date_or_time("2026-01-02"), "2026-01-02");
        assert!(display("1700000000").contains("2023-11-1"));
    }

    #[test]
    fn day_start_is_rfc3339() {
        assert!(DateTime::parse_from_rfc3339(&day_start_rfc3339(0)).is_ok());
    }
}
