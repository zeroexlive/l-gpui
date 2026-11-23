//! Integration tests that drive the real `Runner` against a fake lark-cli
//! shell script (Unix only).
#![cfg(unix)]

use std::path::PathBuf;
use std::sync::Mutex;

use lgpui_core::domains::approval::Topic;
use lgpui_core::{CliError, Lark, Runner, SignInState};

// FAKE_LARK_* env vars are process-wide; serialise the tests.
static ENV: Mutex<()> = Mutex::new(());

fn fake() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-lark-cli.sh")
}

fn lark(mode: &str) -> Lark {
    // SAFETY: guarded by ENV mutex; tests in this file never run concurrently.
    unsafe { std::env::set_var("FAKE_LARK_MODE", mode) };
    Lark::new(Runner::new(fake()))
}

#[test]
fn full_happy_path() {
    let _g = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let l = lark("ok");
    assert_eq!(
        l.sign_in_state(),
        SignInState::SignedIn { identity: "user".into(), display: "Test User".into() }
    );
    let chats = l.chats(20).unwrap();
    assert_eq!(chats.len(), 2);
    assert_eq!(chats[0].name, "Design Team");

    let msgs = l.messages("oc_a", 50).unwrap();
    assert_eq!(msgs.first().unwrap().text, "Draft ready?");
    assert_eq!(msgs.last().unwrap().sender, "Ada");
    l.send_text("oc_a", "hello").unwrap();
    assert_eq!(l.search_messages("draft").unwrap()[0].chat_name, "Design Team");

    assert_eq!(l.agenda(7).unwrap()[0].summary, "Standup");
    let tasks = l.my_tasks().unwrap();
    assert_eq!(tasks[0].summary, "Ship l-gpui");
    l.complete_task("t1").unwrap();
    l.create_task("New").unwrap();

    assert_eq!(l.search_docs("roadmap").unwrap()[0].title, "Roadmap 2030");
    assert_eq!(l.wiki_spaces().unwrap()[0].name, "Engineering");

    let inbox = l.mail_triage(20, None).unwrap();
    assert!(inbox[0].unread);
    assert_eq!(l.mail_message("m1").unwrap().body, "All green.");
    l.mail_send("a@example.com", "Hi", "Body").unwrap();
    assert!(l.mail_send(" ", "Hi", "Body").is_err());

    assert_eq!(l.search_users("ada").unwrap()[0].email, "ada@example.com");
    assert_eq!(l.approvals(Topic::Pending).unwrap()[0].initiator, "Bob");
    assert_eq!(l.search_minutes("").unwrap()[0].title, "Weekly sync");
}

#[test]
fn unconfigured_cli_is_detected() {
    let _g = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let l = lark("unconfigured");
    assert!(matches!(l.sign_in_state(), SignInState::NotConfigured { .. }));
    let err = l.chats(10).unwrap_err();
    assert!(err.needs_sign_in());
}

#[test]
fn signed_out_identity_is_detected() {
    let _g = ENV.lock().unwrap_or_else(|e| e.into_inner());
    assert!(matches!(lark("signed_out").sign_in_state(), SignInState::SignedOut { .. }));
}

#[test]
fn api_errors_surface_message() {
    let _g = ENV.lock().unwrap_or_else(|e| e.into_inner());
    match lark("api_error").my_tasks() {
        Err(CliError::Api { message, .. }) => assert!(message.contains("permission denied")),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn identity_env_is_forwarded() {
    let _g = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let log = std::env::temp_dir().join(format!("fake-lark-{}.log", std::process::id()));
    let _ = std::fs::remove_file(&log);
    unsafe {
        std::env::set_var("FAKE_LARK_MODE", "ok");
        std::env::set_var("FAKE_LARK_LOG", &log);
    }
    let l = Lark::new(Runner::new(fake()).with_identity(Some("bot".into())));
    l.chats(5).unwrap();
    l.sign_in_state();
    unsafe { std::env::remove_var("FAKE_LARK_LOG") };
    let logged = std::fs::read_to_string(&log).unwrap();
    let _ = std::fs::remove_file(&log);
    let lines: Vec<&str> = logged.lines().collect();
    assert!(lines[0].starts_with("im +chat-list") && lines[0].ends_with("--as bot"), "{}", lines[0]);
    assert!(!lines[1].contains("--as"));
}
