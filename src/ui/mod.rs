//! GPUI front-end. State and actions live here; rendering is in `views`.
//!
//! Every lark-cli call runs on GPUI's background executor via
//! [`LarkApp::spawn_cli`], so the UI thread never blocks on a subprocess.

mod theme;
mod views;
mod widgets;

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use gpui::{
    App, Application, Bounds, ClipboardItem, Context, FocusHandle, KeyDownEvent, ScrollHandle, Window,
    WindowBounds, WindowOptions, prelude::*, px, size,
};

use lgpui_core::domains::approval::{ApprovalTask, Topic};
use lgpui_core::domains::calendar::Event;
use lgpui_core::domains::contact::User;
use lgpui_core::domains::docs::{DocHit, WikiSpace};
use lgpui_core::domains::im::{Chat, Message};
use lgpui_core::domains::mail::{MailMessage, MailSummary};
use lgpui_core::domains::minutes::Minute;
use lgpui_core::domains::task::Task;
use lgpui_core::{CliError, Lark, SignInState};

const AUTH_RECHECK: Duration = Duration::from_secs(4);
const MESSAGE_POLL: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum View {
    Messenger,
    Calendar,
    Tasks,
    Docs,
    Mail,
    Contacts,
    Approvals,
    Minutes,
    Meetings,
    Okr,
    Base,
    Attendance,
}

impl View {
    pub const ALL: [View; 12] = [
        View::Messenger,
        View::Calendar,
        View::Tasks,
        View::Docs,
        View::Mail,
        View::Contacts,
        View::Approvals,
        View::Minutes,
        View::Meetings,
        View::Okr,
        View::Base,
        View::Attendance,
    ];

    pub fn label(self) -> &'static str {
        match self {
            View::Messenger => "Messenger",
            View::Calendar => "Calendar",
            View::Tasks => "Tasks",
            View::Docs => "Docs",
            View::Mail => "Mail",
            View::Contacts => "Contacts",
            View::Approvals => "Approval",
            View::Minutes => "Minutes",
            View::Meetings => "Meetings",
            View::Okr => "OKR",
            View::Base => "Base",
            View::Attendance => "Attendance",
        }
    }

    /// One- or two-letter glyph shown in the rail (no bundled icon assets).
    pub fn glyph(self) -> &'static str {
        match self {
            View::Messenger => "M",
            View::Calendar => "Ca",
            View::Tasks => "T",
            View::Docs => "D",
            View::Mail => "@",
            View::Contacts => "Co",
            View::Approvals => "Ap",
            View::Minutes => "Mi",
            View::Meetings => "Me",
            View::Okr => "O",
            View::Base => "B",
            View::Attendance => "At",
        }
    }

    pub fn is_stub(self) -> bool {
        matches!(self, View::Meetings | View::Okr | View::Base | View::Attendance)
    }
}

/// Editable text fields. Only one has keyboard focus at a time.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Field {
    ChatFilter,
    Composer,
    MsgSearch,
    NewTask,
    DocsQuery,
    MailQuery,
    MailTo,
    MailSubject,
    MailBody,
    ContactsQuery,
    MinutesQuery,
}

