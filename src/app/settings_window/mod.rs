//! 设置窗口：用 GTK 控件编辑 [`Settings`]，风格简洁（类 macOS），支持中英文。
//!
//! 这是普通 `gtk::Window`（非 layer-shell），能正常获得键盘焦点。控件改动**实时生效**：
//! 每改一项就写回共享设置、持久化并热更新悬浮层 / 主题 / 文案。「重置」把全部设置
//! 改回默认值。
//!
//! 目录模块：主窗口逻辑在本文件；界面文案 / 主题 CSS / 行控件构造分见
//! [`strings`] / [`theme`] / [`widgets`]。

mod strings;
mod theme;
mod widgets;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use gtk4 as gtk;
use gtk::glib;
use gtk::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use crate::config::{Hotkey, Language, Position, Settings, SettingsHandle, Theme};
use crate::core::PauseControl;
use crate::input::keymap::hotkey_label;
use crate::overlay::Overlay;

use strings::Strings;
use theme::window_css;
use widgets::{
    add_row, add_section, lang_index, make_spin, position_index, screen_height,
    set_dropdown_items, theme_index,
};

/// 设置窗口宽度（像素）：固定值，所有行按这个宽度测量。
const WINDOW_WIDTH: i32 = 440;

/// 设置窗口最小高度（像素）：屏幕过矮时的兜底。
const MIN_WINDOW_HEIGHT: i32 = 360;

/// 默认高度上限占屏幕高度的百分比：小屏上留出四周余量，不让窗口像全屏。
const MAX_HEIGHT_PERCENT: i32 = 75;

/// 配置写盘的防抖延迟。
///
/// 控件是实时生效的，拖动一次数值 SpinButton 会触发几十次变化；每次都写盘既慢又
/// 无意义，攒这么久之后只写一次（写入的始终是触发时刻的最新值）。
const SAVE_DEBOUNCE: Duration = Duration::from_millis(400);

/// 设置窗口。控件变化实时生效。
pub struct SettingsWindow {
    window: gtk::Window,
    settings: SettingsHandle,
    overlay: Rc<Overlay>,
    refresh_tx: UnboundedSender<()>,
    provider: gtk::CssProvider,
    title_label: gtk::Label,

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
    click_through_switch: gtk::Switch,
    hotkey_button: gtk::Button,
    // 快捷键草稿（录制后暂存，实时写回设置）
    hotkey_draft: RefCell<Hotkey>,
    pause_ctl: Arc<PauseControl>,
    // 程序化修改控件时置位，抑制实时应用（sync/reset/刷新文案）
    syncing: RefCell<bool>,
    // 已排好一次防抖写盘（计时器触发时会重新读最新设置，期间的变化无需再排队）
    save_armed: Rc<Cell<bool>>,
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
        let window = gtk::Window::builder()
            .application(app)
            .title("设置")
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
        let click_through_switch = gtk::Switch::new();
        click_through_switch.set_active(init.click_through);
        // 暂停快捷键按钮：显示当前组合，点击进入录制
        let hotkey_button = gtk::Button::with_label(&hotkey_label(&init.pause_hotkey));

        // 布局：顶部标题栏（固定）+ 中部滚动内容 + 底部按钮栏（固定）。
        // 标题栏与「重置 / 完成」都在滚动区外，不随内容滚走。
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("settings-root");

        // 顶部：标题 + 右上角关闭按钮
        let title_label = gtk::Label::new(Some(strings.title));
        title_label.add_css_class("settings-title");
        title_label.set_halign(gtk::Align::Start);
        let close_button = gtk::Button::with_label("✖");
        close_button.add_css_class("close-button");
        {
            let window = window.clone();
            close_button.connect_clicked(move |_| window.hide());
        }
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        header.add_css_class("settings-header");
        header.append(&title_label);
        let spacer = gtk::Label::new(None);
        spacer.set_hexpand(true);
        header.append(&spacer);
        header.append(&close_button);
        root.append(&header);

        // 滚动区内容：分组标题 + 行（rows 顺序见 Strings::rows）
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.add_css_class("settings-content");

