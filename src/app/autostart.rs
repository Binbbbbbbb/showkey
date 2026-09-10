//! 开机自启：在 `~/.config/autostart/` 下写入 / 删除 `showkey.desktop`。

use std::path::PathBuf;

/// 自启桌面文件路径：`$XDG_CONFIG_HOME/autostart/showkey.desktop`（回退
/// `~/.config/autostart/showkey.desktop`）。
fn autostart_path() -> Option<PathBuf> {
    Some(crate::config::xdg::config_dir()?.join("autostart").join("showkey.desktop"))
}

/// 按 `enabled` 写入或删除自启文件（尽力而为，失败静默忽略）。
pub fn apply(enabled: bool) {
    let Some(path) = autostart_path() else {
        return;
    };
    if !enabled {
        let _ = std::fs::remove_file(path);
        return;
    }

    let Some(dir) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }

    // Exec 指向当前运行的可执行文件；取不到则回退 `~/.local/bin/showkey`
    let exec = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local/bin/showkey"))
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "showkey".to_string())
        });

    let content = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=showkey\n\
         Comment=显示按键组合的悬浮胶囊\n\
         Exec={exec}\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n"
    );
    let _ = std::fs::write(path, content);
}
