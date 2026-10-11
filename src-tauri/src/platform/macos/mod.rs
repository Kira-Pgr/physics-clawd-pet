//! macOS 实现。还在移植中：没做的先占位（功能关掉，但能编译、能跑），文件结构和 windows/ 一一对应。

mod assist;
mod desktop;
mod input;

use tauri::WebviewWindow;

use crate::lang::Lang;

pub use assist::{decode_system_codepage, http_get, idle_ms, pick_ics};
pub use desktop::{
    cursor_pos, frame_bounds, fullscreen_on_primary, primary_work_area, visible_windows, window_for_pids,
    window_scale,
};
pub use input::InputProbe;

// TODO(macOS)：没有单实例保护，可以开好几只
pub fn single_instance() -> bool {
    true
}

// TODO(macOS)：应该是覆盖层 NSWindow 的 windowNumber（= CGWindowID）
pub fn window_id(_win: &WebviewWindow) -> tauri::Result<isize> {
    Ok(0)
}

// TODO(macOS)：从 Finder 启动的程序没有 LANG，要改问 NSLocale.preferredLanguages
pub fn ui_language() -> Lang {
    match std::env::var("LANG") {
        Ok(l) if l.starts_with("zh") => Lang::Zh,
        Ok(l) if l.starts_with("ja") => Lang::Ja,
        _ => Lang::En,
    }
}

// TODO(macOS)：找不到祖先进程，Claude Code 气泡就不知道跟着哪个终端窗口
pub fn ancestor_pids() -> Vec<u32> {
    vec![]
}
