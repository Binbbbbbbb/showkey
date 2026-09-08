//! UI 绘制 / 胶囊管理：文字、背景、圆角，以及胶囊的新增和过期。

use gtk4 as gtk;
use gtk::glib;
use gtk::prelude::*;

use crate::config::{BORDER_RADIUS, CHIP_ALPHA, DISPLAY_DURATION, MAX_CHIPS};
use super::wayland::build_window;

/// 根据配置生成全局 CSS：透明窗口 + 胶囊样式。
pub(super) fn build_css() -> String {
    format!(
        r#"
window {{
    background-color: transparent;
}}
.key-chip {{
    background-color: rgba(0, 0, 0, {alpha});
    color: #ffffff;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: {radius}px;
    padding: 12px 24px;
    font-family: "JetBrainsMono Nerd Font", "JetBrains Mono", monospace;
    font-size: 20px;
    font-weight: bold;
}}
"#,
        alpha = CHIP_ALPHA,
        radius = BORDER_RADIUS,
    )
}

/// 悬浮窗口，内部管理一排按键胶囊。
pub struct Overlay {
    // 持有窗口引用，防止被提前销毁；胶囊清空时用于隐藏窗口
    window: gtk::ApplicationWindow,
    container: gtk::Box,
}

impl Overlay {
    /// 创建窗口并初始化为 layer-shell 悬浮层。
    pub fn build(app: &gtk::Application) -> Self {
        let (window, container) = build_window(app);
        Self { window, container }
    }

    /// 添加一个按键胶囊，最多 [`MAX_CHIPS`] 个，每个独立 [`DISPLAY_DURATION`] 后过期。
    pub fn push(&self, text: &str) {
        let label = gtk::Label::new(Some(text));
        label.add_css_class("key-chip");
        self.container.append(&label);

        // 超过上限就移除最旧的那个（最左边）
        while self.chip_count() > MAX_CHIPS {
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
        let container = self.container.clone();
        let window = self.window.clone();
        glib::timeout_add_local_once(DISPLAY_DURATION, move || {
            container.remove(&label);
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
