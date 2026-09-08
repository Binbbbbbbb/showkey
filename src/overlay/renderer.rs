//! UI 绘制 / 胶囊管理：文字、背景、圆角，以及胶囊的新增和过期。

use gtk4 as gtk;
use gtk::glib;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, LayerShell};

use crate::config::{Position, Settings, SettingsHandle, Theme};
use super::wayland::build_window;

/// 根据设置生成全局 CSS：透明窗口 + 胶囊样式。
///
/// 胶囊配色由 `chip_theme` 决定（独立于界面主题）：黑暗 = 黑底白字，明亮 = 白底黑字。
pub(super) fn build_css(s: &Settings) -> String {
    let (bg, fg, border) = match s.chip_theme {
        Theme::Dark => (
            format!("rgba(0, 0, 0, {})", s.chip_alpha),
            "#ffffff".to_string(),
            "rgba(255, 255, 255, 0.2)".to_string(),
        ),
        Theme::Light => (
            format!("rgba(255, 255, 255, {})", s.chip_alpha),
            "#1a1a1a".to_string(),
            "rgba(0, 0, 0, 0.15)".to_string(),
        ),
    };

    format!(
        r#"
window {{
    background-color: transparent;
}}
.key-chip {{
    background-color: {bg};
    color: {fg};
    border: 1px solid {border};
    border-radius: {radius}px;
    padding: 12px 24px;
    font-family: "JetBrainsMono Nerd Font", "JetBrains Mono", monospace;
    font-size: 20px;
    font-weight: bold;
}}
"#,
        bg = bg,
        fg = fg,
        border = border,
        radius = s.border_radius,
    )
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

    /// 按当前设置重新应用胶囊样式（配色 / 圆角 / 透明度）、位置、边距与间距。
    pub fn apply_settings(&self) {
        let s = self.settings.read().unwrap();
        let css = build_css(&s);
        self.provider.load_from_data(&css);
        self.container.set_spacing(s.spacing);

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

    /// 添加一个按键胶囊，最多 `max_chips` 个，每个独立 `display_duration` 后过期。
    pub fn push(&self, text: &str) {
        let (max_chips, duration) = {
            let s = self.settings.read().unwrap();
            (s.max_chips, s.display_duration())
        };

        let label = gtk::Label::new(Some(text));
        label.add_css_class("key-chip");
        self.container.append(&label);

        // 超过上限就移除最旧的那个（最左边）
        while self.chip_count() > max_chips {
            if let Some(oldest) = self.container.first_child() {
                self.container.remove(&oldest);
            } else {
                break;
            }
        }

        // 有胶囊就确保窗口可见
        self.window.set_visible(true);

        // 每个胶囊独立计时，到点从容器移除；若清空则隐藏窗口，
        // 强制 layer-shell 表面重新映射，避免最后一枚胶囊的画面残留。
        // 若该胶囊已因超过上限被提前移除（parent 为空），则跳过，避免重复 remove。
        let container = self.container.clone();
        let window = self.window.clone();
        glib::timeout_add_local_once(duration, move || {
            if label.parent().is_some() {
                container.remove(&label);
            }
            if container.first_child().is_none() {
                window.set_visible(false);
            }
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
