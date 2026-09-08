//! 程序入口：读键盘，把组合键显示成 Wayland 悬浮胶囊。
//!
//! 结构：主线程跑 GTK 悬浮窗口；后台线程跑 tokio，读 evdev 事件并把
//! 格式化好的显示字符串通过 channel 交给 UI 线程。

mod config;
mod input;
mod overlay;
mod tray;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;

use overlay::Overlay;

fn main() {
    let keyboards = input::find_keyboards();
    if keyboards.is_empty() {
        eprintln!("没有找到可读取的键盘（可能需要 root 或加入 input 组）");
        std::process::exit(1);
    }

    // 后台线程 → UI 线程的显示字符串通道
    let (ui_tx, ui_rx) = mpsc::channel::<String>();
    // 退出标志：托盘菜单点「退出」后置位，主循环检测到后退出
    let quit = Arc::new(AtomicBool::new(false));
    spawn_keyboard_thread(keyboards, ui_tx, quit.clone());
    let ui_rx = std::rc::Rc::new(ui_rx);

    let app = gtk::Application::builder()
        .application_id("dev.showkey.Showkey")
        .build();

    app.connect_activate(move |app| {
        let app = app.clone();
        let overlay = std::rc::Rc::new(Overlay::build(&app));
        let ui_rx = ui_rx.clone();
        let quit = quit.clone();

        // 主循环轮询 channel，把新的显示字符串变成胶囊；检测到退出标志则退出
        glib::timeout_add_local(Duration::from_millis(16), move || {
            while let Ok(text) = ui_rx.try_recv() {
                overlay.push(&text);
            }
            if quit.load(Ordering::SeqCst) {
                app.quit();
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    });

    app.run();
}

/// 在后台线程起一个 tokio runtime，读键盘并把显示字符串发到 `ui_tx`，同时启动托盘图标。
fn spawn_keyboard_thread(
    keyboards: Vec<evdev::Device>,
    ui_tx: mpsc::Sender<String>,
    quit: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("创建 tokio runtime 失败");
        rt.block_on(async move {
            // 托盘图标（SNI）：保持 Handle 存活；失败只告警，不影响按键显示
            let _tray = tray::run(quit).await;

            let (ktx, krx) = tokio::sync::mpsc::channel(256);

            for device in keyboards {
                let tx = ktx.clone();
                tokio::spawn(input::run_keyboard_listener(device, tx));
            }
            // 释放最后一个 sender：所有键盘断开后 channel 关闭，report 任务随之退出
            drop(ktx);

            input::report_keys(krx, ui_tx).await;
        });
    });
}
