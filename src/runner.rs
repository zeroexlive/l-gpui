//! Spawning `lark-cli` and normalising its output.
//!
//! lark-cli prints JSON. Successful calls usually print an envelope like
//! `{"ok": true, "data": {...}}` (some shortcuts print the payload bare).
//! Failures print `{"ok": false, "error": {"type", "subtype", "message",
//! "hint"}}` on stderr and exit non-zero (exit 3 = not configured).
//! [`normalize`] turns any of these into `Result<Value, CliError>`.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::Value;

/// Environment variable overriding the lark-cli executable.
pub const ENV_PROGRAM: &str = "LARK_CLI";
/// Environment variable selecting the identity (`user` or `bot`) passed as `--as`.
pub const ENV_IDENTITY: &str = "LARK_CLI_AS";

/// Exit code lark-cli uses when it has not been configured yet.
pub const EXIT_NOT_CONFIGURED: i32 = 3;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CliError {
    #[error("lark-cli was not found ({0}). Install it with `npm i -g @larksuite/cli`.")]
    NotInstalled(String),
    #[error("lark-cli is not configured. {hint}")]
    NotConfigured { hint: String },
    #[error("Not signed in to Lark. {hint}")]
    NotSignedIn { hint: String },
    #[error("{message}")]
    Api { kind: String, message: String, hint: String },
    #[error("lark-cli exited with code {code}: {stderr}")]
    Exit { code: i32, stderr: String },
    #[error("could not parse lark-cli output: {0}")]
    Parse(String),
    #[error("failed to run lark-cli: {0}")]
    Io(String),
}

impl CliError {
    /// True when the user must (re)run the sign-in flow.
    pub fn needs_sign_in(&self) -> bool {
        matches!(self, CliError::NotInstalled(_) | CliError::NotConfigured { .. } | CliError::NotSignedIn { .. })
    }
}

/// How to invoke lark-cli.
#[derive(Debug, Clone)]
pub struct Runner {
    pub program: PathBuf,
    pub identity: Option<String>,
}

impl Default for Runner {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Top-level commands that do not accept `--as`.
const NO_IDENTITY: &[&str] = &["auth", "config", "skills", "version", "--version", "help"];

impl Runner {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self { program: program.into(), identity: None }
    }

    pub fn with_identity(mut self, identity: Option<String>) -> Self {
        self.identity = identity.filter(|s| !s.trim().is_empty());
        self
    }

    /// `LARK_CLI` (default `lark-cli` on `PATH`) and `LARK_CLI_AS`.
    pub fn from_env() -> Self {
        let program = std::env::var_os(ENV_PROGRAM)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("lark-cli"));
        Self::new(program).with_identity(std::env::var(ENV_IDENTITY).ok())
    }

    /// Final argument vector (adds `--as <identity>` where applicable).
    pub fn full_args(&self, args: &[String]) -> Vec<String> {
        let mut out = args.to_vec();
        if let Some(id) = &self.identity {
            let skip = args.first().is_some_and(|a| NO_IDENTITY.contains(&a.as_str()));
            if !skip && !args.iter().any(|a| a == "--as" || a.starts_with("--as=")) {
                out.push("--as".into());
                out.push(id.clone());
            }
        }
        out
    }

    /// Run lark-cli synchronously (call this off the UI thread).
    pub fn run(&self, args: &[String]) -> Result<Value, CliError> {
        let output = Command::new(&self.program)
            .args(self.full_args(args))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => CliError::NotInstalled(self.program.display().to_string()),
                _ => CliError::Io(e.to_string()),
            })?;
        normalize(
            output.status.code(),
            &String::from_utf8_lossy(&output.stdout),
            &String::from_utf8_lossy(&output.stderr),
        )
    }
}

/// Find the first JSON value in a text blob (lark-cli may print warnings
/// such as `warning: bot_strip_p2p: ...` before or after it).
pub fn extract_json(text: &str) -> Option<Value> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str(t) {
        return Some(v);
    }
    for (i, ch) in t.char_indices() {
        if ch == '{' || ch == '[' {
            let mut stream = serde_json::Deserializer::from_str(&t[i..]).into_iter::<Value>();
            if let Some(Ok(v)) = stream.next() {
                return Some(v);
            }
        }
    }
    None
}

fn classify(err: &Value, fallback_hint: &str) -> CliError {
    let s = |k: &str| err.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
    let kind = s("type");
    let subtype = s("subtype");
    let mut message = s("message");
    if message.is_empty() {
        message = s("msg");
    }
    let mut hint = s("hint");
    if hint.is_empty() {
        hint = fallback_hint.to_string();
    }
    let k = format!("{kind} {subtype}").to_lowercase();
    if k.contains("config") {
        return CliError::NotConfigured { hint };
    }
    if ["auth", "login", "token", "unauthor", "not_logged", "expired"].iter().any(|w| k.contains(w)) {
        return CliError::NotSignedIn { hint };
    }
    if message.is_empty() {
        message = if kind.is_empty() { "lark-cli reported an error".into() } else { format!("{kind} error") };
    }
    CliError::Api { kind, message, hint }
}

/// Keep error text short and free of anything that looks like a token.
pub fn sanitize(s: &str) -> String {
    let redacted: Vec<String> = s
        .split_whitespace()
        .map(|w| {
            let core = w.trim_matches(|c: char| !c.is_ascii_alphanumeric());
            let looks_secret = core.len() >= 24
                && core.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
                && core.chars().any(|c| c.is_ascii_digit())
                && core.chars().any(|c| c.is_ascii_alphabetic());
            if looks_secret { "[redacted]".to_string() } else { w.to_string() }
        })
        .collect();
    let joined = redacted.join(" ");
    if joined.chars().count() > 400 {
        format!("{}…", joined.chars().take(400).collect::<String>())
    } else {
        joined
    }
}

