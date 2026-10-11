//! Windows 实现（Win32）。

mod assist;
mod desktop;
mod input;

use std::collections::HashMap;

use tauri::WebviewWindow;

use crate::lang::Lang;

pub use assist::{decode_system_codepage, http_get, idle_ms, pick_ics};
pub use desktop::{
    cursor_pos, frame_bounds, fullscreen_on_primary, primary_work_area, visible_windows, window_for_pids,
    window_scale,
};
pub use input::{input_permission, request_input_permission, InputProbe};

/// 只允许一只宠物：第二个实例直接退出（返回 false）
pub fn single_instance() -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;
    // 句柄故意不关：进程活着它就一直占着
    let created = unsafe { CreateMutexW(None, false, w!(r"Local\PhysicsClawdPet.SingleInstance")) };
    created.is_ok() && unsafe { GetLastError() } != ERROR_ALREADY_EXISTS
}

/// 我们自己窗口的 HWND
pub fn window_id(win: &WebviewWindow) -> tauri::Result<isize> {
    Ok(win.hwnd()?.0 as isize)
}

/// 覆盖层要的都在建窗口时设好了（不进任务栏、不抢焦点）
pub fn prepare_overlay(_win: &WebviewWindow) -> tauri::Result<()> {
    Ok(())
}

pub fn ui_language() -> Lang {
    from_langid(unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() })
}

/// Windows 的 LANGID：低 10 位是主语言（0x04 = 中文，简体繁体都算；0x11 = 日文）
fn from_langid(id: u16) -> Lang {
    match id & 0x3ff {
        0x04 => Lang::Zh,
        0x11 => Lang::Ja,
        _ => Lang::En,
    }
}

/// 本进程往上的祖先进程（父、祖父……），用来找是哪个窗口在跑 Claude Code
pub fn ancestor_pids() -> Vec<u32> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    let mut parent: HashMap<u32, u32> = HashMap::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return vec![] };
        let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        if Process32FirstW(snap, &mut e).is_ok() {
            loop {
                parent.insert(e.th32ProcessID, e.th32ParentProcessID);
                if Process32NextW(snap, &mut e).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    let mut out = Vec::new();
    let mut pid = std::process::id();
    for _ in 0..16 {
        match parent.get(&pid) {
            Some(&pp) if pp != 0 && pp != pid && !out.contains(&pp) => {
                out.push(pp);
                pid = pp;
            }
            _ => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn langid_maps_primary_language() {
        assert_eq!(from_langid(0x0411), Lang::Ja); // ja-JP
        assert_eq!(from_langid(0x0804), Lang::Zh); // zh-CN
        assert_eq!(from_langid(0x0404), Lang::Zh); // zh-TW
        assert_eq!(from_langid(0x0c04), Lang::Zh); // zh-HK
        assert_eq!(from_langid(0x0409), Lang::En); // en-US
        assert_eq!(from_langid(0x0412), Lang::En); // ko-KR 归到英文
    }
}
