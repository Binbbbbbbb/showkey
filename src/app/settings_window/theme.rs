//! 设置窗口自身的主题 CSS：按明 / 暗生成简洁调色板，含右上角关闭按钮。

use crate::config::Theme;

/// 按主题生成设置窗口的 CSS（含右上角关闭按钮样式）。
pub(super) fn window_css(theme: Theme) -> String {
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
}}
.settings-header {{
    padding: 14px 20px 10px 20px;
}}
.settings-title {{
    font-size: 15px;
    font-weight: 600;
}}
/* 内边距放在滚动内容上，滚动条才能贴住窗口右缘 */
.settings-content {{
    padding: 0 20px;
}}
/* 底部按钮栏固定在滚动区外 */
.settings-footer {{
    padding: 12px 20px 16px 20px;
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
