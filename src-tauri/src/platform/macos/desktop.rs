//! 桌面状态查询（macOS）。TODO(macOS)：全是占位，宠物看不见光标和别的窗口，只能在屏幕底边活动。

use crate::desktop::{Rect, WinInfo};

pub fn cursor_pos() -> Option<(i32, i32)> {
    None
}

pub fn primary_work_area() -> Option<Rect> {
    None
}

pub fn window_scale(_id: isize) -> f64 {
    1.0
}

pub fn frame_bounds(_id: isize) -> Option<Rect> {
    None
}

pub fn visible_windows(_skip: isize) -> Vec<WinInfo> {
    vec![]
}

pub fn window_for_pids(_pids: &[u32]) -> Option<isize> {
    None
}

pub fn fullscreen_on_primary(_skip: isize) -> bool {
    false
}
