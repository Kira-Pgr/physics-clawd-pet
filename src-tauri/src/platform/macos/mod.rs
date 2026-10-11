//! macOS 实现，文件结构和 windows/ 一一对应。

mod assist;
mod desktop;
mod input;

use objc2::msg_send;
use objc2::runtime::NSObjectProtocol;
use objc2::sel;
use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
use objc2_core_foundation::{CFArray, CFLocale, CFRetained, CFString};
use tauri::{ActivationPolicy, Manager, WebviewWindow};

use crate::lang::Lang;

pub use assist::{decode_system_codepage, http_get, idle_ms, pick_ics};
pub use desktop::{
    cursor_pos, frame_bounds, fullscreen_on_primary, primary_work_area, visible_windows, window_for_pids,
    window_scale,
};
pub use input::{input_permission, request_input_permission, InputProbe};

/// 只允许一只宠物：第二个实例直接退出（返回 false）。
/// 锁住临时目录（每个用户各一个）里的一个文件；文件故意不关，进程退出（包括崩溃）时系统自动解锁。
/// 文件打不开就照常运行：宁可多开也别开不起来。
pub fn single_instance() -> bool {
    lock_file(&std::env::temp_dir().join("PhysicsClawdPet.SingleInstance.lock"))
}

fn lock_file(path: &std::path::Path) -> bool {
    let Ok(file) = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(path) else {
        return true;
    };
    let locked = file.try_lock().is_ok();
    std::mem::forget(file);
    locked
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

/// 系统设置里"首选语言"的第一个。网页里的 navigator.language 也是从这儿来的，两边一致。
/// （图形界面启动的程序没有 LANG 环境变量，不能看它。）
pub fn ui_language() -> Lang {
    let first = CFLocale::preferred_languages()
        .map(|list| unsafe { CFRetained::cast_unchecked::<CFArray<CFString>>(list) })
        .and_then(|list| list.get(0))
        .map(|tag| tag.to_string())
        .unwrap_or_default();
    from_tag(&first)
}

/// "zh-Hans-CN"、"ja-JP"、"en-US" 这种语言标签：只看主语言（简体繁体都算中文）
fn from_tag(tag: &str) -> Lang {
    match tag.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase().as_str() {
        "zh" => Lang::Zh,
        "ja" => Lang::Ja,
        _ => Lang::En,
    }
}

/// 本进程往上的祖先进程（父、祖父……），用来找是哪个窗口在跑 Claude Code
pub fn ancestor_pids() -> Vec<u32> {
    let mut out = Vec::new();
    let mut pid = std::process::id();
    for _ in 0..16 {
        match parent_pid(pid) {
            Some(pp) if pp != 0 && pp != pid && !out.contains(&pp) => {
                out.push(pp);
                pid = pp;
            }
            _ => break,
        }
    }
    out
}

fn parent_pid(pid: u32) -> Option<u32> {
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
    let n = unsafe {
        libc::proc_pidinfo(pid as libc::c_int, libc::PROC_PIDTBSDINFO, 0, &mut info as *mut _ as *mut libc::c_void, size)
    };
    (n == size).then_some(info.pbi_ppid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_tag_maps_primary_language() {
        assert_eq!(from_tag("ja-JP"), Lang::Ja);
        assert_eq!(from_tag("zh-Hans-CN"), Lang::Zh);
        assert_eq!(from_tag("zh-Hant-TW"), Lang::Zh);
        assert_eq!(from_tag("zh_CN"), Lang::Zh);
        assert_eq!(from_tag("en-US"), Lang::En);
        assert_eq!(from_tag("ko-KR"), Lang::En); // 韩文归到英文
        assert_eq!(from_tag(""), Lang::En);
    }

    #[test]
    fn first_ancestor_is_the_parent() {
        assert_eq!(ancestor_pids().first().copied(), Some(std::os::unix::process::parent_id()));
    }

    #[test]
    fn second_lock_on_the_same_file_fails() {
        let path = std::env::temp_dir().join(format!("clawd-lock-test-{}", std::process::id()));
        assert!(lock_file(&path));
        assert!(!lock_file(&path), "同一个文件第二次锁不上");
        let _ = std::fs::remove_file(path);
    }
}