        let mut section_headers = Vec::new();
        let mut row_labels = Vec::new();

        // 外观：设置窗口主题 / 胶囊配色 / 语言
        section_headers.push(add_section(&content, strings.sections[0]));
        row_labels.push(add_row(&content, strings.rows[0], &theme_dropdown));
        row_labels.push(add_row(&content, strings.rows[1], &chip_theme_dropdown));
        row_labels.push(add_row(&content, strings.rows[2], &lang_dropdown));

        // 胶囊：透明度 / 圆角 / 字体
        section_headers.push(add_section(&content, strings.sections[1]));
        row_labels.push(add_row(&content, strings.rows[3], &alpha_spin));
        row_labels.push(add_row(&content, strings.rows[4], &alpha_history_spin));
        row_labels.push(add_row(&content, strings.rows[5], &radius_spin));
        row_labels.push(add_row(&content, strings.rows[6], &font_spin));

        // 布局：位置 / 边距 / 间距 / 最大数量
        section_headers.push(add_section(&content, strings.sections[2]));
        row_labels.push(add_row(&content, strings.rows[7], &position_dropdown));
        row_labels.push(add_row(&content, strings.rows[8], &margin_x_spin));
        row_labels.push(add_row(&content, strings.rows[9], &margin_y_spin));
        row_labels.push(add_row(&content, strings.rows[10], &spacing_spin));
        row_labels.push(add_row(&content, strings.rows[11], &max_chips_spin));

        // 行为：时长 / 快捷键 / 鼠标穿透 / 开机自启
        section_headers.push(add_section(&content, strings.sections[3]));
        row_labels.push(add_row(&content, strings.rows[12], &duration_spin));
        row_labels.push(add_row(&content, strings.rows[13], &fade_spin));
        row_labels.push(add_row(&content, strings.rows[14], &hotkey_button));
        row_labels.push(add_row(&content, strings.rows[15], &click_through_switch));
        row_labels.push(add_row(&content, strings.rows[16], &autostart_switch));

        // 中部滚动区：只有这部分内容滚动，标题栏与底部按钮栏固定
        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_hscrollbar_policy(gtk::PolicyType::Never);
        scrolled.set_vscrollbar_policy(gtk::PolicyType::Automatic);
        scrolled.set_vexpand(true);
        scrolled.set_child(Some(&content));
        root.append(&scrolled);

        // 底部按钮栏：重置（改回默认值）+ 完成（关闭设置），固定在滚动区外
        let reset_button = gtk::Button::with_label(strings.reset);
        let done_button = gtk::Button::with_label(strings.done);
        done_button.add_css_class("suggested-action");
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        buttons.add_css_class("settings-footer");
        buttons.set_halign(gtk::Align::End);
        buttons.append(&reset_button);
        buttons.append(&done_button);
        root.append(&buttons);

        window.set_child(Some(&root));

