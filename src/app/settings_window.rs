//! 设置窗口：用 GTK 控件编辑 [`Settings`]，风格简洁（类 macOS），支持中英文。
//!
//! 这是普通 `gtk::Window`（非 layer-shell），能正常获得键盘焦点。控件改动**实时生效**：
//! 每改一项就写回共享设置、持久化并热更新悬浮层 / 主题 / 文案。「重置」把全部设置
//! 改回默认值。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use gtk4 as gtk;
use gtk::glib;
use gtk::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use crate::config::{Hotkey, Language, Position, Settings, SettingsHandle, Theme};
use crate::control::PauseControl;
use crate::input::keymap::hotkey_label;
use crate::overlay::Overlay;

/// 界面文案（中 / 英），`rows` 顺序与 `build` 里 add_row 的调用顺序一致。
struct Strings {
    title: &'static str,
    sections: [&'static str; 4],
    rows: [&'static str; 16],
    light: &'static str,
    dark: &'static str,
    positions: [&'static str; 4],
    reset: &'static str,
    done: &'static str,
    record_hint: &'static str,
}

impl Strings {
    fn for_lang(lang: Language) -> Self {
        match lang {
            Language::Zh => Self {
                title: "设置",
                sections: ["外观", "胶囊", "布局", "行为"],
                rows: [
                    "界面主题",
                    "胶囊配色",
                    "语言",
                    "最新透明度 (%)",
                    "历史透明度 (%)",
                    "圆角 (px)",
                    "间距 (px)",
                    "字体大小 (px)",
                    "位置",
                    "水平距离 (px)",
                    "垂直距离 (px)",
                    "显示时长 (ms)",
                    "最大数量",
                    "消失动画时长 (ms)",
                    "暂停快捷键",
                    "开机自启",
                ],
                light: "明亮",
                dark: "黑暗",
                positions: ["左下", "右下", "左上", "右上"],
                reset: "重置",
                done: "完成",
                record_hint: "请按下组合键…",
            },
            Language::En => Self {
                title: "Settings",
                sections: ["Appearance", "Chips", "Layout", "Behavior"],
                rows: [
                    "Theme",
                    "Chip color",
                    "Language",
                    "Latest opacity (%)",
                    "History opacity (%)",
                    "Corner radius (px)",
                    "Spacing (px)",
                    "Font size (px)",
                    "Position",
                    "Horizontal distance (px)",
                    "Vertical distance (px)",
                    "Duration (ms)",
                    "Max chips",
                    "Fade duration (ms)",
                    "Pause hotkey",
                    "Autostart",
                ],
                light: "Light",
                dark: "Dark",
                positions: ["Bottom-left", "Bottom-right", "Top-left", "Top-right"],
                reset: "Reset",
                done: "Done",
                record_hint: "Press a combo…",
            },
        }
    }
}

/// 设置窗口自身的 CSS：按主题生成明 / 暗两套简洁调色板，含右上角关闭按钮。
fn window_css(theme: Theme) -> String {
    let (bg, fg, muted, sep, close_bg, close_fg, close_hover) = match theme {
        // 关闭按钮：暗色 = 浅一点的黑色；亮色 = 浅一点的白色（略灰，稍微可见但不突兀）
        Theme::Light => (
            "#f5f5f7",
            "#1d1d1f",
            "#86868b",
            "rgba(0, 0, 0, 0.08)",
            "#e2e2e5",
            "#1d1d1f",
            "#d2d2d6",
        ),
        Theme::Dark => (
            "#1c1c1e",
            "#f5f5f7",
            "#98989d",
            "rgba(255, 255, 255, 0.10)",
            "#333336",
            "#f5f5f7",
            "#454549",
        ),
    };
    format!(
        r#"
.settings-window {{
    background-color: {bg};
    color: {fg};
}}
.settings-root {{
    background-color: {bg};
    padding: 12px 20px 20px 20px;
}}
.section-header {{
    color: {muted};
    font-size: 12px;
    font-weight: 600;
    margin-top: 20px;
    margin-bottom: 4px;
}}
.settings-row {{
    padding: 9px 0;
    border-bottom: 1px solid {sep};
}}
.row-label {{
    font-size: 14px;
}}
.close-button {{
    min-width: 26px;
    min-height: 26px;
    border-radius: 13px;
    padding: 0;
    font-size: 12px;
    font-weight: bold;
    border: none;
    box-shadow: none;
    background-image: none;
    background-color: {close_bg};
    color: {close_fg};
}}
.close-button:hover {{
    background-color: {close_hover};
}}
"#,
        bg = bg,
        fg = fg,
        muted = muted,
        sep = sep,
        close_bg = close_bg,
        close_fg = close_fg,
        close_hover = close_hover,
    )
}

/// 屏幕工作区高度（像素），用于限制设置窗口最大高度为屏幕的 60%。取不到就回退 900。
fn screen_workarea_height() -> i32 {
    gtk::gdk::Display::default()
        .and_then(|d| d.monitors().item(0))
        .and_then(|m| m.downcast::<gtk::gdk::Monitor>().ok())
        .map(|m| m.geometry().height())
        .unwrap_or(900)
}

/// 整数取值用的 `SpinButton`。
fn make_spin(value: f64, lower: f64, upper: f64, step: f64) -> gtk::SpinButton {
    let adj = gtk::Adjustment::new(value, lower, upper, step, step * 10.0, 0.0);
    gtk::SpinButton::new(Some(&adj), 1.0, 0)
}

/// 追加一个分组标题，返回该 label（供语言切换刷新文案）。
fn add_section(parent: &gtk::Box, title: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(title));
    label.add_css_class("section-header");
    label.set_halign(gtk::Align::Start);
    parent.append(&label);
    label
}

/// 追加一行「左 label + 右控件」，返回该 label（供语言切换刷新文案）。
fn add_row(parent: &gtk::Box, title: &str, control: &impl IsA<gtk::Widget>) -> gtk::Label {
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
fn set_dropdown_items(dropdown: &gtk::DropDown, items: &[&str]) {
    let selected = dropdown.selected();
    let model = gtk::StringList::new(items);
    dropdown.set_model(Some(&model));
    dropdown.set_selected(selected);
}

fn theme_index(theme: Theme) -> u32 {
    match theme {
        Theme::Light => 0,
        Theme::Dark => 1,
    }
}

fn lang_index(lang: Language) -> u32 {
    match lang {
        Language::Zh => 0,
        Language::En => 1,
    }
}

fn position_index(position: Position) -> u32 {
    match position {
        Position::BottomLeft => 0,
        Position::BottomRight => 1,
        Position::TopLeft => 2,
        Position::TopRight => 3,
    }
}

/// 设置窗口。控件变化实时生效。
pub struct SettingsWindow {
    window: gtk::Window,
    settings: SettingsHandle,
    overlay: Rc<Overlay>,
    refresh_tx: UnboundedSender<()>,
    provider: gtk::CssProvider,

    theme_dropdown: gtk::DropDown,
    chip_theme_dropdown: gtk::DropDown,
    lang_dropdown: gtk::DropDown,
    position_dropdown: gtk::DropDown,
    alpha_spin: gtk::SpinButton,
    alpha_history_spin: gtk::SpinButton,
    radius_spin: gtk::SpinButton,
    spacing_spin: gtk::SpinButton,
    margin_x_spin: gtk::SpinButton,
    margin_y_spin: gtk::SpinButton,
    duration_spin: gtk::SpinButton,
    max_chips_spin: gtk::SpinButton,
    font_spin: gtk::SpinButton,
    fade_spin: gtk::SpinButton,
    autostart_switch: gtk::Switch,
    hotkey_button: gtk::Button,
    // 快捷键草稿（录制后暂存，实时写回设置）
    hotkey_draft: RefCell<Hotkey>,
    pause_ctl: Arc<PauseControl>,
    // 程序化修改控件时置位，抑制实时应用（sync/reset/刷新文案）
    syncing: RefCell<bool>,
    reset_button: gtk::Button,
    done_button: gtk::Button,

    section_headers: Vec<gtk::Label>,
    row_labels: Vec<gtk::Label>,
}

impl SettingsWindow {
    /// 创建（但不显示）设置窗口。`overlay` 用于保存后热更新悬浮层；`refresh_tx`
    /// 用于语言切换后通知托盘刷新菜单。
    pub fn build(
        app: &gtk::Application,
        settings: SettingsHandle,
        overlay: Rc<Overlay>,
        refresh_tx: UnboundedSender<()>,
        pause_ctl: Arc<PauseControl>,
    ) -> Rc<Self> {
        // 最大高度 = 屏幕高度的 60%；默认高度取 640 与 60% 的较小值，内容超出可滚动
        let target_height = 640.min(screen_workarea_height() * 60 / 100);

        let window = gtk::Window::builder()
            .application(app)
            .title("设置")
            .default_width(440)
            .default_height(target_height)
            .resizable(false)
            .build();
        window.add_css_class("settings-window");
        window.set_decorated(false);

        let provider = gtk::CssProvider::new();
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let init: Settings = settings.read().unwrap().clone();
        let strings = Strings::for_lang(init.language);

        // 下拉：主题 / 胶囊配色 / 语言
        let theme_dropdown = gtk::DropDown::from_strings(&[strings.light, strings.dark]);
        theme_dropdown.set_selected(theme_index(init.theme));
        let chip_theme_dropdown = gtk::DropDown::from_strings(&[strings.light, strings.dark]);
        chip_theme_dropdown.set_selected(theme_index(init.chip_theme));
        let lang_dropdown = gtk::DropDown::from_strings(&["中文", "English"]);
        lang_dropdown.set_selected(lang_index(init.language));
        let position_dropdown = gtk::DropDown::from_strings(&strings.positions);
        position_dropdown.set_selected(position_index(init.position));

        // 数字控件
        let alpha_spin = make_spin(f64::from(init.chip_alpha) * 100.0, 0.0, 100.0, 5.0);
        let alpha_history_spin =
            make_spin(f64::from(init.chip_alpha_history) * 100.0, 0.0, 100.0, 5.0);
        let radius_spin = make_spin(f64::from(init.border_radius), 0.0, 50.0, 1.0);
        let spacing_spin = make_spin(f64::from(init.spacing), 0.0, 100.0, 2.0);
        let margin_x_spin = make_spin(f64::from(init.margin_x), 0.0, 200.0, 4.0);
        let margin_y_spin = make_spin(f64::from(init.margin_y), 0.0, 200.0, 4.0);
        let duration_spin = make_spin(init.display_duration_ms as f64, 500.0, 10000.0, 250.0);
        let max_chips_spin = make_spin(init.max_chips as f64, 1.0, 6.0, 1.0);
        let font_spin = make_spin(f64::from(init.font_size), 10.0, 48.0, 1.0);
        let fade_spin = make_spin(init.fade_duration_ms as f64, 50.0, 1000.0, 25.0);
        let autostart_switch = gtk::Switch::new();
        autostart_switch.set_active(init.autostart);
        // 暂停快捷键按钮：显示当前组合，点击进入录制
        let hotkey_button = gtk::Button::with_label(&hotkey_label(&init.pause_hotkey));

        // 布局：分组标题 + 行（rows 顺序见 Strings::rows）
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("settings-root");

        // 顶部：右上角关闭按钮
        let close_button = gtk::Button::with_label("✖");
        close_button.add_css_class("close-button");
        {
            let window = window.clone();
            close_button.connect_clicked(move |_| window.hide());
        }
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let spacer = gtk::Label::new(None);
        spacer.set_hexpand(true);
        header.append(&spacer);
        header.append(&close_button);
        root.append(&header);

        let mut section_headers = Vec::new();
        let mut row_labels = Vec::new();

        section_headers.push(add_section(&root, strings.sections[0]));
        row_labels.push(add_row(&root, strings.rows[0], &theme_dropdown));
        row_labels.push(add_row(&root, strings.rows[1], &chip_theme_dropdown));
        row_labels.push(add_row(&root, strings.rows[2], &lang_dropdown));

        section_headers.push(add_section(&root, strings.sections[1]));
        row_labels.push(add_row(&root, strings.rows[3], &alpha_spin));
        row_labels.push(add_row(&root, strings.rows[4], &alpha_history_spin));
        row_labels.push(add_row(&root, strings.rows[5], &radius_spin));
        row_labels.push(add_row(&root, strings.rows[6], &spacing_spin));
        row_labels.push(add_row(&root, strings.rows[7], &font_spin));

        section_headers.push(add_section(&root, strings.sections[2]));
        row_labels.push(add_row(&root, strings.rows[8], &position_dropdown));
        row_labels.push(add_row(&root, strings.rows[9], &margin_x_spin));
        row_labels.push(add_row(&root, strings.rows[10], &margin_y_spin));

        section_headers.push(add_section(&root, strings.sections[3]));
        row_labels.push(add_row(&root, strings.rows[11], &duration_spin));
        row_labels.push(add_row(&root, strings.rows[12], &max_chips_spin));
        row_labels.push(add_row(&root, strings.rows[13], &fade_spin));
        row_labels.push(add_row(&root, strings.rows[14], &hotkey_button));
        row_labels.push(add_row(&root, strings.rows[15], &autostart_switch));

        // 底部按钮：重置（改回默认值）+ 完成（关闭设置）
        let reset_button = gtk::Button::with_label(strings.reset);
        let done_button = gtk::Button::with_label(strings.done);
        done_button.add_css_class("suggested-action");
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        buttons.set_halign(gtk::Align::End);
        buttons.set_margin_top(24);
        buttons.append(&reset_button);
        buttons.append(&done_button);
        root.append(&buttons);

        // 窗口高度固定（target_height），内容超出时滚动
        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_hscrollbar_policy(gtk::PolicyType::Never);
        scrolled.set_vscrollbar_policy(gtk::PolicyType::Automatic);
        scrolled.set_child(Some(&root));
        window.set_child(Some(&scrolled));

        // 关闭按钮 = 隐藏而非销毁，可重复打开
        window.connect_close_request(|w| {
            w.hide();
            glib::Propagation::Stop
        });

        let sw = Rc::new(Self {
            window,
            settings,
            overlay,
            refresh_tx,
            provider,
            theme_dropdown,
            chip_theme_dropdown,
            lang_dropdown,
            position_dropdown,
            alpha_spin,
            alpha_history_spin,
            radius_spin,
            spacing_spin,
            margin_x_spin,
            margin_y_spin,
            duration_spin,
            max_chips_spin,
            font_spin,
            fade_spin,
            autostart_switch,
            hotkey_button,
            hotkey_draft: RefCell::new(init.pause_hotkey),
            pause_ctl,
            syncing: RefCell::new(false),
            reset_button,
            done_button,
            section_headers,
            row_labels,
        });

        // 应用当前（已保存的）主题
        sw.apply_window_theme();

        {
            let sw = Rc::clone(&sw);
            let button = sw.reset_button.clone();
            button.connect_clicked(move |_| sw.reset());
        }

        // 下拉（主题 / 胶囊配色 / 语言）变化 → 实时应用
        {
            let sw = Rc::clone(&sw);
            let dropdown = sw.theme_dropdown.clone();
            dropdown.connect_selected_notify(move |_| sw.on_change());
        }
        {
            let sw = Rc::clone(&sw);
            let dropdown = sw.chip_theme_dropdown.clone();
            dropdown.connect_selected_notify(move |_| sw.on_change());
        }
        {
            let sw = Rc::clone(&sw);
            let dropdown = sw.lang_dropdown.clone();
            dropdown.connect_selected_notify(move |_| sw.on_change());
        }

        // 数字控件变化 → 实时应用
        {
            let sw = Rc::clone(&sw);
            let spin = sw.alpha_spin.clone();
            spin.connect_value_changed(move |_| sw.on_change());
        }
        {
            let sw = Rc::clone(&sw);
            let spin = sw.alpha_history_spin.clone();
            spin.connect_value_changed(move |_| sw.on_change());
        }
        {
            let sw = Rc::clone(&sw);
            let spin = sw.radius_spin.clone();
            spin.connect_value_changed(move |_| sw.on_change());
        }
        {
            let sw = Rc::clone(&sw);
            let spin = sw.spacing_spin.clone();
            spin.connect_value_changed(move |_| sw.on_change());
        }
        {
            let sw = Rc::clone(&sw);
            let spin = sw.margin_x_spin.clone();
            spin.connect_value_changed(move |_| sw.on_change());
        }
        {
            let sw = Rc::clone(&sw);
            let spin = sw.margin_y_spin.clone();
            spin.connect_value_changed(move |_| sw.on_change());
        }
        {
            let sw = Rc::clone(&sw);
            let spin = sw.duration_spin.clone();
            spin.connect_value_changed(move |_| sw.on_change());
        }
        {
            let sw = Rc::clone(&sw);
            let spin = sw.max_chips_spin.clone();
            spin.connect_value_changed(move |_| sw.on_change());
        }
        {
            let sw = Rc::clone(&sw);
            let spin = sw.font_spin.clone();
            spin.connect_value_changed(move |_| sw.on_change());
        }
        {
            let sw = Rc::clone(&sw);
            let spin = sw.fade_spin.clone();
            spin.connect_value_changed(move |_| sw.on_change());
        }

        // 开机自启开关 → 实时应用
        {
            let sw = Rc::clone(&sw);
            let switch = sw.autostart_switch.clone();
            switch.connect_active_notify(move |_| sw.on_change());
        }

        // 位置下拉：实时应用
        {
            let sw = Rc::clone(&sw);
            let dropdown = sw.position_dropdown.clone();
            dropdown.connect_selected_notify(move |_| sw.on_change());
        }

        // 「完成」按钮：关闭设置窗口
        {
            let sw = Rc::clone(&sw);
            let button = sw.done_button.clone();
            button.connect_clicked(move |_| sw.window.hide());
        }

        // 点击「暂停快捷键」按钮 → 进入录制模式，等待键盘线程捕获下一个组合
        {
            let sw = Rc::clone(&sw);
            let button = sw.hotkey_button.clone();
            button.connect_clicked(move |_| {
                *sw.pause_ctl.captured.lock().unwrap() = None;
                sw.pause_ctl.capture.store(true, Ordering::SeqCst);
                let hint = Strings::for_lang(sw.settings.read().unwrap().language).record_hint;
                sw.hotkey_button.set_label(hint);
            });
        }

        // 轮询键盘线程捕获到的快捷键，实时写回并应用
        {
            let sw = Rc::clone(&sw);
            glib::timeout_add_local(Duration::from_millis(100), move || {
                if let Some(hotkey) = sw.pause_ctl.captured.lock().unwrap().take() {
                    *sw.hotkey_draft.borrow_mut() = hotkey;
                    sw.hotkey_button.set_label(&hotkey_label(&hotkey));
                    sw.apply();
                }
                glib::ControlFlow::Continue
            });
        }

        sw
    }

    /// 显示（置前）设置窗口，先同步控件到当前已保存的设置。
    pub fn show(&self) {
        self.sync_from_settings();
        self.window.present();
    }

    /// 把控件当前值实时写回共享设置并持久化，再热更新悬浮层 / 主题 / 文案。
    fn apply(&self) {
        let theme = if self.theme_dropdown.selected() == 0 {
            Theme::Light
        } else {
            Theme::Dark
        };
        let chip_theme = if self.chip_theme_dropdown.selected() == 0 {
            Theme::Light
        } else {
            Theme::Dark
        };
        let language = if self.lang_dropdown.selected() == 0 {
            Language::Zh
        } else {
            Language::En
        };
        let position = match self.position_dropdown.selected() {
            0 => Position::BottomLeft,
            1 => Position::BottomRight,
            2 => Position::TopLeft,
            _ => Position::TopRight,
        };
        let pause_hotkey = *self.hotkey_draft.borrow();
        let autostart = self.autostart_switch.is_active();

        let (theme_changed, lang_changed, autostart_changed) = {
            let mut s = self.settings.write().unwrap();
            let theme_changed = s.theme != theme;
            let lang_changed = s.language != language;
            let autostart_changed = s.autostart != autostart;

            s.theme = theme;
            s.chip_theme = chip_theme;
            s.language = language;
            s.position = position;
            s.chip_alpha = (self.alpha_spin.value() as f32) / 100.0;
            s.chip_alpha_history = (self.alpha_history_spin.value() as f32) / 100.0;
            s.border_radius = self.radius_spin.value() as u32;
            s.spacing = self.spacing_spin.value() as i32;
            s.margin_x = self.margin_x_spin.value() as i32;
            s.margin_y = self.margin_y_spin.value() as i32;
            s.display_duration_ms = self.duration_spin.value() as u64;
            s.max_chips = self.max_chips_spin.value() as usize;
            s.font_size = self.font_spin.value() as u32;
            s.fade_duration_ms = self.fade_spin.value() as u64;
            s.pause_hotkey = pause_hotkey;
            s.autostart = autostart;
            s.save();

            (theme_changed, lang_changed, autostart_changed)
        };

        // 胶囊相关设置：重应用悬浮层样式
        self.overlay.apply_settings();

        // 开机自启：只在开关状态变化时写入 / 删除自启文件
        if autostart_changed {
            super::autostart::apply(autostart);
        }

        if theme_changed {
            self.apply_window_theme();
        }
        if lang_changed {
            self.refresh_labels();
            let _ = self.refresh_tx.send(());
        }
    }

    /// 控件变化统一入口：忽略程序化修改（`syncing`），实时应用所有设置。
    fn on_change(&self) {
        if *self.syncing.borrow() {
            return;
        }
        self.apply();
    }

    /// 把控件刷新为当前已保存的设置值，打开窗口时调用（不触发实时应用）。
    fn sync_from_settings(&self) {
        *self.syncing.borrow_mut() = true;
        let s = self.settings.read().unwrap().clone();
        self.theme_dropdown.set_selected(theme_index(s.theme));
        self.chip_theme_dropdown.set_selected(theme_index(s.chip_theme));
        self.lang_dropdown.set_selected(lang_index(s.language));
        self.position_dropdown.set_selected(position_index(s.position));
        self.alpha_spin.set_value(f64::from(s.chip_alpha) * 100.0);
        self.alpha_history_spin
            .set_value(f64::from(s.chip_alpha_history) * 100.0);
        self.radius_spin.set_value(f64::from(s.border_radius));
        self.spacing_spin.set_value(f64::from(s.spacing));
        self.margin_x_spin.set_value(f64::from(s.margin_x));
        self.margin_y_spin.set_value(f64::from(s.margin_y));
        self.duration_spin.set_value(s.display_duration_ms as f64);
        self.max_chips_spin.set_value(s.max_chips as f64);
        self.font_spin.set_value(f64::from(s.font_size));
        self.fade_spin.set_value(s.fade_duration_ms as f64);
        self.autostart_switch.set_active(s.autostart);
        *self.hotkey_draft.borrow_mut() = s.pause_hotkey;
        self.hotkey_button.set_label(&hotkey_label(&s.pause_hotkey));
        *self.syncing.borrow_mut() = false;
    }

    /// 把控件改回默认值并实时应用。
    fn reset(&self) {
        *self.syncing.borrow_mut() = true;
        let d = Settings::default();
        self.theme_dropdown.set_selected(theme_index(d.theme));
        self.chip_theme_dropdown.set_selected(theme_index(d.chip_theme));
        self.lang_dropdown.set_selected(lang_index(d.language));
        self.position_dropdown.set_selected(position_index(d.position));
        self.alpha_spin.set_value(f64::from(d.chip_alpha) * 100.0);
        self.alpha_history_spin
            .set_value(f64::from(d.chip_alpha_history) * 100.0);
        self.radius_spin.set_value(f64::from(d.border_radius));
        self.spacing_spin.set_value(f64::from(d.spacing));
        self.margin_x_spin.set_value(f64::from(d.margin_x));
        self.margin_y_spin.set_value(f64::from(d.margin_y));
        self.duration_spin.set_value(d.display_duration_ms as f64);
        self.max_chips_spin.set_value(d.max_chips as f64);
        self.font_spin.set_value(f64::from(d.font_size));
        self.fade_spin.set_value(d.fade_duration_ms as f64);
        self.autostart_switch.set_active(d.autostart);
        *self.hotkey_draft.borrow_mut() = d.pause_hotkey;
        self.hotkey_button.set_label(&hotkey_label(&d.pause_hotkey));
        *self.syncing.borrow_mut() = false;
        self.apply();
    }

    /// 按当前主题重载设置窗口 CSS，并切换 GTK 明暗。
    fn apply_window_theme(&self) {
        let theme = self.settings.read().unwrap().theme;
        self.provider.load_from_data(&window_css(theme));
        if let Some(gtk_settings) = gtk::Settings::default() {
            gtk_settings.set_gtk_application_prefer_dark_theme(theme == Theme::Dark);
        }
    }

    /// 语言切换后刷新所有文案与下拉条目（抑制控件信号，避免重入）。
    fn refresh_labels(&self) {
        *self.syncing.borrow_mut() = true;
        let lang = self.settings.read().unwrap().language;
        let s = Strings::for_lang(lang);

        self.window.set_title(Some(s.title));
        for (i, header) in self.section_headers.iter().enumerate() {
            header.set_label(s.sections[i]);
        }
        for (i, label) in self.row_labels.iter().enumerate() {
            label.set_label(s.rows[i]);
        }
        set_dropdown_items(&self.theme_dropdown, &[s.light, s.dark]);
        set_dropdown_items(&self.chip_theme_dropdown, &[s.light, s.dark]);
        set_dropdown_items(&self.position_dropdown, &s.positions);
        self.reset_button.set_label(s.reset);
        self.done_button.set_label(s.done);
        *self.syncing.borrow_mut() = false;
    }
}
