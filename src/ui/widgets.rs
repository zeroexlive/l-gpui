//! Small stateless building blocks made from GPUI primitives.

use gpui::{Div, FontWeight, SharedString, Stateful, div, prelude::*, px, rgb};

use super::theme;

pub fn avatar(name: &str, size: f32) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(size))
        .rounded_full()
        .bg(rgb(theme::avatar_color(name)))
        .text_color(rgb(theme::WHITE))
        .text_xs()
        .font_weight(FontWeight::BOLD)
        .child(lgpui_core::domains::im::initials(name))
}

pub fn button(id: impl Into<SharedString>, label: impl Into<SharedString>, primary: bool) -> Stateful<Div> {
    let label: SharedString = label.into();
    div()
        .id(id.into())
        .flex_none()
        .px_3()
        .py_1()
        .rounded_md()
        .text_sm()
        .cursor_pointer()
        .when(primary, |d| {
            d.bg(rgb(theme::ACCENT)).text_color(rgb(theme::WHITE)).hover(|s| s.bg(rgb(theme::ACCENT_HOVER)))
        })
        .when(!primary, |d| {
            d.border_1()
                .border_color(rgb(theme::BORDER))
                .bg(rgb(theme::MAIN_BG))
                .text_color(rgb(theme::TEXT))
                .hover(|s| s.bg(rgb(theme::HOVER)))
        })
        .child(label)
}

pub fn header(title: impl Into<SharedString>, subtitle: impl Into<SharedString>) -> Div {
    let subtitle: SharedString = subtitle.into();
    div()
        .flex()
        .flex_col()
        .flex_1()
        .child(div().text_lg().font_weight(FontWeight::BOLD).text_color(rgb(theme::TEXT)).child(title.into()))
        .when(!subtitle.is_empty(), |d| d.child(div().text_xs().text_color(rgb(theme::MUTED)).child(subtitle)))
}

pub fn empty(text: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .p_8()
        .text_sm()
        .text_color(rgb(theme::MUTED))
        .child(text.into())
}

pub fn pill(text: impl Into<SharedString>, fg: u32, bg: u32) -> Div {
    div().flex_none().px_2().rounded_md().text_xs().text_color(rgb(fg)).bg(rgb(bg)).child(text.into())
}

pub fn code_line(text: impl Into<SharedString>) -> Div {
    div()
        .flex_1()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(rgb(theme::CODE_BG))
        .text_color(rgb(theme::CODE_TEXT))
        .font_family("monospace")
        .text_sm()
        .child(text.into())
}
