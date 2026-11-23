//! Sign-in state detection via `lark-cli auth status --json`.

use serde_json::Value;

use crate::json::first_str;
use crate::runner::{CliError, Runner};

/// Commands the user runs in a terminal to set up lark-cli.
pub const SETUP_COMMANDS: [&str; 3] = [
    "npm i -g @larksuite/cli",
    "lark-cli config init --new",
    "lark-cli auth login --recommend",
];

#[derive(Debug, Clone, PartialEq)]
pub enum SignInState {
    /// Not checked yet.
    Unknown,
    /// The lark-cli executable could not be started.
    NotInstalled(String),
    /// lark-cli has no app configuration yet.
    NotConfigured { hint: String },
    /// Configured, but no usable identity.
    SignedOut { hint: String },
    /// Ready. `identity` is `user` or `bot`; `display` is a name if known.
    SignedIn { identity: String, display: String },
    /// Could not determine (e.g. unexpected failure); message is sanitized.
    Error(String),
}

impl SignInState {
    pub fn is_signed_in(&self) -> bool {
        matches!(self, SignInState::SignedIn { .. })
    }
}

pub fn status_args() -> Vec<String> {
    vec!["auth".into(), "status".into(), "--json".into()]
}

/// Interpret the (unwrapped) JSON of `auth status`.
pub fn parse_status(v: &Value) -> SignInState {
    let identity = first_str(v, &["identity", "effective_identity", "data.identity", "current.identity"]).to_lowercase();
    let display = first_str(
        v,
        &["user_name", "userName", "user.name", "user.en_name", "name", "data.user_name", "app_name", "app.name", "app_id"],
    );
    match identity.as_str() {
        "user" | "bot" => SignInState::SignedIn { identity, display },
        _ => SignInState::SignedOut {
            hint: {
                let h = first_str(v, &["hint", "message"]);
                if h.is_empty() { "Run `lark-cli auth login --recommend`.".into() } else { h }
            },
        },
    }
}

/// Map a runner result of `auth status` onto a state.
pub fn from_result(r: Result<Value, CliError>) -> SignInState {
    match r {
        Ok(v) => parse_status(&v),
        Err(CliError::NotInstalled(p)) => SignInState::NotInstalled(p),
        Err(CliError::NotConfigured { hint }) => SignInState::NotConfigured { hint },
        Err(CliError::NotSignedIn { hint }) => SignInState::SignedOut { hint },
        Err(e) => SignInState::Error(e.to_string()),
    }
}

pub fn check(runner: &Runner) -> SignInState {
    from_result(runner.run(&status_args()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::normalize;
    use serde_json::json;

    #[test]
    fn user_identity_is_signed_in() {
        let s = parse_status(&json!({"identity": "user", "user_name": "Ada", "app_id": "cli_x"}));
        assert_eq!(s, SignInState::SignedIn { identity: "user".into(), display: "Ada".into() });
    }

    #[test]
    fn none_identity_is_signed_out() {
        assert!(matches!(parse_status(&json!({"identity": "none"})), SignInState::SignedOut { .. }));
        assert!(matches!(parse_status(&json!({})), SignInState::SignedOut { .. }));
    }

    #[test]
    fn exit3_is_not_configured() {
        let err = r#"{"ok":false,"error":{"type":"config","hint":"run config init"}}"#;
        let st = from_result(normalize(Some(3), "", err));
        assert_eq!(st, SignInState::NotConfigured { hint: "run config init".into() });
    }

    #[test]
    fn envelope_is_unwrapped_first() {
        let st = from_result(normalize(Some(0), r#"{"ok":true,"data":{"identity":"bot","app_name":"My App"}}"#, ""));
        assert!(st.is_signed_in());
    }
}
