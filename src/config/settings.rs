//! 集中管理程序的可调设置：结构体、默认值，以及 TOML 持久化。

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::xdg;

/// 明 / 暗两种外观。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

/// 界面语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    Zh,
    En,
}

/// 悬浮层在屏幕上的锚定位置（贴哪两条边）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Position {
    BottomLeft,
    BottomRight,
    TopLeft,
    #[default]
    TopRight,
}

/// 暂停显示的快捷键组合。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hotkey {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub super_key: bool,
    /// 触发键的 Linux evdev 键码（`evdev::KeyCode::code()`）。
    pub key: u16,
}

impl Default for Hotkey {
    fn default() -> Self {
        // Ctrl + Alt + P（KEY_P = 25）
        Self {
            ctrl: true,
            shift: false,
            alt: true,
            super_key: false,
            key: 25,
        }
    }
}

/// 可调设置。
///
/// 容器级的 `#[serde(default)]` 让**所有**缺失字段都回落到 [`Settings::default()`]，
/// 因此新增设置项时只要在 `Default` 里给个值，旧配置文件就能继续读——不必再写
/// 字段级的 `#[serde(default = "...")]`（那是多余的，两者只能取其一）。
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
    /// 悬浮层在屏幕上的锚定位置。
    pub position: Position,
    /// 悬浮层与贴边水平方向的屏幕距离（像素）。
    pub margin_x: i32,
    /// 悬浮层与贴边垂直方向的屏幕距离（像素）。
    pub margin_y: i32,
    /// 胶囊之间的间距（像素）。
    pub spacing: i32,
    /// 胶囊圆角半径（像素）。
    pub border_radius: u32,
    /// 最新胶囊的背景透明度，0.0（全透明）~ 1.0（不透明）。
    pub chip_alpha: f32,
    /// 历史胶囊（非最新）的背景透明度，0.0 ~ 1.0。
    pub chip_alpha_history: f32,
    /// 胶囊字体大小（像素）。
    pub font_size: u32,
    /// 胶囊淡入 / 淡出的过渡时长（毫秒）。
    pub fade_duration_ms: u64,
    /// 暂停显示的快捷键组合。
    pub pause_hotkey: Hotkey,
    /// 是否开机自启（写入 `~/.config/autostart/showkey.desktop`）。
    pub autostart: bool,
    /// 鼠标穿透：开启后悬浮层不接收指针事件，点击会落到底层窗口。
    pub click_through: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            chip_theme: Theme::Dark,
            language: Language::Zh,
            max_chips: 5,
            display_duration_ms: 2750,
            position: Position::TopRight,
            margin_x: 24,
            margin_y: 24,
            spacing: 8,
            border_radius: 18,
            chip_alpha: 0.85,
            chip_alpha_history: 0.5,
            font_size: 20,
            fade_duration_ms: 150,
            pause_hotkey: Hotkey::default(),
            autostart: false,
            click_through: true,
        }
    }
}

impl Settings {
    /// 每个胶囊的显示时长，转成 [`Duration`] 方便计时器使用。
    pub fn display_duration(&self) -> Duration {
        Duration::from_millis(self.display_duration_ms)
    }

    /// 胶囊淡入 / 淡出的过渡时长，转成 [`Duration`] 方便计时器使用。
    pub fn fade_duration(&self) -> Duration {
        Duration::from_millis(self.fade_duration_ms)
    }

    /// 配置文件路径：`$XDG_CONFIG_HOME/showkey/config.toml`（回退 `~/.config/showkey/config.toml`）。
    pub fn config_path() -> Option<PathBuf> {
        Some(xdg::config_dir()?.join("showkey").join("config.toml"))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 锁住向后兼容：旧配置文件只有部分字段时，其余字段必须回落到默认值。
    ///
    /// 这依赖 `Settings` 容器级的 `#[serde(default)]`；若哪天有人在字段上又加了
    /// 自己的默认值、或把容器级属性删掉，这里就会失败。
    #[test]
    fn partial_toml_falls_back_to_defaults() {
        let s: Settings = toml::from_str("max_chips = 3").unwrap();
        assert_eq!(s.max_chips, 3, "显式给出的字段应被采用");
        assert_eq!(s.font_size, 20);
        assert_eq!(s.fade_duration_ms, 150);
        assert!(s.click_through);
        assert_eq!(s.margin_x, 24);
        assert_eq!(s.border_radius, 18);
    }

    /// 空配置（以及缺失的配置文件）等价于全默认。
    #[test]
    fn empty_toml_is_all_defaults() {
        let s: Settings = toml::from_str("").unwrap();
        let d = Settings::default();
        assert_eq!(s.max_chips, d.max_chips);
        assert_eq!(s.font_size, d.font_size);
        assert_eq!(s.fade_duration_ms, d.fade_duration_ms);
        assert_eq!(s.click_through, d.click_through);
        assert_eq!(s.pause_hotkey.key, d.pause_hotkey.key);
    }
}
