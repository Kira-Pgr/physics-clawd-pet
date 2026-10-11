//! 焦点输入框（Win32）。
//! 先问 UI Automation 焦点控件（Win32、WinUI、Chrome、Electron 都支持），
//! 拿不到或者是铺满屏幕的大文档时，退回到系统插入符（GetGUIThreadInfo）附近的一小块。
//! 全部只读，不碰任何窗口。

use std::mem::size_of;

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, UIA_DocumentControlTypeId, UIA_EditControlTypeId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
};

use crate::desktop::Rect;

/// 在哪个线程用就在哪个线程建（里面初始化了 COM）
pub struct InputProbe {
    uia: Option<IUIAutomation>,
}

impl InputProbe {
    pub fn new() -> Self {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let uia = unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }.ok();
        InputProbe { uia }
    }

    /// 焦点输入框（屏幕物理像素）
    pub fn focused_input(&self, skip: isize, area: Rect) -> Option<Rect> {
        let fg = unsafe { GetForegroundWindow() };
        if fg.is_invalid() || fg.0 as isize == skip {
            return None;
        }
        // 超过工作区一半的"输入框"是编辑器、终端、整个网页：躲不开，只躲插入符附近
        let big = |r: &Rect| {
            (r.width() as i64) * (r.height() as i64) * 2 > (area.width() as i64) * (area.height() as i64)
        };
        if let Some(r) = self.uia_focus() {
            if !big(&r) {
                return Some(r);
            }
        }
        caret_rect(fg).map(|c| Rect {
            left: c.left - 160,
            top: c.top - 24,
            right: c.right + 160,
            bottom: c.bottom + 24,
        })
    }

    fn uia_focus(&self) -> Option<Rect> {
        let uia = self.uia.as_ref()?;
        unsafe {
            let el = uia.GetFocusedElement().ok()?;
            let ct = el.CurrentControlType().ok()?;
            if ct != UIA_EditControlTypeId && ct != UIA_DocumentControlTypeId {
                return None;
            }
            if el.CurrentIsOffscreen().map(|b| b.as_bool()).unwrap_or(false) {
                return None;
            }
            let r: Rect = el.CurrentBoundingRectangle().ok()?.into();
            (r.width() > 0 && r.height() > 0).then_some(r)
        }
    }
}

/// 前台线程的系统插入符（屏幕坐标）。很多现代程序不创建系统插入符，那就是 None。
fn caret_rect(fg: HWND) -> Option<Rect> {
    unsafe {
        let tid = GetWindowThreadProcessId(fg, None);
        let mut info = GUITHREADINFO { cbSize: size_of::<GUITHREADINFO>() as u32, ..Default::default() };
        GetGUIThreadInfo(tid, &mut info).ok()?;
        if info.hwndCaret.is_invalid() {
            return None;
        }
        let rc = info.rcCaret;
        let mut tl = POINT { x: rc.left, y: rc.top };
        let mut br = POINT { x: rc.right, y: rc.bottom };
        if !ClientToScreen(info.hwndCaret, &mut tl).as_bool() || !ClientToScreen(info.hwndCaret, &mut br).as_bool() {
            return None;
        }
        let r = Rect { left: tl.x, top: tl.y, right: br.x.max(tl.x + 1), bottom: br.y.max(tl.y + 1) };
        Some(r)
    }
}
