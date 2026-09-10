//! 设置窗口布局的小工具：屏幕高度、数值 / 下拉 / 开关等控件的构造与下拉重建。

use gtk4 as gtk;
use gtk::prelude::*;

use crate::config::{Language, Position, Theme};

/// 屏幕高度（像素），用于给设置窗口的默认高度定上限。取不到就回退 900。
///
/// 注意是整块显示器的高度而非「工作区」：GDK4 未暴露 `workarea`，且 Wayland 下
/// 面板 / 栏本来也不缩小它，两者实际等价。
pub(super) fn screen_height() -> i32 {
    gtk::gdk::Display::default()
        .and_then(|d| d.monitors().item(0))
        .and_then(|m| m.downcast::<gtk::gdk::Monitor>().ok())
        .map(|m| m.geometry().height())
        .unwrap_or(900)
}

/// 整数取值用的 `SpinButton`。
pub(super) fn make_spin(value: f64, lower: f64, upper: f64, step: f64) -> gtk::SpinButton {
    let adj = gtk::Adjustment::new(value, lower, upper, step, step * 10.0, 0.0);
    gtk::SpinButton::new(Some(&adj), 1.0, 0)
}

/// 追加一个分组标题，返回该 label（供语言切换刷新文案）。
pub(super) fn add_section(parent: &gtk::Box, title: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(title));
    label.add_css_class("section-header");
    label.set_halign(gtk::Align::Start);
    parent.append(&label);
    label
}

/// 追加一行「左 label + 右控件」，返回该 label（供语言切换刷新文案）。
pub(super) fn add_row(parent: &gtk::Box, title: &str, control: &impl IsA<gtk::Widget>) -> gtk::Label {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.add_css_class("settings-row");
    let label = gtk::Label::new(Some(title));
    label.add_css_class("row-label");
    label.set_halign(gtk::Align::Start);
    label.set_hexpand(true);
    row.append(&label);
    row.append(control);
    parent.append(&row);
    label
}

/// 重建下拉条目文本（保持当前选中项）。
pub(super) fn set_dropdown_items(dropdown: &gtk::DropDown, items: &[&str]) {
    let selected = dropdown.selected();
    let model = gtk::StringList::new(items);
    dropdown.set_model(Some(&model));
    dropdown.set_selected(selected);
}

pub(super) fn theme_index(theme: Theme) -> u32 {
    match theme {
        Theme::Light => 0,
        Theme::Dark => 1,
    }
}

pub(super) fn lang_index(lang: Language) -> u32 {
    match lang {
        Language::Zh => 0,
        Language::En => 1,
    }
}

pub(super) fn position_index(position: Position) -> u32 {
    match position {
        Position::BottomLeft => 0,
        Position::BottomRight => 1,
        Position::TopLeft => 2,
        Position::TopRight => 3,
    }
}
