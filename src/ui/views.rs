//! Rendering for every screen. Pure GPUI primitives (div, text, flex).

use gpui::{
    AnyElement, Context, Div, FontWeight, MouseButton, SharedString, Window, div, prelude::*, px, rgb,
};

use lgpui_core::SignInState;
use lgpui_core::auth::SETUP_COMMANDS;
use lgpui_core::domains::approval::Topic;
use lgpui_core::time;

use super::widgets::{avatar, button, code_line, empty, header, pill};
use super::{Field, LarkApp, MailPane, Status, View, theme};

fn sid(s: impl Into<String>) -> SharedString {
    SharedString::from(s.into())
}

impl LarkApp {
    // ------------------------------------------------------------ text input

    fn input(&self, f: Field, placeholder: &str, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let focused = self.active == Some(f);
        let value = self.field(f).to_string();
        let is_empty = value.is_empty();
        let text = if is_empty && !focused {
            placeholder.to_string()
        } else if focused {
            format!("{value}▏")
        } else {
            value
        };
        div()
            .id(sid(format!("input-{f:?}")))
            .px_3()
            .py_1()
            .min_h(px(32.))
            .rounded_md()
            .border_1()
            .border_color(rgb(if focused { theme::ACCENT } else { theme::BORDER }))
            .bg(rgb(theme::MAIN_BG))
            .text_sm()
            .text_color(rgb(if is_empty && !focused { theme::MUTED } else { theme::TEXT }))
            .cursor_text()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| this.activate(f, window, cx)))
            .child(text)
    }

    fn toolbar(&self, title: &str, subtitle: String, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .items_center()
            .gap_3()
            .px_6()
            .py_4()
            .border_b_1()
            .border_color(rgb(theme::BORDER))
            .child(header(title.to_string(), subtitle))
            .child(button("refresh", "Refresh", false).on_click(cx.listener(|this, _, _, cx| this.refresh(cx))))
    }

    fn status_bar(&self) -> Div {
        let (text, color) = match &self.status {
            Status::Idle if self.busy > 0 => ("Working…".to_string(), theme::MUTED),
            Status::Idle => (String::new(), theme::MUTED),
            Status::Info(s) => (s.clone(), theme::TEXT_2),
            Status::Error(e) => (e.clone(), theme::ERROR),
        };
        let who = match &self.auth {
            SignInState::SignedIn { identity, display } if !display.is_empty() => format!("{display} · {identity}"),
            SignInState::SignedIn { identity, .. } => format!("signed in as {identity}"),
            _ => String::new(),
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(26.))
            .px_4()
            .border_t_1()
            .border_color(rgb(theme::BORDER))
            .bg(rgb(theme::PANEL_BG))
            .text_xs()
            .child(div().text_color(rgb(color)).truncate().child(text))
            .child(div().text_color(rgb(theme::MUTED)).child(who))
    }

    // ------------------------------------------------------------------ rail

    fn rail(&self, cx: &mut Context<Self>) -> Div {
        let items: Vec<AnyElement> = View::ALL
            .iter()
            .map(|&v| {
                let active = self.view == v;
                div()
                    .id(sid(format!("nav-{v:?}")))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .py_2()
                    .w_full()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| this.navigate(v, cx)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(32.))
                            .rounded_lg()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .when(active, |d| d.bg(rgb(theme::ACCENT)).text_color(rgb(theme::WHITE)))
                            .when(!active, |d| {
                                d.text_color(rgb(if v.is_stub() { theme::MUTED } else { theme::TEXT_2 }))
                                    .hover(|s| s.bg(rgb(theme::BORDER)))
                            })
                            .child(v.glyph()),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(if active { theme::ACCENT } else { theme::TEXT_2 }))
                            .child(v.label()),
                    )
                    .into_any_element()
            })
            .collect();
        let me = match &self.auth {
            SignInState::SignedIn { display, identity } => {
                if display.is_empty() { identity.clone() } else { display.clone() }
            }
            _ => "?".into(),
        };
        div()
            .flex()
            .flex_col()
            .items_center()
            .w(px(72.))
            .h_full()
            .bg(rgb(theme::RAIL_BG))
            .border_r_1()
            .border_color(rgb(theme::BORDER))
            .pt_3()
            .child(avatar(&me, 36.).mb_3())
            .child(div().id("rail-items").flex().flex_col().w_full().flex_1().overflow_y_scroll().children(items))
    }

    // --------------------------------------------------------------- sign in

    fn sign_in(&self, cx: &mut Context<Self>) -> Div {
        let (title, detail) = match &self.auth {
            SignInState::Unknown => ("Checking lark-cli…".to_string(), String::new()),
            SignInState::NotInstalled(p) => (
                "lark-cli is not installed".to_string(),
                format!("Could not start `{p}`. Install Node.js, then run the first command below. Set LARK_CLI to use a custom path."),
            ),
            SignInState::NotConfigured { hint } => ("Sign in with lark-cli".to_string(), hint.clone()),
            SignInState::SignedOut { hint } => ("Sign in with lark-cli".to_string(), hint.clone()),
            SignInState::Error(e) => ("Could not check sign-in".to_string(), e.clone()),
            SignInState::SignedIn { .. } => (String::new(), String::new()),
        };
        let steps: Vec<AnyElement> = SETUP_COMMANDS
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                let c = cmd.to_string();
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().w(px(18.)).text_sm().text_color(rgb(theme::MUTED)).child(format!("{}.", i + 1)))
                    .child(code_line(cmd.to_string()))
                    .child(button(sid(format!("copy-{i}")), "Copy", false).on_click(cx.listener(move |this, _, _, cx| this.copy(&c, cx))))
                    .into_any_element()
            })
            .collect();
        div()
            .flex()
            .flex_1()
            .items_center()
            .justify_center()
            .bg(rgb(theme::PANEL_BG))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .w(px(620.))
                    .p_8()
                    .rounded_xl()
                    .bg(rgb(theme::MAIN_BG))
                    .border_1()
                    .border_color(rgb(theme::BORDER))
                    .child(div().flex().items_center().gap_3().child(avatar("L G", 40.)).child(header(title, "l-gpui uses the official lark-cli for sign-in and data.")))
                    .when(!detail.is_empty(), |d| d.child(div().text_sm().text_color(rgb(theme::TEXT_2)).child(detail)))
                    .child(div().text_sm().text_color(rgb(theme::TEXT)).child("Run these in a terminal. `config init` and `auth login` print a link to open in your browser:"))
                    .child(div().flex().flex_col().gap_2().children(steps))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(button("recheck", if self.checking_auth { "Checking…" } else { "Check again" }, true).on_click(cx.listener(|this, _, _, cx| this.check_auth(cx))))
                            .child(div().text_xs().text_color(rgb(theme::MUTED)).child("This screen re-checks automatically every few seconds.")),
                    ),
            )
    }

    // ------------------------------------------------------------- messenger

    fn messenger(&self, cx: &mut Context<Self>) -> Div {
        let filter = self.field(Field::ChatFilter).to_lowercase();
        let rows: Vec<AnyElement> = self
            .chats
            .iter()
            .filter(|c| filter.is_empty() || c.display_name().to_lowercase().contains(&filter))
            .map(|c| {
                let selected = self.selected_chat.as_deref() == Some(c.chat_id.as_str());
                let id = c.chat_id.clone();
                let kind = if c.mode == "p2p" { "Direct" } else if c.mode.is_empty() { "" } else { "Group" };
                div()
                    .id(sid(format!("chat-{}", c.chat_id)))
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .mx_2()
                    .rounded_md()
                    .cursor_pointer()
                    .when(selected, |d| d.bg(rgb(theme::ACCENT_SOFT)))
                    .when(!selected, |d| d.hover(|s| s.bg(rgb(theme::HOVER))))
                    .on_click(cx.listener(move |this, _, _, cx| this.select_chat(id.clone(), cx)))
                    .child(avatar(c.display_name(), 36.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .min_w(px(0.))
                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).text_color(rgb(theme::TEXT)).truncate().child(c.display_name().to_string()))
                            .child(div().text_xs().text_color(rgb(theme::MUTED)).truncate().child(kind)),
                    )
                    .into_any_element()
            })
            .collect();
        let list = div()
            .flex()
            .flex_col()
            .w(px(280.))
            .h_full()
            .bg(rgb(theme::PANEL_BG))
            .border_r_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .pt_4()
                    .pb_2()
                    .child(div().text_lg().font_weight(FontWeight::BOLD).child("Messenger"))
                    .child(button("chats-refresh", "↻", false).on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))),
            )
            .child(div().px_3().pb_2().child(self.input(Field::ChatFilter, "Filter chats", cx)))
            .child(div().px_3().pb_2().flex().gap_2().child(div().flex_1().child(self.input(Field::MsgSearch, "Search messages (Enter)", cx))))
            .child(
                div()
                    .id("chat-list")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .when(rows.is_empty(), |d| d.child(empty(if self.busy > 0 { "Loading chats…" } else { "No chats" })))
                    .children(rows),
            );
        div().flex().flex_1().h_full().child(list).child(self.conversation(cx))
    }

    fn conversation(&self, cx: &mut Context<Self>) -> Div {
        if let Some(results) = &self.msg_results {
            let rows: Vec<AnyElement> = results
                .iter()
                .map(|m| {
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .px_6()
                        .py_3()
                        .border_b_1()
                        .border_color(rgb(theme::BORDER))
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .text_xs()
                                .text_color(rgb(theme::MUTED))
                                .child(if m.chat_name.is_empty() { m.chat_id.clone() } else { m.chat_name.clone() })
                                .child("·")
                                .child(m.sender.clone())
                                .child(time::display(&m.create_time)),
                        )
                        .child(div().text_sm().text_color(rgb(theme::TEXT)).child(m.text.clone()))
                        .into_any_element()
                })
                .collect();
            return div()
                .flex()
                .flex_col()
                .flex_1()
                .h_full()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .px_6()
                        .py_4()
                        .border_b_1()
                        .border_color(rgb(theme::BORDER))
                        .child(header("Search results", format!("{} message(s) for “{}”", results.len(), self.field(Field::MsgSearch))))
                        .child(button("close-search", "Close", false).on_click(cx.listener(|this, _, _, cx| this.clear_message_search(cx)))),
                )
                .child(
                    div()
                        .id("search-results")
                        .flex()
                        .flex_col()
                        .flex_1()
                        .overflow_y_scroll()
                        .when(rows.is_empty(), |d| d.child(empty("No messages matched")))
                        .children(rows),
                );
        }

        let Some(chat) = self.selected_chat.as_ref().and_then(|id| self.chats.iter().find(|c| &c.chat_id == id)) else {
            return div().flex().flex_1().child(empty("Select a chat"));
        };
        let me = match &self.auth {
            SignInState::SignedIn { display, .. } => display.clone(),
            _ => String::new(),
        };
        let bubbles: Vec<AnyElement> = self
            .messages
            .iter()
            .map(|m| {
                let mine = !me.is_empty() && m.sender == me;
                let sender = if m.sender.is_empty() { "Unknown".to_string() } else { m.sender.clone() };
                div()
                    .flex()
                    .gap_2()
                    .when(mine, |d| d.flex_row_reverse())
                    .child(avatar(&sender, 32.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .when(mine, |d| d.items_end())
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .text_xs()
                                    .text_color(rgb(theme::MUTED))
                                    .child(sender)
                                    .child(time::display(&m.create_time)),
                            )
                            .child(
                                div()
                                    .max_w(px(560.))
                                    .px_3()
                                    .py_2()
                                    .rounded_lg()
                                    .bg(rgb(if mine { theme::ACCENT_SOFT } else { theme::BUBBLE_OTHER }))
                                    .text_sm()
                                    .text_color(rgb(theme::TEXT))
                                    .child(m.text.clone()),
                            ),
                    )
                    .into_any_element()
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_6()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(theme::BORDER))
                    .child(avatar(chat.display_name(), 32.))
                    .child(header(chat.display_name().to_string(), chat.description.clone())),
            )
            .child(
                div()
                    .id("messages")
                    .track_scroll(&self.msg_scroll)
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_4()
                    .px_6()
                    .py_4()
                    .overflow_y_scroll()
                    .when(bubbles.is_empty(), |d| d.child(empty(if self.busy > 0 { "Loading…" } else { "No messages yet" })))
                    .children(bubbles),
            )
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap_2()
                    .px_4()
                    .pb_4()
                    .child(div().flex_1().child(self.input(Field::Composer, &format!("Message {}  (Enter to send, Shift+Enter for newline)", chat.display_name()), cx).min_h(px(44.))))
                    .child(button("send", "Send", true).on_click(cx.listener(|this, _, _, cx| this.send_message(cx)))),
            )
    }

    // --------------------------------------------------------------- calendar

    fn calendar(&self, cx: &mut Context<Self>) -> Div {
        let mut rows: Vec<AnyElement> = Vec::new();
        let mut last_day = String::new();
        for e in &self.events {
            let day = time::parse(&e.start)
                .map(|d| d.format("%A, %B %-d").to_string())
                .unwrap_or_else(|| e.start.chars().take(10).collect());
            if day != last_day {
                rows.push(div().px_6().pt_4().pb_1().text_sm().font_weight(FontWeight::BOLD).text_color(rgb(theme::TEXT)).child(day.clone()).into_any_element());
                last_day = day;
            }
            let when = if e.all_day {
                "All day".to_string()
            } else {
                let fmt = |s: &str| time::parse(s).map(|d| d.format("%H:%M").to_string()).unwrap_or_else(|| s.to_string());
                format!("{} – {}", fmt(&e.start), fmt(&e.end))
            };
            rows.push(
                div()
                    .flex()
                    .gap_3()
                    .mx_6()
                    .my_1()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(theme::PANEL_BG))
                    .child(div().w(px(4.)).rounded_full().bg(rgb(theme::ACCENT)))
                    .child(div().w(px(110.)).text_sm().text_color(rgb(theme::TEXT_2)).child(when))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).text_color(rgb(theme::TEXT)).child(e.summary.clone()))
                            .when(!e.location.is_empty(), |d| d.child(div().text_xs().text_color(rgb(theme::MUTED)).child(e.location.clone())))
                            .when(!e.organizer.is_empty(), |d| d.child(div().text_xs().text_color(rgb(theme::MUTED)).child(format!("Organizer: {}", e.organizer)))),
                    )
                    .into_any_element(),
            );
        }
        let tb = self.toolbar("Calendar", format!("Agenda for the next {} day(s)", self.agenda_days), cx);
        let range = |d: i64, label: &'static str| {
            button(sid(format!("days-{d}")), label, self.agenda_days == d).on_click(cx.listener(move |this, _, _, cx| this.set_agenda_days(d, cx)))
        };
        div()
            .flex()
            .flex_col()
            .flex_1()
            .child(
                tb.child(range(1, "Today"))
                    .child(range(7, "7 days"))
                    .child(range(30, "30 days")),
            )
            .child(
                div()
                    .id("agenda")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .pb_4()
                    .overflow_y_scroll()
                    .when(rows.is_empty(), |d| d.child(empty(if self.busy > 0 { "Loading agenda…" } else { "Nothing scheduled" })))
                    .children(rows),
            )
    }

    // ------------------------------------------------------------------ tasks

    fn tasks_view(&self, cx: &mut Context<Self>) -> Div {
        let rows: Vec<AnyElement> = self
            .tasks
            .iter()
            .map(|t| {
                let guid = t.guid.clone();
                let url = t.url.clone();
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_6()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(theme::BORDER))
                    .child(
                        div()
                            .id(sid(format!("done-{}", t.guid)))
                            .size(px(18.))
                            .rounded_full()
                            .border_2()
                            .border_color(rgb(theme::MUTED))
                            .cursor_pointer()
                            .hover(|s| s.border_color(rgb(theme::SUCCESS)).bg(rgb(0xe8f8e6)))
                            .on_click(cx.listener(move |this, _, _, cx| this.complete_task(guid.clone(), cx))),
                    )
                    .child(div().flex_1().text_sm().text_color(rgb(theme::TEXT)).child(if t.summary.is_empty() { "(Untitled task)".to_string() } else { t.summary.clone() }))
                    .when(!t.due.is_empty(), |d| d.child(pill(format!("Due {}", time::display(&t.due)), theme::TEXT_2, theme::BUBBLE_OTHER)))
                    .when(!url.is_empty(), |d| {
                        d.child(button(sid(format!("open-{}", t.guid)), "Open", false).on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))))
                    })
                    .into_any_element()
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .flex_1()
            .child(self.toolbar("Tasks", format!("{} open task(s) assigned to me", self.tasks.len()), cx))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .px_6()
                    .py_3()
                    .child(div().flex_1().child(self.input(Field::NewTask, "Add a task and press Enter", cx)))
                    .child(button("add-task", "Add", true).on_click(cx.listener(|this, _, _, cx| this.create_task(cx)))),
            )
            .child(
                div()
                    .id("tasks")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .when(rows.is_empty(), |d| d.child(empty(if self.busy > 0 { "Loading tasks…" } else { "All done. No open tasks." })))
                    .children(rows),
            )
    }

    // ------------------------------------------------------------------- docs

    fn docs_view(&self, cx: &mut Context<Self>) -> Div {
        let hits: Vec<AnyElement> = self
            .docs
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let url = h.url.clone();
                div()
                    .id(sid(format!("doc-{i}")))
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_6()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(theme::BORDER))
                    .when(!url.is_empty(), |d| d.cursor_pointer().hover(|s| s.bg(rgb(theme::HOVER))).on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))))
                    .child(pill(if h.kind.is_empty() { "DOC".to_string() } else { h.kind.to_uppercase() }, theme::ACCENT, theme::ACCENT_SOFT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(px(0.))
                            .child(div().text_sm().text_color(rgb(theme::TEXT)).truncate().child(h.title.clone()))
                            .child(div().text_xs().text_color(rgb(theme::MUTED)).truncate().child(if h.owner.is_empty() { h.url.clone() } else { format!("{} · {}", h.owner, h.url) })),
                    )
                    .into_any_element()
            })
            .collect();
        let spaces: Vec<AnyElement> = self
            .spaces
            .iter()
            .map(|s| {
                div()
                    .flex()
                    .flex_col()
                    .w(px(220.))
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(theme::BORDER))
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).text_color(rgb(theme::TEXT)).truncate().child(s.name.clone()))
                    .child(div().text_xs().text_color(rgb(theme::MUTED)).truncate().child(if s.description.is_empty() { s.space_id.clone() } else { s.description.clone() }))
                    .into_any_element()
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .flex_1()
            .child(self.toolbar("Docs & Wiki", "Search documents across Docs, Sheets and Wiki".into(), cx))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .px_6()
                    .py_3()
                    .child(div().flex_1().child(self.input(Field::DocsQuery, "Search docs (Enter)", cx)))
                    .child(button("docs-go", "Search", true).on_click(cx.listener(|this, _, _, cx| this.search_docs(cx)))),
            )
            .child(
                div()
                    .id("docs")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .children(hits)
                    .when(!spaces.is_empty(), |d| {
                        d.child(div().px_6().pt_4().pb_2().text_sm().font_weight(FontWeight::BOLD).text_color(rgb(theme::TEXT)).child("Wiki spaces"))
                            .child(div().flex().flex_wrap().gap_3().px_6().pb_4().children(spaces))
                    })
                    .when(self.docs.is_empty() && self.spaces.is_empty(), |d| d.child(empty("Type a query to search your documents"))),
            )
    }

    // ------------------------------------------------------------------- mail

    fn mail_view(&self, cx: &mut Context<Self>) -> Div {
        let open_id = self.mail_open.as_ref().map(|m| m.message_id.clone()).unwrap_or_default();
        let rows: Vec<AnyElement> = self
            .mails
            .iter()
            .map(|m| {
                let id = m.message_id.clone();
                let selected = !open_id.is_empty() && open_id == m.message_id;
                div()
                    .id(sid(format!("mail-{}", m.message_id)))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(theme::BORDER))
                    .cursor_pointer()
                    .when(selected, |d| d.bg(rgb(theme::ACCENT_SOFT)))
                    .when(!selected, |d| d.hover(|s| s.bg(rgb(theme::HOVER))))
                    .on_click(cx.listener(move |this, _, _, cx| this.open_mail(id.clone(), cx)))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .gap_2()
                            .child(div().text_sm().font_weight(if m.unread { FontWeight::BOLD } else { FontWeight::NORMAL }).text_color(rgb(theme::TEXT)).truncate().child(m.from.clone()))
                            .child(div().flex_none().text_xs().text_color(rgb(theme::MUTED)).child(time::display(&m.date))),
                    )
                    .child(div().text_xs().text_color(rgb(if m.unread { theme::TEXT } else { theme::TEXT_2 })).truncate().child(m.subject.clone()))
                    .into_any_element()
            })
            .collect();
        let list = div()
            .flex()
            .flex_col()
            .w(px(340.))
            .h_full()
            .bg(rgb(theme::PANEL_BG))
            .border_r_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .pt_4()
                    .pb_2()
                    .child(div().text_lg().font_weight(FontWeight::BOLD).child("Mail"))
                    .child(div().flex().gap_2().child(button("compose", "Compose", true).on_click(cx.listener(|this, _, _, cx| this.compose_mail(false, cx)))).child(button("mail-refresh", "↻", false).on_click(cx.listener(|this, _, _, cx| this.refresh(cx))))),
            )
            .child(div().px_3().pb_2().child(self.input(Field::MailQuery, "Search mail (Enter)", cx)))
            .child(
                div()
                    .id("mail-list")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .when(rows.is_empty(), |d| d.child(empty(if self.busy > 0 { "Loading inbox…" } else { "Inbox is empty" })))
                    .children(rows),
            );
        let right = match (self.mail_pane, &self.mail_open) {
            (MailPane::Compose, _) => div()
                .flex()
                .flex_col()
                .flex_1()
                .gap_3()
                .p_6()
                .child(header("New message", "Tab moves between fields · Ctrl+Enter sends"))
                .child(self.input(Field::MailTo, "To (comma-separated addresses)", cx))
                .child(self.input(Field::MailSubject, "Subject", cx))
                .child(self.input(Field::MailBody, "Write your message…", cx).min_h(px(220.)))
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(button("mail-send", "Send", true).on_click(cx.listener(|this, _, _, cx| this.send_mail(cx))))
                        .child(button("mail-discard", "Discard", false).on_click(cx.listener(|this, _, _, cx| {
                            this.mail_pane = MailPane::Inbox;
                            this.active = None;
                            cx.notify();
                        }))),
                ),
            (MailPane::Inbox, Some(m)) => div()
                .flex()
                .flex_col()
                .flex_1()
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap_3()
                        .px_6()
                        .py_4()
                        .border_b_1()
                        .border_color(rgb(theme::BORDER))
                        .child(avatar(&m.from, 36.))
                        .child(header(if m.subject.is_empty() { "(No subject)".to_string() } else { m.subject.clone() }, format!("From {} · To {} · {}", m.from, m.to, time::display(&m.date))))
                        .child(button("reply", "Reply", false).on_click(cx.listener(|this, _, _, cx| this.compose_mail(true, cx)))),
                )
                .child(div().id("mail-body").flex_1().px_6().py_4().overflow_y_scroll().text_sm().text_color(rgb(theme::TEXT)).child(m.body.clone())),
            _ => div().flex().flex_1().child(empty("Select a message to read it")),
        };
        div().flex().flex_1().h_full().child(list).child(right)
    }

    // --------------------------------------------------------------- contacts

    fn contacts_view(&self, cx: &mut Context<Self>) -> Div {
        let rows: Vec<AnyElement> = self
            .users
            .iter()
            .map(|u| {
                let email = u.email.clone();
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_6()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(theme::BORDER))
                    .child(avatar(&u.name, 36.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).text_color(rgb(theme::TEXT)).child(if u.name.is_empty() { u.open_id.clone() } else { u.name.clone() }))
                            .child(div().text_xs().text_color(rgb(theme::MUTED)).child([u.title.as_str(), u.department.as_str(), u.email.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" · "))),
                    )
                    .when(!email.is_empty(), |d| {
                        d.child(button(sid(format!("mailto-{}", u.open_id)), "Email", false).on_click(cx.listener(move |this, _, _, cx| {
                            this.fields.insert(Field::MailTo, email.clone());
                            this.navigate(View::Mail, cx);
                            this.compose_mail(false, cx);
                            this.active = Some(Field::MailSubject);
                        })))
                    })
                    .into_any_element()
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .flex_1()
            .child(self.toolbar("Contacts", "Search people in your organization".into(), cx))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .px_6()
                    .py_3()
                    .child(div().flex_1().child(self.input(Field::ContactsQuery, "Name or email (Enter)", cx)))
                    .child(button("contacts-go", "Search", true).on_click(cx.listener(|this, _, _, cx| this.search_contacts(cx)))),
            )
            .child(
                div()
                    .id("contacts")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .when(rows.is_empty(), |d| d.child(empty("Search for a colleague by name or email")))
                    .children(rows),
            )
    }

    // -------------------------------------------------------------- approvals

    fn approvals_view(&self, cx: &mut Context<Self>) -> Div {
        let rows: Vec<AnyElement> = self
            .approvals
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let url = a.url.clone();
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_6()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(theme::BORDER))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).text_color(rgb(theme::TEXT)).child(a.title.clone()))
                            .child(div().text_xs().text_color(rgb(theme::MUTED)).child([a.initiator.clone(), time::display(&a.time)].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · "))),
                    )
                    .when(!a.status.is_empty(), |d| d.child(pill(a.status.clone(), theme::TEXT_2, theme::BUBBLE_OTHER)))
                    .when(!url.is_empty(), |d| d.child(button(sid(format!("appr-{i}")), "Open", false).on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url)))))
                    .into_any_element()
            })
            .collect();
        let tb = self.toolbar("Approval", "Approval tasks assigned to me (open in Lark to act)".into(), cx);
        let tab = |t: Topic, label: &'static str| {
            button(sid(format!("topic-{}", t.code())), label, self.approval_topic == t).on_click(cx.listener(move |this, _, _, cx| this.set_approval_topic(t, cx)))
        };
        div()
            .flex()
            .flex_col()
            .flex_1()
            .child(tb.child(tab(Topic::Pending, "Pending")).child(tab(Topic::Handled, "Handled")))
            .child(
                div()
                    .id("approvals")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .when(rows.is_empty(), |d| d.child(empty(if self.busy > 0 { "Loading approvals…" } else { "No approvals here" })))
                    .children(rows),
            )
    }

    // ---------------------------------------------------------------- minutes

    fn minutes_view(&self, cx: &mut Context<Self>) -> Div {
        let rows: Vec<AnyElement> = self
            .minutes
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let url = m.url.clone();
                div()
                    .id(sid(format!("minute-{i}")))
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_6()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(theme::BORDER))
                    .when(!url.is_empty(), |d| d.cursor_pointer().hover(|s| s.bg(rgb(theme::HOVER))).on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))))
                    .child(pill("REC", theme::WHITE, theme::ACCENT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(div().text_sm().text_color(rgb(theme::TEXT)).child(m.title.clone()))
                            .child(div().text_xs().text_color(rgb(theme::MUTED)).child([m.owner.clone(), time::display(&m.time)].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · "))),
                    )
                    .into_any_element()
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .flex_1()
            .child(self.toolbar("Minutes", "Meeting recordings and transcripts".into(), cx))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .px_6()
                    .py_3()
                    .child(div().flex_1().child(self.input(Field::MinutesQuery, "Search minutes (Enter)", cx)))
                    .child(button("minutes-go", "Search", true).on_click(cx.listener(|this, _, _, cx| this.search_minutes(cx)))),
            )
            .child(
                div()
                    .id("minutes")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .when(rows.is_empty(), |d| d.child(empty(if self.busy > 0 { "Loading minutes…" } else { "No minutes found" })))
                    .children(rows),
            )
    }

    fn coming_soon(&self, v: View) -> Div {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_3()
            .child(div().flex().items_center().justify_center().size(px(64.)).rounded_xl().bg(rgb(theme::ACCENT_SOFT)).text_color(rgb(theme::ACCENT)).text_xl().font_weight(FontWeight::BOLD).child(v.glyph()))
            .child(div().text_xl().font_weight(FontWeight::BOLD).text_color(rgb(theme::TEXT)).child(v.label()))
            .child(div().text_sm().text_color(rgb(theme::MUTED)).child("Coming soon. This module is not wired to lark-cli yet."))
    }
}

impl Render for LarkApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body: Div = if !self.auth.is_signed_in() {
            self.sign_in(cx)
        } else {
            match self.view {
                View::Messenger => self.messenger(cx),
                View::Calendar => self.calendar(cx),
                View::Tasks => self.tasks_view(cx),
                View::Docs => self.docs_view(cx),
                View::Mail => self.mail_view(cx),
                View::Contacts => self.contacts_view(cx),
                View::Approvals => self.approvals_view(cx),
                View::Minutes => self.minutes_view(cx),
                v => self.coming_soon(v),
            }
        };
        div()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(theme::MAIN_BG))
            .text_color(rgb(theme::TEXT))
            .font_family(".SystemUIFont")
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.))
                    .when(self.auth.is_signed_in(), |d| d.child(self.rail(cx)))
                    .child(body.h_full()),
            )
            .child(self.status_bar())
    }
}
