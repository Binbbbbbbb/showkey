//! 设置窗口的界面文案（中 / 英）。`rows` 顺序与 `build` 里 add_row 的调用顺序一致。

use crate::config::Language;

/// 界面文案（中 / 英），`rows` 顺序与 `build` 里 add_row 的调用顺序一致。
pub(super) struct Strings {
    pub(super) title: &'static str,
    pub(super) sections: [&'static str; 4],
    pub(super) rows: [&'static str; 17],
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
                    "设置窗口主题",
                    "胶囊配色",
                    "语言",
                    "最新透明度 (%)",
                    "历史透明度 (%)",
                    "圆角 (px)",
                    "字体大小 (px)",
                    "位置",
                    "水平边距 (px)",
                    "垂直边距 (px)",
                    "间距 (px)",
                    "最大胶囊数",
                    "显示时长 (ms)",
                    "淡入淡出时长 (ms)",
                    "暂停快捷键",
                    "鼠标穿透",
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
                    "Settings window theme",
                    "Chip color",
                    "Language",
                    "Latest opacity (%)",
                    "History opacity (%)",
                    "Corner radius (px)",
                    "Font size (px)",
                    "Position",
                    "Horizontal margin (px)",
                    "Vertical margin (px)",
                    "Spacing (px)",
                    "Max chips",
                    "Display duration (ms)",
                    "Fade duration (ms)",
                    "Pause hotkey",
                    "Click-through",
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