impl Field {
    fn multiline(self) -> bool {
        matches!(self, Field::MailBody)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Idle,
    Info(String),
    Error(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MailPane {
    Inbox,
    Compose,
}

pub struct LarkApp {
    pub(crate) lark: Lark,
    pub(crate) focus: FocusHandle,
    pub(crate) auth: SignInState,
    pub(crate) checking_auth: bool,
    pub(crate) view: View,
    pub(crate) loaded: HashSet<View>,
    pub(crate) busy: usize,
    pub(crate) status: Status,
    pub(crate) fields: HashMap<Field, String>,
    pub(crate) active: Option<Field>,

    pub(crate) chats: Vec<Chat>,
    pub(crate) selected_chat: Option<String>,
    pub(crate) messages: Vec<Message>,
    pub(crate) msg_results: Option<Vec<Message>>,
    pub(crate) msg_scroll: ScrollHandle,

    pub(crate) events: Vec<Event>,
    pub(crate) agenda_days: i64,
    pub(crate) tasks: Vec<Task>,
    pub(crate) docs: Vec<DocHit>,
    pub(crate) spaces: Vec<WikiSpace>,
    pub(crate) mails: Vec<MailSummary>,
    pub(crate) mail_open: Option<MailMessage>,
    pub(crate) mail_pane: MailPane,
    pub(crate) users: Vec<User>,
    pub(crate) approvals: Vec<ApprovalTask>,
    pub(crate) approval_topic: Topic,
    pub(crate) minutes: Vec<Minute>,
}

impl LarkApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus);
        let mut this = Self {
            lark: Lark::from_env(),
            focus,
            auth: SignInState::Unknown,
            checking_auth: false,
            view: View::Messenger,
            loaded: HashSet::new(),
            busy: 0,
            status: Status::Idle,
            fields: HashMap::new(),
            active: None,
            chats: Vec::new(),
            selected_chat: None,
            messages: Vec::new(),
            msg_results: None,
            msg_scroll: ScrollHandle::new(),
            events: Vec::new(),
            agenda_days: 7,
            tasks: Vec::new(),
            docs: Vec::new(),
            spaces: Vec::new(),
            mails: Vec::new(),
            mail_open: None,
            mail_pane: MailPane::Inbox,
            users: Vec::new(),
            approvals: Vec::new(),
            approval_topic: Topic::Pending,
            minutes: Vec::new(),
        };
        this.check_auth(cx);
        this.start_timers(cx);
        this
    }

    // ---------------------------------------------------------------- helpers

    pub(crate) fn field(&self, f: Field) -> &str {
        self.fields.get(&f).map(String::as_str).unwrap_or("")
    }

    fn take_field(&mut self, f: Field) -> String {
        self.fields.remove(&f).unwrap_or_default()
    }

