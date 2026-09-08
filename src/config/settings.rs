//! 集中管理程序的可调设置。

use std::time::Duration;

/// 同时最多显示的胶囊数量。
pub const MAX_CHIPS: usize = 5;

/// 每个胶囊的显示时长（过期时间）。
pub const DISPLAY_DURATION: Duration = Duration::from_millis(2500);

/// 悬浮层与屏幕边缘的间距（像素）。
pub const MARGIN: i32 = 24;

/// 胶囊之间的间距（像素）。
pub const SPACING: i32 = 8;

/// 胶囊圆角半径（像素）。
pub const BORDER_RADIUS: u32 = 20;

/// 胶囊背景透明度，0.0（全透明）~ 1.0（不透明）。
pub const CHIP_ALPHA: f32 = 0.85;
