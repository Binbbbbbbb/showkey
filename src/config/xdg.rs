//! XDG 基础目录解析：按规范取配置 / 数据目录，并做 `HOME` 回退。
//!
//! 原先 `settings` / `autostart` / `tray` 各自抄了一遍「读环境变量 → 回退 HOME →
//! 拼路径」的兜底逻辑，集中到这里以免三处走样。

use std::path::PathBuf;

/// `$XDG_CONFIG_HOME`（回退 `~/.config`）；都取不到时返回 `None`。
pub fn config_dir() -> Option<PathBuf> {
    xdg_dir("XDG_CONFIG_HOME", ".config")
}

/// `$XDG_DATA_HOME`（回退 `~/.local/share`）；都取不到时返回 `None`。
pub fn data_dir() -> Option<PathBuf> {
    xdg_dir("XDG_DATA_HOME", ".local/share")
}

/// `$XDG_RUNTIME_DIR`（回退系统临时目录）。
///
/// 用于存放只在本次登录期间需要的临时文件（如内置字体的落盘副本）。这个变量按规范
/// 没有「主目录」形式的回退，所以拿不到时退到临时目录。
pub fn runtime_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

/// 取 `$var` 的值，未设置时回退 `~/fallback`。
///
/// 规范要求把「已设置但为空」视同未设置：否则 `PathBuf::from("")` 会拼出相对路径，
/// 配置就被写到当前工作目录去了。
fn xdg_dir(var: &str, fallback: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(fallback))
        })
}
