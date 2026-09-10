//! 内置 Nerd Font：把**编译进二进制**的字体注册给 fontconfig，供悬浮层 CSS 的
//! `font-family` 命中。
//!
//! 这样用户无需自行安装 Nerd Font，图标也不会显示成方块。字体只嵌在二进制里，
//! 不随 `install.sh` 装进系统，因此不会影响用户的其它程序。
//!
//! fontconfig 只能按**文件路径**加载字体，所以启动时先把字节写到运行时目录
//! （已存在且大小一致就跳过，通常一次登录只写一次），再调 `FcConfigAppFontAddFile`
//! 注册到本进程。注册失败只告警：用户自己装了 Nerd Font 时一切照旧。

use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use crate::config::xdg;

/// 内置字体：JetBrainsMono Nerd Font Bold（正文 + 图标）。
///
/// 只打 Bold 一档，因为 CSS 里 `font-weight: bold` 是唯一的字重。
const FONT: &[u8] = include_bytes!("../../font/JetBrainsMonoNerdFont-Bold.ttf");

/// 落盘时用的文件名。
const FILE_NAME: &str = "JetBrainsMonoNerdFont-Bold.ttf";

/// 字体族名。**必须与 `overlay::renderer` 的 CSS 里 `font-family` 的首项一致**，
/// 否则 CSS 命不中这个字体（那边有对应的注释）。
pub const FAMILY: &str = "JetBrainsMono Nerd Font";

/// 把内置字体注册到本进程的 fontconfig。
///
/// 必须在任何字体被解析之前调用（`main` 里最早处），否则 Pango 已经建好字体表，
/// 之后再注册就来不及了。
pub fn register() {
    let Some(path) = materialize() else {
        eprintln!("内置字体无法写入运行时目录，图标可能显示为方块");
        return;
    };
    if !add_to_fontconfig(&path) {
        eprintln!("内置字体 {FAMILY} 注册失败，图标可能显示为方块");
    }
}

/// 把内置字体写到运行时目录，返回文件路径（fontconfig 只认路径）。
///
/// 文件已存在且大小一致就直接复用，避免每次启动都写 2.5 MB。
fn materialize() -> Option<PathBuf> {
    let dir = xdg::runtime_dir().join("showkey");
    std::fs::create_dir_all(&dir).ok()?;

    let path = dir.join(FILE_NAME);
    let up_to_date = std::fs::metadata(&path)
        .map(|meta| meta.len() == FONT.len() as u64)
        .unwrap_or(false);
    if !up_to_date {
        std::fs::write(&path, FONT).ok()?;
    }
    Some(path)
}

/// fontconfig 的 `FcConfig` 是不透明结构，只需要一个指针。
#[repr(C)]
struct FcConfig {
    _private: [u8; 0],
}

// fontconfig 的 `FcBool` 就是 `int`：0 = 失败，1 = 成功。
//
// 这几个声明必须放在**模块级**——`#[link]` 写在函数体内的 extern 块上不会被传给
// 链接器，符号会找不到。
#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcInit() -> c_int;
    fn FcConfigGetCurrent() -> *mut FcConfig;
    fn FcConfigAppFontAddFile(config: *mut FcConfig, file: *const c_char) -> c_int;
}

/// 调 `FcConfigAppFontAddFile` 把 `path` 加进当前 fontconfig 配置。返回是否成功。
fn add_to_fontconfig(path: &Path) -> bool {
    let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: 三个函数都是 fontconfig 的公开入口；`c_path` 活到调用结束，
    // `FcConfigGetCurrent` 返回的指针由 fontconfig 自己持有，无需释放。
    unsafe {
        FcInit();
        FcConfigAppFontAddFile(FcConfigGetCurrent(), c_path.as_ptr()) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 内置字体必须是 fontconfig 认可的有效字体——文件损坏 / 路径写错都会让
    /// `FcConfigAppFontAddFile` 返回失败。
    #[test]
    fn bundled_font_registers() {
        let path = materialize().expect("应能把内置字体写入运行时目录");
        assert!(add_to_fontconfig(&path), "fontconfig 应能加载内置字体");
    }
}
