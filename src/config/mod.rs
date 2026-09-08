//! 程序配置：可调设置的结构体、默认值，以及 TOML 持久化。

pub mod settings;

pub use settings::{Language, Settings, Theme};

/// 跨线程共享的设置句柄：主线程（GTK / 设置窗口）读写，托盘菜单读语言。
pub type SettingsHandle = std::sync::Arc<std::sync::RwLock<Settings>>;
