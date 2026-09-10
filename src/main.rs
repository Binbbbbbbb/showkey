//! 程序入口：读键盘，把组合键显示成 Wayland 悬浮胶囊。
//!
//! 结构：主线程跑 GTK 悬浮窗口与设置窗口；后台线程跑 tokio，读 evdev 事件并把
//! 格式化好的显示字符串通过 channel 交给 UI 线程，同时托管系统托盘图标。

mod app;
mod config;
mod core;
mod input;
mod overlay;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, mpsc};
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;

use config::{Settings, SettingsHandle};
use core::Chip;
use overlay::Overlay;

fn main() {
    let keyboards = input::find_keyboards();
    if keyboards.is_empty() {
        eprintln!("没有找到可读取的键盘（可能需要 root 或加入 input 组）");
        std::process::exit(1);
    }

    // 注册内置的 Nerd Font（编译进二进制），没装 Nerd Font 的机器也能显示图标。
    // 必须在任何字体被解析之前调用。
    app::font::register();

    // 可调设置：从磁盘加载，跨线程共享（主线程读写，托盘菜单读语言）
    let settings: SettingsHandle = Arc::new(RwLock::new(Settings::load()));

    // 后台线程 → UI 线程的胶囊通道
    let (ui_tx, ui_rx) = mpsc::channel::<Chip>();
    // 退出 / 显示设置标志：托盘置位，主循环轮询检测
    let quit = Arc::new(AtomicBool::new(false));
    let show_settings = Arc::new(AtomicBool::new(false));
    // 暂停显示 + 快捷键录制的共享控制状态
    let pause_ctl = Arc::new(core::PauseControl::new());
    // 托盘菜单刷新通道（语言切换时刷新托盘菜单文案）
    let (refresh_tx, refresh_rx) = tokio::sync::mpsc::unbounded_channel();

    spawn_keyboard_thread(
        keyboards,
        ui_tx.clone(),
        quit.clone(),
        show_settings.clone(),
        settings.clone(),
        refresh_rx,
        pause_ctl.clone(),
    );
    // 指针（鼠标 / 触控板）用 libinput，独立线程阻塞循环
    spawn_pointer_thread(ui_tx);
    let ui_rx = std::rc::Rc::new(ui_rx);

    let app = gtk::Application::builder()
        .application_id("dev.showkey.Showkey")
        .build();

    app.connect_activate(move |app| {
        let app = app.clone();
        let overlay = std::rc::Rc::new(Overlay::build(&app, settings.clone()));
        let settings_window = app::settings_window::SettingsWindow::build(
            &app,
            settings.clone(),
            overlay.clone(),
            refresh_tx.clone(),
            pause_ctl.clone(),
        );
        let ui_rx = ui_rx.clone();
        let quit = quit.clone();
        let show_settings = show_settings.clone();
        let pause_ctl = pause_ctl.clone();

        // 主循环轮询 channel：把显示消息变成胶囊；响应托盘请求；检测退出标志
        glib::timeout_add_local(Duration::from_millis(16), move || {
            while let Ok(chip) = ui_rx.try_recv() {
                // 暂停时丢弃新胶囊（保留已经显示的，它们会自行淡出）
                if !pause_ctl.paused.load(Ordering::SeqCst) {
                    overlay.push(&chip);
                }
            }
            if show_settings.swap(false, Ordering::SeqCst) {
                settings_window.show();
            }
            if quit.load(Ordering::SeqCst) {
                // 退出前把还压在防抖计时器里的设置写盘
                settings_window.flush_save();
                app.quit();
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    });

    app.run();
}

/// 在后台线程起一个 tokio runtime，读键盘并把显示消息发到 `ui_tx`，同时启动托盘图标。
fn spawn_keyboard_thread(
    keyboards: Vec<evdev::Device>,
    ui_tx: mpsc::Sender<Chip>,
    quit: Arc<AtomicBool>,
    show_settings: Arc<AtomicBool>,
    settings: SettingsHandle,
    refresh_rx: tokio::sync::mpsc::UnboundedReceiver<()>,
    pause_ctl: Arc<core::PauseControl>,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("创建 tokio runtime 失败");
        rt.block_on(async move {
            // 托盘图标（SNI）：保持 Handle 存活；失败只告警，不影响按键显示
            let _tray = app::tray::run(quit, show_settings, settings.clone(), refresh_rx).await;

            // 键盘：事件先交给状态合并（组合键 / 修饰键）
            let (ktx, krx) = tokio::sync::mpsc::channel(256);

            for device in keyboards {
                let tx = ktx.clone();
                tokio::spawn(input::run_keyboard_listener(device, tx));
            }
            // 释放最后一个 sender：所有键盘断开后 channel 关闭，report 任务随之退出
            drop(ktx);

            input::report_keys(krx, ui_tx, settings, pause_ctl).await;
        });
    });
}

/// 在独立线程里跑 libinput，把指针事件（点击 / 滚轮 / 手势）发到 `ui_tx`。
fn spawn_pointer_thread(ui_tx: mpsc::Sender<Chip>) {
    std::thread::spawn(move || {
        input::run_pointer_listener(ui_tx);
    });
}
