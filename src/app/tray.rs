//! 系统托盘图标（StatusNotifierItem）：在应用栏显示一个小图标，单击打开设置，
//! 右键菜单可显示设置 / 退出。
//!
//! 基于 [`ksni`]（freedesktop StatusNotifierItem 的 Rust 实现，走 DBus）。图标直接用
//! `icon/icon.svg`：quickshell 会按 `IconName` + `IconThemePath` 拼成文件路径加载。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use ksni::menu::StandardItem;
use ksni::{MenuItem, Tray, TrayMethods};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::config::{Language, SettingsHandle};

/// 托盘图标。`quit` 被「退出」置位；`show_settings` 被单击或「显示设置」置位，
/// 两者都由主循环轮询检测。
pub struct ShowkeyTray {
    quit: Arc<AtomicBool>,
    show_settings: Arc<AtomicBool>,
    settings: SettingsHandle,
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

    /// 单击图标 → 打开设置窗口。
    fn activate(&mut self, _x: i32, _y: i32) {
        self.show_settings.store(true, Ordering::SeqCst);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let lang = self.settings.read().unwrap().language;
        let (settings_label, quit_label) = match lang {
            Language::Zh => ("显示设置", "退出"),
            Language::En => ("Show Settings", "Quit"),
        };

        vec![
            StandardItem {
                label: settings_label.into(),
                activate: Box::new(|tray: &mut Self| {
                    tray.show_settings.store(true, Ordering::SeqCst);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: quit_label.into(),
                activate: Box::new(|tray: &mut Self| {
                    tray.quit.store(true, Ordering::SeqCst);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// 在已有的 tokio runtime 里启动托盘，返回的 `Handle` 需保持存活。
///
/// 失败（如无 DBus session / 无 StatusNotifierWatcher）只告警并返回 `None`，
/// 不影响按键显示功能。`refresh_rx` 收到信号时刷新菜单（用于语言切换后更新文案）。
pub async fn run(
    quit: Arc<AtomicBool>,
    show_settings: Arc<AtomicBool>,
    settings: SettingsHandle,
    mut refresh_rx: UnboundedReceiver<()>,
) -> Option<ksni::Handle<ShowkeyTray>> {
    let tray = ShowkeyTray {
        quit,
        show_settings,
        settings,
    };
    match tray.spawn().await {
        Ok(handle) => {
            // 菜单文案随语言切换刷新：update 会重调 menu() 并推送 DBus 信号
            let updater = handle.clone();
            tokio::spawn(async move {
                while refresh_rx.recv().await.is_some() {
                    let _ = updater.update(|_| {}).await;
                }
            });
            Some(handle)
        }
        Err(err) => {
            eprintln!("托盘图标启动失败（继续运行，仅不显示图标）: {err}");
            None
        }
    }
}
