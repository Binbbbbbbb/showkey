//! 通过 libinput 读取鼠标 / 触控板事件（点击、滚轮、手势），显示成悬浮胶囊。
//!
//! libinput 直接给出高层事件：轻点 / 物理点击（含 clickpad 右下角 = 右键）都变成
//! 按钮事件，双指滚动变成滚动事件，3/4 指滑动变成手势事件（带手指数与方向），
//! 因此无需再手写手势识别。

use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, OwnedFd};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use input::event::device::DeviceEvent;
use input::event::gesture::{
    GestureEndEvent, GestureEvent, GestureEventCoordinates, GestureEventTrait, GestureSwipeEvent,
};
use input::event::pointer::{Axis, ButtonState, PointerEvent, PointerScrollEvent};
use input::event::EventTrait;
use input::{Event, Libinput, LibinputInterface};

use crate::display::{Accent, Chip};

use super::keymap::SUPER_ICON;

/// 双击判定窗口。
const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(300);
/// 滚动结算的静止时长：最后一次滚动事件过去这么久后，把累计滚动显示成单个胶囊。
const SCROLL_DEBOUNCE: Duration = Duration::from_millis(150);

/// Linux 鼠标按键码（libinput 的 `button()` 直接返回这些值）。
const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;

/// 鼠标图标（左 / 右 / 中键都用它，靠颜色区分）。
const MOUSE_ICON: &str = "\u{F037D}";
/// 滚轮向上（`nf-md-mouse_move_up`，鼠标 + 上箭头）。
const SCROLL_UP_ICON: &str = "\u{F1551}";
/// 滚轮向下（`nf-md-mouse_move_down`，鼠标 + 下箭头）。
const SCROLL_DOWN_ICON: &str = "\u{F1550}";

/// libinput 打开设备文件时回调到这里（按 libinput 要求由调用方打开，便于权限控制）。
struct Interface;

impl LibinputInterface for Interface {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<OwnedFd, i32> {
        use libc::{O_ACCMODE, O_RDONLY, O_RDWR, O_WRONLY};
        OpenOptions::new()
            .custom_flags(flags)
            .read((flags & O_ACCMODE) == O_RDONLY || (flags & O_ACCMODE) == O_RDWR)
            .write((flags & O_ACCMODE) == O_WRONLY || (flags & O_ACCMODE) == O_RDWR)
            .open(path)
            .map(|file| file.into())
            .map_err(|err| err.raw_os_error().unwrap_or(-1))
    }

    fn close_restricted(&mut self, fd: OwnedFd) {
        // OwnedFd 的 Drop 会关闭文件描述符
        drop(fd);
    }
}

/// 指针事件 → 胶囊的跨事件状态。
struct PointerState {
    /// 上一次按下的按键码与时间（双击判定）。
    last_press: Option<(u32, Instant)>,
    /// 当前滑动的累计位移与手指数。
    swipe_fingers: u32,
    swipe_dx: f64,
    swipe_dy: f64,
    /// 滚动的累计位移（正 = 下、负 = 上）。
    scroll_accum: f64,
    /// 最后一次滚动事件的时间（有未结算的滚动时存在）。
    last_scroll: Option<Instant>,
}

impl PointerState {
    fn new() -> Self {
        Self {
            last_press: None,
            swipe_fingers: 0,
            swipe_dx: 0.0,
            swipe_dy: 0.0,
            scroll_accum: 0.0,
            last_scroll: None,
        }
    }

