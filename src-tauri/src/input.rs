//! 输入框检测：找出当前有键盘焦点的文本输入框，让宠物别挡着它（怎么找见 platform 的 InputProbe）。
//! 单独一个线程：跨进程的无障碍接口调用偶尔会慢，不能拖住观察线程。

use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::platform::{self, InputProbe};
use crate::watcher::{Shared, OVERLAY};

#[derive(Serialize, Clone, PartialEq)]
struct ZoneMsg {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

pub fn spawn(app: AppHandle, shared: Shared, overlay_id: isize) {
    thread::Builder::new()
        .name("input-watcher".into())
        .spawn(move || run(app, shared, overlay_id))
        .expect("failed to spawn input watcher");
}

fn run(app: AppHandle, shared: Shared, overlay_id: isize) {
    let probe = InputProbe::new();
    let mut last: Option<Option<ZoneMsg>> = None;
    loop {
        let (hidden, resync) = {
            let mut s = shared.lock().unwrap();
            (s.manual_hidden, std::mem::take(&mut s.input_resync))
        };
        if resync {
            last = None;
        }
        let zone = if hidden {
            None
        } else {
            platform::primary_work_area().and_then(|area| {
                let scale = platform::window_scale(overlay_id);
                probe.focused_input(overlay_id, area).map(|r| ZoneMsg {
                    left: (r.left - area.left) as f64 / scale,
                    top: (r.top - area.top) as f64 / scale,
                    right: (r.right - area.left) as f64 / scale,
                    bottom: (r.bottom - area.top) as f64 / scale,
                })
            })
        };
        if last.as_ref() != Some(&zone) {
            let _ = app.emit_to(OVERLAY, "desk-input", zone.clone());
            last = Some(zone);
        }
        thread::sleep(Duration::from_millis(250));
    }
}