        // 默认高度 = 内容自然高度，夹在 [MIN_WINDOW_HEIGHT, 屏幕高度 75%] 之间：
        // 屏幕放得下就一屏显示完（没有滚动条），放不下才滚动。
        // 高度要算上滚动区之外的标题栏与底部按钮栏，否则窗口会矮一截、白白多出滚动条。
        // 窗口保持 resizable(false)（尺寸约束 min == max），niri 等合成器据此仍自动浮动。
        let chrome_height = header.measure(gtk::Orientation::Vertical, WINDOW_WIDTH).1
            + buttons.measure(gtk::Orientation::Vertical, WINDOW_WIDTH).1;
        let natural_height =
            chrome_height + content.measure(gtk::Orientation::Vertical, WINDOW_WIDTH).1;
        let max_height = (screen_height() * MAX_HEIGHT_PERCENT / 100).max(MIN_WINDOW_HEIGHT);
        window.set_default_size(
            WINDOW_WIDTH,
            natural_height.clamp(MIN_WINDOW_HEIGHT, max_height),
        );

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
            title_label,
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
            click_through_switch,
            hotkey_button,
            hotkey_draft: RefCell::new(init.pause_hotkey),
            pause_ctl,
            syncing: RefCell::new(false),
            save_armed: Rc::new(Cell::new(false)),
            reset_button,
            done_button,
            section_headers,
            row_labels,
        });

        // 应用当前（已保存的）主题
        sw.apply_window_theme();

        // Esc：优先取消正在录制的快捷键，否则关闭设置窗口
        // （捕获阶段，抢在获得焦点的控件之前处理）
        {
            let handle = Rc::clone(&sw);
            let key_controller = gtk::EventControllerKey::new();
            key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
            key_controller.connect_key_pressed(move |_, keyval, _, _| {
                if keyval != gtk::gdk::Key::Escape {
                    return glib::Propagation::Proceed;
                }
                if handle.pause_ctl.capture.swap(false, Ordering::SeqCst) {
                    // 录制中：恢复原快捷键，不关窗口
                    let draft = *handle.hotkey_draft.borrow();
                    handle.hotkey_button.set_label(&hotkey_label(&draft));
                } else {
                    handle.window.hide();
                }
                glib::Propagation::Stop
            });
            sw.window.add_controller(key_controller);
        }

        // 窗口隐藏时立即落盘。关闭路径有四条（✖ / Esc / 完成 / 合成器关闭），统一在
        // 这里兜底，免得改完设置马上关窗、最后一次改动还压在防抖计时器里。
        {
            let sw = Rc::clone(&sw);
            let window = sw.window.clone();
            window.connect_visible_notify(move |w| {
                if !w.is_visible() {
                    sw.flush_save();
                }
            });
        }

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

        // 鼠标穿透开关 → 实时应用
        {
            let sw = Rc::clone(&sw);
            let switch = sw.click_through_switch.clone();
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
        let click_through = self.click_through_switch.is_active();

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
            s.click_through = click_through;

            (theme_changed, lang_changed, autostart_changed)
        };

        // 内存里的设置已是最新（悬浮层立即生效），落盘则防抖合并
        self.schedule_save();

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

    /// 安排一次防抖写盘：已有计时器在跑就什么都不做——它触发时会重新读取设置，
    /// 因此中间发生的改动同样会被写进去，不需要重复排队。
    fn schedule_save(&self) {
        if self.save_armed.replace(true) {
            return;
        }
        let settings = self.settings.clone();
        let armed = self.save_armed.clone();
        glib::timeout_add_local_once(SAVE_DEBOUNCE, move || {
            // 若期间已被 flush_save 落盘，这里就不再重复写
            if armed.replace(false) {
                settings.read().unwrap().save();
            }
        });
    }

    /// 立即写盘，跳过防抖。关闭设置窗口与退出程序时调用，避免防抖窗口内
    /// 的最后一次改动还没落盘就没了。
    pub fn flush_save(&self) {
        if self.save_armed.replace(false) {
            self.settings.read().unwrap().save();
        }
    }

    /// 控件变化统一入口：忽略程序化修改（`syncing`），实时应用所有设置。
    fn on_change(&self) {
        if *self.syncing.borrow() {
            return;
        }
        self.apply();
    }

    /// 把 `s` 的值写入各控件，期间抑制信号（不触发实时应用）。
    ///
    /// [`Self::sync_from_settings`]（载入已保存的设置）与 [`Self::reset`]（载入默认值）
    /// 共用这里；新增设置项时只需在这里补一行。
    fn load_into_controls(&self, s: &Settings) {
        *self.syncing.borrow_mut() = true;
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
        self.click_through_switch.set_active(s.click_through);
        *self.hotkey_draft.borrow_mut() = s.pause_hotkey;
        self.hotkey_button.set_label(&hotkey_label(&s.pause_hotkey));
        *self.syncing.borrow_mut() = false;
    }

    /// 把控件刷新为当前已保存的设置值，打开窗口时调用（不触发实时应用）。
    fn sync_from_settings(&self) {
        let s = self.settings.read().unwrap().clone();
        self.load_into_controls(&s);
    }

    /// 把控件改回默认值并实时应用。
    fn reset(&self) {
        self.load_into_controls(&Settings::default());
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
        self.title_label.set_label(s.title);
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
