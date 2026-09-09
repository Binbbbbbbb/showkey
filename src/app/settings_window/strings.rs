//! 设置窗口的界面文案（中 / 英）。`rows` 顺序与 `build` 里 add_row 的调用顺序一致。

use crate::config::Language;

/// 界面文案（中 / 英），`rows` 顺序与 `build` 里 add_row 的调用顺序一致。
pub(super) struct Strings {
    pub(super) title: &'static str,
    pub(super) sections: [&'static str; 4],
    pub(super) rows: [&'static str; 16],
    pub(super) light: &'static str,
    pub(super) dark: &'static str,
    pub(super) positions: [&'static str; 4],
    pub(super) reset: &'static str,
    pub(super) done: &'static str,
    pub(super) record_hint: &'static str,
}

impl Strings {
    pub(super) fn for_lang(lang: Language) -> Self {
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