    /// Run `job` with a clone of the client on the background executor, then
    /// apply `done` on the UI thread.
    fn spawn_cli<T, J, D>(&mut self, cx: &mut Context<Self>, job: J, done: D)
    where
        T: Send + 'static,
        J: FnOnce(Lark) -> T + Send + 'static,
        D: FnOnce(&mut Self, T, &mut Context<Self>) + 'static,
    {
        let lark = self.lark.clone();
        self.busy += 1;
        cx.spawn(async move |this, cx| {
            let out = cx.background_executor().spawn(async move { job(lark) }).await;
            this.update(cx, |this, cx| {
                this.busy = this.busy.saturating_sub(1);
                done(this, out, cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Route an error: sign-in problems switch to the sign-in screen.
    fn fail(&mut self, e: CliError) {
        if e.needs_sign_in() {
            self.auth = lgpui_core::auth::from_result(Err(e));
            self.loaded.clear();
        } else {
            self.status = Status::Error(e.to_string());
        }
    }

    fn ok_or_fail<T>(&mut self, r: Result<T, CliError>) -> Option<T> {
        match r {
            Ok(v) => {
                if matches!(self.status, Status::Error(_)) {
                    self.status = Status::Idle;
                }
                Some(v)
            }
            Err(e) => {
                self.fail(e);
                None
            }
        }
    }

    // ------------------------------------------------------------------ auth

    pub(crate) fn check_auth(&mut self, cx: &mut Context<Self>) {
        if self.checking_auth {
            return;
        }
        self.checking_auth = true;
        self.spawn_cli(cx, |l| l.sign_in_state(), |this, state, cx| {
            this.checking_auth = false;
            let was_in = this.auth.is_signed_in();
            this.auth = state;
            if this.auth.is_signed_in() && !was_in {
                this.status = Status::Idle;
                this.loaded.clear();
                this.ensure_loaded(cx);
            }
        });
    }

    fn start_timers(&self, cx: &mut Context<Self>) {
        // Re-check sign-in while the sign-in screen is showing.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(AUTH_RECHECK).await;
                let ok = this.update(cx, |this, cx| {
                    if !this.auth.is_signed_in() {
                        this.check_auth(cx);
                    }
                });
                if ok.is_err() {
                    break;
                }
            }
        })
        .detach();
        // Refresh the open conversation.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(MESSAGE_POLL).await;
                let ok = this.update(cx, |this, cx| {
                    if this.auth.is_signed_in() && this.view == View::Messenger && this.busy == 0 {
                        this.load_messages(cx, false);
                    }
                });
                if ok.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    // ------------------------------------------------------------ navigation

    pub(crate) fn navigate(&mut self, view: View, cx: &mut Context<Self>) {
        self.view = view;
        self.active = None;
        self.status = Status::Idle;
        self.ensure_loaded(cx);
        cx.notify();
    }

    fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        if !self.auth.is_signed_in() || self.loaded.contains(&self.view) {
            return;
        }
        self.loaded.insert(self.view);
        self.refresh(cx);
    }

    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        match self.view {
            View::Messenger => self.load_chats(cx),
            View::Calendar => self.load_agenda(cx),
            View::Tasks => self.load_tasks(cx),
            View::Docs => self.load_spaces(cx),
            View::Mail => self.load_mail(cx),
            View::Approvals => self.load_approvals(cx),
            View::Minutes => self.search_minutes(cx),
            View::Contacts => self.search_contacts(cx),
            _ => {}
        }
    }

    // ------------------------------------------------------------- messenger

    fn load_chats(&mut self, cx: &mut Context<Self>) {
        self.spawn_cli(cx, |l| l.chats(50), |this, r, cx| {
            if let Some(chats) = this.ok_or_fail(r) {
                this.chats = chats;
                if this.selected_chat.is_none() {
                    if let Some(first) = this.chats.first() {
                        this.selected_chat = Some(first.chat_id.clone());
                        this.load_messages(cx, true);
                    }
                }
            }
        });
    }

    pub(crate) fn select_chat(&mut self, chat_id: String, cx: &mut Context<Self>) {
        if self.selected_chat.as_deref() != Some(chat_id.as_str()) {
            self.selected_chat = Some(chat_id);
            self.messages.clear();
            self.msg_results = None;
            self.load_messages(cx, true);
        }
        self.active = Some(Field::Composer);
        cx.notify();
    }

    fn load_messages(&mut self, cx: &mut Context<Self>, scroll: bool) {
        let Some(chat_id) = self.selected_chat.clone() else { return };
        let id = chat_id.clone();
        self.spawn_cli(cx, move |l| l.messages(&id, 50), move |this, r, _| {
            if this.selected_chat.as_deref() != Some(chat_id.as_str()) {
                return;
            }
            if let Some(msgs) = this.ok_or_fail(r) {
                let grew = msgs.len() != this.messages.len()
                    || msgs.last().map(|m| &m.message_id) != this.messages.last().map(|m| &m.message_id);
                this.messages = msgs;
                if scroll || grew {
                    this.msg_scroll.scroll_to_bottom();
                }
            }
        });
    }

    fn send_message(&mut self, cx: &mut Context<Self>) {
        let Some(chat_id) = self.selected_chat.clone() else { return };
        let text = self.take_field(Field::Composer);
        if text.trim().is_empty() {
            return;
        }
        self.status = Status::Info("Sending…".into());
        let id = chat_id.clone();
        self.spawn_cli(cx, move |l| l.send_text(&id, &text), |this, r, cx| {
            if this.ok_or_fail(r).is_some() {
                this.status = Status::Idle;
                this.load_messages(cx, true);
            }
        });
    }

    fn search_messages(&mut self, cx: &mut Context<Self>) {
        let q = self.field(Field::MsgSearch).trim().to_string();
        if q.is_empty() {
            self.msg_results = None;
            cx.notify();
            return;
        }
        self.spawn_cli(cx, move |l| l.search_messages(&q), |this, r, _| {
            if let Some(m) = this.ok_or_fail(r) {
                this.msg_results = Some(m);
            }
        });
    }

    pub(crate) fn clear_message_search(&mut self, cx: &mut Context<Self>) {
        self.msg_results = None;
        self.fields.remove(&Field::MsgSearch);
        cx.notify();
    }

    // --------------------------------------------------------------- calendar

    pub(crate) fn set_agenda_days(&mut self, days: i64, cx: &mut Context<Self>) {
        self.agenda_days = days;
        self.load_agenda(cx);
    }

    fn load_agenda(&mut self, cx: &mut Context<Self>) {
        let days = self.agenda_days;
        self.spawn_cli(cx, move |l| l.agenda(days), |this, r, _| {
            if let Some(e) = this.ok_or_fail(r) {
                this.events = e;
            }
        });
    }

    // ------------------------------------------------------------------ tasks

    fn load_tasks(&mut self, cx: &mut Context<Self>) {
        self.spawn_cli(cx, |l| l.my_tasks(), |this, r, _| {
            if let Some(t) = this.ok_or_fail(r) {
                this.tasks = t;
            }
        });
    }

    pub(crate) fn complete_task(&mut self, guid: String, cx: &mut Context<Self>) {
        self.spawn_cli(cx, move |l| l.complete_task(&guid).map(|_| guid), |this, r, cx| {
            if let Some(guid) = this.ok_or_fail(r) {
                this.tasks.retain(|t| t.guid != guid);
                this.status = Status::Info("Task completed".into());
                this.load_tasks(cx);
            }
        });
    }

    fn create_task(&mut self, cx: &mut Context<Self>) {
        let summary = self.take_field(Field::NewTask);
        if summary.trim().is_empty() {
            return;
        }
        self.spawn_cli(cx, move |l| l.create_task(summary.trim()), |this, r, cx| {
            if this.ok_or_fail(r).is_some() {
                this.status = Status::Info("Task created".into());
                this.load_tasks(cx);
            }
        });
    }

    // ------------------------------------------------------------------- docs

    fn load_spaces(&mut self, cx: &mut Context<Self>) {
        self.spawn_cli(cx, |l| l.wiki_spaces(), |this, r, _| {
            if let Some(s) = this.ok_or_fail(r) {
                this.spaces = s;
            }
        });
    }

    fn search_docs(&mut self, cx: &mut Context<Self>) {
        let q = self.field(Field::DocsQuery).trim().to_string();
        if q.is_empty() {
            return;
        }
        self.spawn_cli(cx, move |l| l.search_docs(&q), |this, r, _| {
            if let Some(d) = this.ok_or_fail(r) {
                if d.is_empty() {
                    this.status = Status::Info("No documents matched".into());
                }
                this.docs = d;
            }
        });
    }

    // ------------------------------------------------------------------- mail

    fn load_mail(&mut self, cx: &mut Context<Self>) {
        let q = self.field(Field::MailQuery).trim().to_string();
        self.spawn_cli(cx, move |l| l.mail_triage(40, Some(&q)), |this, r, _| {
            if let Some(m) = this.ok_or_fail(r) {
                this.mails = m;
            }
        });
    }

    pub(crate) fn open_mail(&mut self, id: String, cx: &mut Context<Self>) {
        self.mail_pane = MailPane::Inbox;
        self.spawn_cli(cx, move |l| l.mail_message(&id), |this, r, _| {
            if let Some(m) = this.ok_or_fail(r) {
                this.mail_open = Some(m);
            }
        });
    }

    pub(crate) fn compose_mail(&mut self, reply: bool, cx: &mut Context<Self>) {
        if reply {
            if let Some(m) = &self.mail_open {
                let to = m.from.clone();
                let subject = if m.subject.to_lowercase().starts_with("re:") {
                    m.subject.clone()
                } else {
                    format!("Re: {}", m.subject)
                };
                self.fields.insert(Field::MailTo, to);
                self.fields.insert(Field::MailSubject, subject);
            }
        }
        self.mail_pane = MailPane::Compose;
        self.active = Some(if reply { Field::MailBody } else { Field::MailTo });
        cx.notify();
    }

    pub(crate) fn send_mail(&mut self, cx: &mut Context<Self>) {
        let to = self.field(Field::MailTo).to_string();
        let subject = self.field(Field::MailSubject).to_string();
        let body = self.field(Field::MailBody).to_string();
        if to.trim().is_empty() {
            self.status = Status::Error("Add at least one recipient".into());
            cx.notify();
            return;
        }
        self.status = Status::Info("Sending mail…".into());
        self.spawn_cli(cx, move |l| l.mail_send(&to, &subject, &body), |this, r, _| {
            if this.ok_or_fail(r).is_some() {
                for f in [Field::MailTo, Field::MailSubject, Field::MailBody] {
                    this.fields.remove(&f);
                }
                this.mail_pane = MailPane::Inbox;
                this.active = None;
                this.status = Status::Info("Mail sent".into());
            }
        });
    }

    // --------------------------------------------------- contacts / approvals

    fn search_contacts(&mut self, cx: &mut Context<Self>) {
        let q = self.field(Field::ContactsQuery).trim().to_string();
        if q.is_empty() {
            return;
        }
        self.spawn_cli(cx, move |l| l.search_users(&q), |this, r, _| {
            if let Some(u) = this.ok_or_fail(r) {
                if u.is_empty() {
                    this.status = Status::Info("No people matched".into());
                }
                this.users = u;
            }
        });
    }

    pub(crate) fn set_approval_topic(&mut self, t: Topic, cx: &mut Context<Self>) {
        self.approval_topic = t;
        self.load_approvals(cx);
    }

    fn load_approvals(&mut self, cx: &mut Context<Self>) {
        let t = self.approval_topic;
        self.spawn_cli(cx, move |l| l.approvals(t), |this, r, _| {
            if let Some(a) = this.ok_or_fail(r) {
                this.approvals = a;
            }
        });
    }

    fn search_minutes(&mut self, cx: &mut Context<Self>) {
        let q = self.field(Field::MinutesQuery).trim().to_string();
        self.spawn_cli(cx, move |l| l.search_minutes(&q), |this, r, _| {
            if let Some(m) = this.ok_or_fail(r) {
                this.minutes = m;
            }
        });
    }

    // ------------------------------------------------------------------ input

    pub(crate) fn activate(&mut self, f: Field, window: &mut Window, cx: &mut Context<Self>) {
        self.active = Some(f);
        window.focus(&self.focus);
        cx.notify();
    }

    fn submit(&mut self, f: Field, cx: &mut Context<Self>) {
        match f {
            Field::Composer => self.send_message(cx),
            Field::MsgSearch => self.search_messages(cx),
            Field::ChatFilter => {}
            Field::NewTask => self.create_task(cx),
            Field::DocsQuery => self.search_docs(cx),
            Field::MailQuery => self.load_mail(cx),
            Field::MailTo => self.active = Some(Field::MailSubject),
            Field::MailSubject => self.active = Some(Field::MailBody),
            Field::MailBody => self.send_mail(cx),
            Field::ContactsQuery => self.search_contacts(cx),
            Field::MinutesQuery => self.search_minutes(cx),
        }
    }

    pub(crate) fn copy(&mut self, text: &str, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
        self.status = Status::Info(format!("Copied: {text}"));
        cx.notify();
    }

    fn on_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = ks.modifiers;
        let command = m.control || m.platform;
        if command && ks.key == "r" {
            self.refresh(cx);
            cx.stop_propagation();
            return;
        }
        let Some(f) = self.active else { return };
        match ks.key.as_str() {
            "enter" if m.shift || (f.multiline() && !command) => self.fields.entry(f).or_default().push('\n'),
            "enter" => self.submit(f, cx),
            "escape" => self.active = None,
            "tab" if matches!(f, Field::MailTo | Field::MailSubject | Field::MailBody) => {
                self.active = Some(match (f, m.shift) {
                    (Field::MailTo, false) | (Field::MailBody, true) => Field::MailSubject,
                    (Field::MailSubject, false) => Field::MailBody,
                    (Field::MailSubject, true) => Field::MailTo,
                    _ => Field::MailTo,
                });
            }
            "backspace" => {
                let s = self.fields.entry(f).or_default();
                if command || m.alt {
                    let trimmed = s.trim_end_matches(|c: char| !c.is_whitespace()).len();
                    let base = if trimmed == s.len() { s.trim_end().len() } else { trimmed };
                    s.truncate(base);
                } else {
                    s.pop();
                }
            }
            "v" if command => {
                if let Some(text) = cx.read_from_clipboard().and_then(|c| c.text()) {
                    let text = if f.multiline() { text } else { text.replace(['\n', '\r'], " ") };
                    self.fields.entry(f).or_default().push_str(&text);
                }
            }
            _ if !command => {
                if let Some(ch) = ks.key_char.as_deref() {
                    let clean: String = ch.chars().filter(|c| !c.is_control()).collect();
                    if clean.is_empty() {
                        return;
                    }
                    self.fields.entry(f).or_default().push_str(&clean);
                } else {
                    return;
                }
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }
}

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.), px(780.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions { title: Some("l-gpui — Lark".into()), ..Default::default() }),
                app_id: Some("l-gpui".into()),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| LarkApp::new(window, cx)),
        )
        .expect("failed to open window");
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
}
