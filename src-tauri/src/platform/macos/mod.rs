//! macOS 实现。还在移植中：没做的先占位（功能关掉，但能编译、能跑），文件结构和 windows/ 一一对应。

mod assist;
mod desktop;
mod input;

use objc2::msg_send;
use objc2::runtime::NSObjectProtocol;
use objc2::sel;
use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
use tauri::{ActivationPolicy, Manager, WebviewWindow};

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

/// 我们自己窗口的 CGWindowID（= NSWindow.windowNumber）
pub fn window_id(win: &WebviewWindow) -> tauri::Result<isize> {
    // 指针不能跨线程传，先转成整数；NSWindow 只能在主线程碰
    let ns_window = win.ns_window()? as usize;
    Ok(dispatch2::run_on_main(move |_| unsafe { &*(ns_window as *const NSWindow) }.windowNumber()))
}

/// 覆盖层显示之前调一次：
/// - 程序坞里不显示图标、也没有菜单栏（宠物只有托盘图标），相当于 Windows 上不进任务栏
/// - 每个桌面（Space）都有它、切桌面时不跟着滑走、Cmd+` 切窗口时跳过它
/// - 故意不加 FullScreenAuxiliary：别的程序全屏时（在它自己的 Space 里）覆盖层不出现
/// - 点宠物不会把程序切到前台、不抢焦点（相当于 Windows 的 WS_EX_NOACTIVATE）
pub fn prepare_overlay(win: &WebviewWindow) -> tauri::Result<()> {
    win.app_handle().set_activation_policy(ActivationPolicy::Accessory)?;
    let ns_window = win.ns_window()? as usize;
    dispatch2::run_on_main(move |_| {
        let w = unsafe { &*(ns_window as *const NSWindow) };
        w.setCollectionBehavior(
            w.collectionBehavior()
                | NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        // 不激活的 NSPanel 内部用的就是这个开关（窗口服务器那边的标记：点它不激活程序）。
        // 私有方法，以后的系统没有了就退回原样：功能都正常，只是点一下会抢焦点。
        if w.respondsToSelector(sel!(_setPreventsActivation:)) {
            let _: () = unsafe { msg_send![w, _setPreventsActivation: true] };
        }
    });
    Ok(())
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