/// Normalise one lark-cli invocation's exit code and output streams.
pub fn normalize(code: Option<i32>, stdout: &str, stderr: &str) -> Result<Value, CliError> {
    let code = code.unwrap_or(-1);
    let parsed = extract_json(stdout).or_else(|| extract_json(stderr));

    if let Some(v) = &parsed {
        if v.get("ok").and_then(Value::as_bool) == Some(false) || (code != 0 && v.get("error").is_some()) {
            let err = v.get("error").cloned().unwrap_or(Value::Null);
            let err = if err.is_object() { err } else { serde_json::json!({ "message": err.as_str().unwrap_or("") }) };
            return Err(classify(&err, ""));
        }
    }

    if code != 0 {
        if code == EXIT_NOT_CONFIGURED {
            return Err(CliError::NotConfigured { hint: "Run `lark-cli config init --new`.".into() });
        }
        let text = if stderr.trim().is_empty() { stdout } else { stderr };
        return Err(CliError::Exit { code, stderr: sanitize(text.trim()) });
    }

    match parsed {
        Some(v) => Ok(unwrap_envelope(v)),
        None if stdout.trim().is_empty() => Ok(Value::Null),
        None => Err(CliError::Parse(sanitize(stdout.trim()))),
    }
}

/// `{ok:true, data:X}` → `X`; anything else is returned unchanged.
pub fn unwrap_envelope(v: Value) -> Value {
    if v.get("ok").and_then(Value::as_bool) == Some(true) {
        if let Some(d) = v.get("data") {
            return d.clone();
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unwraps_ok_envelope() {
        let v = normalize(Some(0), r#"{"ok":true,"data":{"chats":[]}}"#, "").unwrap();
        assert_eq!(v, json!({"chats": []}));
    }

    #[test]
    fn bare_payload_passes_through() {
        let v = normalize(Some(0), r#"{"messages":[{"subject":"x"}]}"#, "").unwrap();
        assert_eq!(v["messages"][0]["subject"], "x");
    }

    #[test]
    fn tolerates_warnings_around_json() {
        let out = "warning: bot_strip_p2p: something\n{\"ok\":true,\"data\":{\"chats\":[1]}}\n";
        assert_eq!(normalize(Some(0), out, "").unwrap(), json!({"chats":[1]}));
    }

    #[test]
    fn not_configured_from_stderr_envelope() {
        let err = r#"{"ok":false,"error":{"type":"config","subtype":"not_configured","message":"not configured","hint":"run `lark-cli config init --new`"}}"#;
        match normalize(Some(3), "", err) {
            Err(CliError::NotConfigured { hint }) => assert!(hint.contains("config init")),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn exit3_without_json_is_not_configured() {
        assert!(matches!(normalize(Some(3), "", "boom"), Err(CliError::NotConfigured { .. })));
    }

    #[test]
    fn auth_errors_need_sign_in() {
        let err = r#"{"ok":false,"error":{"type":"auth","message":"token expired","hint":"run lark-cli auth login"}}"#;
        let e = normalize(Some(1), "", err).unwrap_err();
        assert!(matches!(e, CliError::NotSignedIn { .. }));
        assert!(e.needs_sign_in());
    }

    #[test]
    fn api_errors_keep_message() {
        let err = r#"{"ok":false,"error":{"type":"api","message":"permission denied","hint":"grant scope"}}"#;
        match normalize(Some(1), "", err) {
            Err(CliError::Api { message, hint, .. }) => {
                assert_eq!(message, "permission denied");
                assert_eq!(hint, "grant scope");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn plain_failure_is_exit_error() {
        match normalize(Some(2), "", "unknown flag --nope") {
            Err(CliError::Exit { code, stderr }) => {
                assert_eq!(code, 2);
                assert!(stderr.contains("unknown flag"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn non_json_success_is_parse_error_and_empty_is_null() {
        assert!(matches!(normalize(Some(0), "hello table", ""), Err(CliError::Parse(_))));
        assert_eq!(normalize(Some(0), "  ", "").unwrap(), Value::Null);
    }

    #[test]
    fn sanitize_redacts_token_like_words() {
        let s = sanitize("bad token u-7f3kA9xQ2mP0zL8vN4cR1tY6wB5 here");
        assert!(s.contains("[redacted]"));
        assert!(!s.contains("u-7f3kA9"));
        assert!(s.contains("here"));
    }

    #[test]
    fn identity_flag_is_appended() {
        let r = Runner::new("lark-cli").with_identity(Some("bot".into()));
        let a = r.full_args(&["im".into(), "+chat-list".into()]);
        assert_eq!(a[a.len() - 2..], ["--as".to_string(), "bot".to_string()]);
        let b = r.full_args(&["auth".into(), "status".into()]);
        assert!(!b.contains(&"--as".to_string()));
        let c = r.full_args(&["im".into(), "--as".into(), "user".into()]);
        assert_eq!(c.iter().filter(|x| *x == "--as").count(), 1);
    }

    #[test]
    fn missing_binary_is_not_installed() {
        let r = Runner::new("/nonexistent/lark-cli-xyz");
        assert!(matches!(r.run(&["auth".into()]), Err(CliError::NotInstalled(_))));
    }
}
