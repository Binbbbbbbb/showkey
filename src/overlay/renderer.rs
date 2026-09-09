//! UI 绘制 / 胶囊管理：文字、背景、圆角，以及胶囊的新增和过期。

use std::time::Duration;

use gtk4 as gtk;
use gtk::glib;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, LayerShell};

use crate::config::{Position, Settings, SettingsHandle, Theme};
use crate::core::{Accent, Chip};
use super::wayland::build_window;

/// 单个胶囊的最小宽度（像素）：满载时窗口宽度 = 最小宽度 × 最大胶囊数。
const MIN_CHIP_WIDTH: i32 = 72;

/// 淡入 / 淡出的步数（步进间隔由设置的 `fade_duration` 决定）。
const FADE_STEPS: u32 = 16;

/// 根据设置生成全局 CSS：透明窗口 + 胶囊样式。
///
/// 胶囊配色由 `chip_theme` 决定（独立于界面主题）：黑暗 = 黑底白字，明亮 = 白底黑字。
/// 最新胶囊与历史胶囊透明度不同：最新用 `chip_alpha`，历史用 `chip_alpha_history`。
pub(super) fn build_css(s: &Settings) -> String {
    // (最新底色, 历史底色, 前景, 边框, 左键色, 右键色, 中键色, 滚轮色)
    let (bg_new, bg_old, fg, border, left, right, middle, scroll) = match s.chip_theme {
        Theme::Dark => (
            format!("rgba(0, 0, 0, {})", s.chip_alpha),
            format!("rgba(0, 0, 0, {})", s.chip_alpha_history),
            "#ffffff".to_string(),
            "rgba(255, 255, 255, 0.2)".to_string(),
            "#64B5F6", // 左键 青蓝
            "#FFB74D", // 右键 黄
            "#EF5350", // 中键 红
            "#BA68C8", // 滚轮 紫
        ),
        Theme::Light => (
            format!("rgba(255, 255, 255, {})", s.chip_alpha),
            format!("rgba(255, 255, 255, {})", s.chip_alpha_history),
            "#1a1a1a".to_string(),
            "rgba(0, 0, 0, 0.15)".to_string(),
            "#1565C0", // 左键 青蓝（深）
            "#F57C00", // 右键 黄（深）
            "#C62828", // 中键 红（深）
            "#8E24AA", // 滚轮 紫（深）
        ),
    };

    format!(
        r#"
window {{
    background-color: transparent;
}}
.key-chip, .key-chip-old {{
    color: {fg};
    border: 1px solid {border};
    border-radius: {radius}px;
    padding: 12px 24px;
    font-family: "JetBrainsMono Nerd Font", "JetBrains Mono", monospace;
    font-size: {font_size}px;
    font-weight: bold;
}}
.key-chip {{
    background-color: {bg_new};
}}
.key-chip-old {{
    background-color: {bg_old};
}}
.accent-left {{ color: {left}; }}
.accent-right {{ color: {right}; }}
.accent-middle {{ color: {middle}; }}
.accent-scroll {{ color: {scroll}; }}
"#,
        bg_new = bg_new,
        bg_old = bg_old,
        fg = fg,
        border = border,
        left = left,
        right = right,
        middle = middle,
        scroll = scroll,
        radius = s.border_radius,
        font_size = s.font_size,
    )
}

/// 强调色类别 → CSS 类名（颜色在 [`build_css`] 里按主题定义）。
fn accent_class(accent: Accent) -> &'static str {
    match accent {
        Accent::Left => "accent-left",
        Accent::Right => "accent-right",
        Accent::Middle => "accent-middle",
        Accent::Scroll => "accent-scroll",
    }
}

