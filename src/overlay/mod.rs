//! 悬浮组件 / Wayland UI：layer-shell 悬浮窗口 + 按键胶囊。
//!
//! 用 GTK4 + gtk4-layer-shell 做一个不抢焦点、覆盖在其它窗口之上的 overlay。
//! 胶囊最新的在右、旧的往左滑，位置（默认右上）、显示时长、最大数量、配色等
//! 均可在 [`crate::config`] 里调整。

mod renderer;
mod wayland;

pub use renderer::Overlay;
