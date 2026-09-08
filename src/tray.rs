//! 系统托盘图标（StatusNotifierItem）：在应用栏显示一个小图标，右键菜单可退出。
//!
//! 基于 [`ksni`]（freedesktop StatusNotifierItem 的 Rust 实现，走 DBus）。图标直接用
//! `icon/icon.svg`：quickshell 会按 `IconName` + `IconThemePath` 拼成文件路径加载。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ksni::menu::StandardItem;
use ksni::{MenuItem, Tray, TrayMethods};

/// 托盘图标。`quit` 被「退出」菜单项置位，主循环检测到后退出程序。
pub struct ShowkeyTray {
    quit: Arc<AtomicBool>,
}

impl Tray for ShowkeyTray {
    fn id(&self) -> String {
        "showkey".into()
    }

    fn title(&self) -> String {
        "showkey".into()
    }

    /// 带扩展名的文件名：quickshell 会把它拼到 `icon_theme_path` 下按路径加载。
    fn icon_name(&self) -> String {
        "icon.svg".into()
    }

    fn icon_theme_path(&self) -> String {
        format!("{}/icon", env!("CARGO_MANIFEST_DIR"))
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![StandardItem {
            label: "退出".into(),
            activate: Box::new(|tray: &mut Self| {
                tray.quit.store(true, Ordering::SeqCst);
            }),
            ..Default::default()
        }
        .into()]
    }
}

/// 在已有的 tokio runtime 里启动托盘，返回的 `Handle` 需保持存活。
///
/// 失败（如无 DBus session / 无 StatusNotifierWatcher）只告警并返回 `None`，
/// 不影响按键显示功能。
pub async fn run(quit: Arc<AtomicBool>) -> Option<ksni::Handle<ShowkeyTray>> {
    let tray = ShowkeyTray { quit };
    match tray.spawn().await {
        Ok(handle) => Some(handle),
        Err(err) => {
            eprintln!("托盘图标启动失败（继续运行，仅不显示图标）: {err}");
            None
        }
    }
}