/// 让 `widget` 的透明度从 `from` 渐变到 `to`，完成后执行 `done`。
///
/// 用固定间隔计时器分 [`FADE_STEPS`] 步推进，`duration` 为总过渡时长；只改 widget 的
/// `opacity` 属性，不影响 CSS 里定义的背景透明度（胶囊底色仍是 `chip_alpha` / `chip_alpha_history`）。
fn fade(
    widget: gtk::Widget,
    from: f64,
    to: f64,
    duration: Duration,
    done: impl FnOnce() + 'static,
) {
    widget.set_opacity(from);
    let step = (to - from) / f64::from(FADE_STEPS);
    // 步进间隔至少 1ms，避免设置成 0 时退化成忙轮询
    let interval = (duration / FADE_STEPS).max(Duration::from_millis(1));
    let mut done = Some(done);
    let mut i = 0u32;
    glib::timeout_add_local(interval, move || {
        i += 1;
        let opacity = (from + step * f64::from(i)).clamp(0.0, 1.0);
        widget.set_opacity(opacity);
        if i >= FADE_STEPS {
            if let Some(d) = done.take() {
                d();
            }
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

/// 淡入：从全透明渐变到不透明。
fn fade_in(widget: gtk::Widget, duration: Duration) {
    fade(widget, 0.0, 1.0, duration, || {});
}

/// 淡出：从当前透明度渐变到全透明，完成后执行 `done`（通常是移除胶囊）。
fn fade_out(widget: gtk::Widget, duration: Duration, done: impl FnOnce() + 'static) {
    let from = widget.opacity();
    fade(widget, from, 0.0, duration, done);
}

/// 悬浮窗口，内部管理一排按键胶囊。
pub struct Overlay {
    // 持有窗口引用，防止被提前销毁；胶囊清空时用于隐藏窗口
    window: gtk::ApplicationWindow,
    container: gtk::Box,
    // 全局 CSS 提供者：设置变化时重载数据即可热更新
    provider: gtk::CssProvider,
    settings: SettingsHandle,
}

impl Overlay {
    /// 创建窗口并初始化为 layer-shell 悬浮层。
    pub fn build(app: &gtk::Application, settings: SettingsHandle) -> Self {
        let (window, container) = build_window(app);

        // 注册一次全局 CSS provider，之后用 apply_settings 热更新内容
        let provider = gtk::CssProvider::new();
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        let overlay = Self {
            window,
            container,
            provider,
            settings,
        };
        overlay.apply_settings();
        overlay
    }

    /// 按当前设置重新应用胶囊样式（配色 / 圆角 / 透明度 / 字体）、位置、边距与间距。
    pub fn apply_settings(&self) {
        let s = self.settings.read().unwrap();
        let css = build_css(&s);
        self.provider.load_from_data(&css);
        self.container.set_spacing(s.spacing);

        // 满载宽度 = 最小宽度 × 最大数量，随「最大数量」动态变化（高度留自然高度，最小 1 避免空容器为 0）
        self.window
            .set_size_request(MIN_CHIP_WIDTH * s.max_chips as i32, 1);

        // 根据位置决定贴哪两条边（水平 + 垂直），并应用对应两边的边距
        let (h_edge, v_edge) = match s.position {
            Position::BottomLeft => (Edge::Left, Edge::Bottom),
            Position::BottomRight => (Edge::Right, Edge::Bottom),
            Position::TopLeft => (Edge::Left, Edge::Top),
            Position::TopRight => (Edge::Right, Edge::Top),
        };
        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            self.window.set_anchor(edge, false);
            self.window.set_margin(edge, 0);
        }
        self.window.set_anchor(h_edge, true);
        self.window.set_anchor(v_edge, true);
        self.window.set_margin(h_edge, s.margin_x);
        self.window.set_margin(v_edge, s.margin_y);
    }

    /// 添加一个胶囊，最多 `max_chips` 个，每个独立 `display_duration` 后过期。
    pub fn push(&self, chip: &Chip) {
        let (max_chips, duration, fade_duration) = {
            let s = self.settings.read().unwrap();
            (s.max_chips, s.display_duration(), s.fade_duration())
        };

        // 新胶囊是最新的：先把上一个最新的胶囊降级为历史样式（透明度不同）
        if let Some(newest) = self.container.last_child() {
            newest.remove_css_class("key-chip");
            newest.add_css_class("key-chip-old");
        }

        let label = gtk::Label::new(Some(&chip.text));
        label.add_css_class("key-chip");
        // 鼠标胶囊按按钮上色（键盘胶囊无强调色）
        if let Some(accent) = chip.accent {
            label.add_css_class(accent_class(accent));
        }
        self.container.append(&label);

        // 超过上限就移除最旧的那个（最左边）
        while self.chip_count() > max_chips {
            if let Some(oldest) = self.container.first_child() {
                self.container.remove(&oldest);
            } else {
                break;
            }
        }

        // 有胶囊就确保窗口可见，并淡入新胶囊
        self.window.set_visible(true);
        fade_in(label.clone().upcast(), fade_duration);

        // 每个胶囊独立计时，到点先淡出再移除；若清空则隐藏窗口，
        // 强制 layer-shell 表面重新映射，避免最后一枚胶囊的画面残留。
        // 若该胶囊已因超过上限被提前移除（parent 为空），则跳过，避免重复 remove。
        let container = self.container.clone();
        let window = self.window.clone();
        glib::timeout_add_local_once(duration, move || {
            if label.parent().is_none() {
                return;
            }
            fade_out(label.clone().upcast(), fade_duration, move || {
                if label.parent().is_some() {
                    container.remove(&label);
                }
                if container.first_child().is_none() {
                    window.set_visible(false);
                }
            });
        });
    }

    /// 当前胶囊数量。
    fn chip_count(&self) -> usize {
        let mut n: usize = 0;
        let mut child = self.container.first_child();
        while let Some(c) = child {
            n += 1;
            child = c.next_sibling();
        }
        n
    }
}
