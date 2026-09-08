//! 集中管理程序的可调设置：结构体、默认值，以及 TOML 持久化。

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// 明 / 暗两种外观。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

/// 界面语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    Zh,
    En,
}

/// 可调设置。所有字段都有默认值，反序列化时缺失的字段用默认补齐。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// 界面主题（明 / 暗），只影响设置窗口与 GTK 控件外观。
    pub theme: Theme,
    /// 胶囊配色（明 / 暗），只影响悬浮胶囊的颜色。
    pub chip_theme: Theme,
    /// 界面语言（中文 / 英文）。
    pub language: Language,
    /// 同时最多显示的胶囊数量。
    pub max_chips: usize,
    /// 每个胶囊的显示时长（毫秒）。
    pub display_duration_ms: u64,
    /// 悬浮层与屏幕边缘的间距（像素）。
    pub margin: i32,
    /// 胶囊之间的间距（像素）。
    pub spacing: i32,
    /// 胶囊圆角半径（像素）。
    pub border_radius: u32,
    /// 胶囊背景透明度，0.0（全透明）~ 1.0（不透明）。
    pub chip_alpha: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Light,
            chip_theme: Theme::Dark,
            language: Language::Zh,
            max_chips: 5,
            display_duration_ms: 2500,
            margin: 24,
            spacing: 8,
            border_radius: 18,
            chip_alpha: 0.85,
        }
    }
}

impl Settings {
    /// 每个胶囊的显示时长，转成 [`Duration`] 方便计时器使用。
    pub fn display_duration(&self) -> Duration {
        Duration::from_millis(self.display_duration_ms)
    }

    /// 配置文件路径：`$XDG_CONFIG_HOME/showkey/config.toml`（回退 `~/.config/showkey/config.toml`）。
    pub fn config_path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
        Some(base.join("showkey").join("config.toml"))
    }

    /// 从磁盘加载；文件缺失或损坏则回退默认值。
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// 保存到磁盘（尽力而为，失败静默忽略）。
    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = toml::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }
}