    /// 把一条 libinput 事件映射成胶囊（需要跨事件的滚动 / 滑动状态）。
    fn handle(&mut self, event: Event) -> Option<Chip> {
        match event {
            // 设备加入：开启轻点（tap-to-click）。libinput 默认关闭，不开启则触控板轻点不产生按钮事件
            Event::Device(DeviceEvent::Added(event)) => {
                let mut device = event.device();
                let _ = device.config_tap_set_enabled(true);
                None
            }
            // 按钮：轻点 / 物理点击（libinput 已区分 clickpad 分区与双击计数）
            Event::Pointer(PointerEvent::Button(button)) => {
                if button.button_state() != ButtonState::Pressed {
                    return None;
                }
                let accent = match button.button() {
                    BTN_LEFT => Accent::Left,
                    BTN_RIGHT => Accent::Right,
                    BTN_MIDDLE => Accent::Middle,
                    _ => return None,
                };
                let now = Instant::now();
                let double = matches!(
                    self.last_press,
                    Some((code, t)) if code == button.button() && now.duration_since(t) <= DOUBLE_CLICK_WINDOW
                );
                self.last_press = Some((button.button(), now));
                let text = if double {
                    format!("{MOUSE_ICON} *2")
                } else {
                    MOUSE_ICON.to_string()
                };
                Some(Chip::mouse(text, accent))
            }
            // 鼠标滚轮 / 触控板双指滚动：只累计，等静止后由 flush_scroll 结算成单个胶囊
            Event::Pointer(PointerEvent::ScrollWheel(wheel)) => {
                self.scroll_accum += wheel.scroll_value_v120(Axis::Vertical);
                self.last_scroll = Some(Instant::now());
                None
            }
            Event::Pointer(PointerEvent::ScrollFinger(finger)) => {
                if finger.has_axis(Axis::Vertical) {
                    self.scroll_accum += finger.scroll_value(Axis::Vertical);
                    self.last_scroll = Some(Instant::now());
                }
                None
            }
            // 3/4 指滑动：Begin 记录、Update 累计位移、End 判定方向（只发一个胶囊）
            Event::Gesture(GestureEvent::Swipe(swipe)) => match swipe {
                GestureSwipeEvent::Begin(begin) => {
                    self.swipe_fingers = begin.finger_count() as u32;
                    self.swipe_dx = 0.0;
                    self.swipe_dy = 0.0;
                    None
                }
                GestureSwipeEvent::Update(update) => {
                    self.swipe_dx += update.dx();
                    self.swipe_dy += update.dy();
                    None
                }
                GestureSwipeEvent::End(end) => {
                    if end.cancelled() {
                        None
                    } else {
                        swipe_chip(self.swipe_fingers, self.swipe_dx, self.swipe_dy)
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// 滚动静止一段时间后，把累计滚动结算成单个胶囊（方向 = 累计方向）。
    fn flush_scroll(&mut self) -> Option<Chip> {
        let t = self.last_scroll?;
        if t.elapsed() < SCROLL_DEBOUNCE {
            return None;
        }
        self.last_scroll = None;
        let accum = self.scroll_accum;
        self.scroll_accum = 0.0;
        if accum > 0.0 {
            Some(Chip::mouse(SCROLL_DOWN_ICON.to_string(), Accent::Scroll))
        } else if accum < 0.0 {
            Some(Chip::mouse(SCROLL_UP_ICON.to_string(), Accent::Scroll))
        } else {
            None
        }
    }
}

/// 根据手指数和累计位移判定滑动方向，映射成对应的 Super + 字母组合。
///
/// 约定（dx 向右为正、dy 向下为正）：
/// 三指：左=L、右=H、上=U、下=I；四指：仅上=D。
fn swipe_chip(fingers: u32, dx: f64, dy: f64) -> Option<Chip> {
    let letter = match fingers {
        3 => {
            if dx.abs() >= dy.abs() {
                if dx > 0.0 {
                    "H" // 右滑
                } else {
                    "L" // 左滑
                }
            } else if dy > 0.0 {
                "I" // 下滑
            } else {
                "U" // 上滑
            }
        }
        4 => {
            // 四指只有上滑
            if dy < 0.0 && dy.abs() >= dx.abs() {
                "D"
            } else {
                return None;
            }
        }
        _ => return None,
    };
    Some(Chip::key(format!("{SUPER_ICON} + {letter}")))
}

/// 在独立线程里跑 libinput 事件循环，把点击 / 滚轮 / 手势发到 `ui_tx`。
///
/// 阻塞循环，随进程结束而退出。
pub fn run_pointer_listener(ui_tx: mpsc::Sender<Chip>) {
    let mut input = match (|| -> Result<Libinput, ()> {
        let mut input = Libinput::new_with_udev(Interface);
        input.udev_assign_seat("seat0").map_err(|_| ())?;
        Ok(input)
    })() {
        Ok(input) => input,
        Err(()) => {
            eprintln!("无法初始化 libinput（指针显示不可用）");
            return;
        }
    };

    let mut state = PointerState::new();

    loop {
        // poll 带超时：既用于滚动 debounce，也避免无限阻塞
        let mut pfd = libc::pollfd {
            fd: input.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut pfd, 1, SCROLL_DEBOUNCE.as_millis() as i32) };

        if ready > 0 {
            if input.dispatch().is_err() {
                continue;
            }
            for event in &mut input {
                if let Some(chip) = state.handle(event) {
                    // 接收端已退出（程序准备结束）时停止监听
                    if ui_tx.send(chip).is_err() {
                        return;
                    }
                }
            }
        }

        // 滚动静止后结算成单个胶囊
        if let Some(chip) = state.flush_scroll() {
            if ui_tx.send(chip).is_err() {
                return;
            }
        }
    }
}
