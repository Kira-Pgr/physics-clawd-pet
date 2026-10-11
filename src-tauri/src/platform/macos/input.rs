//! 焦点输入框（macOS）：问辅助功能（Accessibility）接口当前的焦点控件。
//! 文本框就躲整个框；铺满屏幕的大文档（编辑器、终端）和其他能打字的控件只躲插入点附近一小块。
//! 需要"系统设置 → 隐私与安全性 → 辅助功能"里的权限：没有就什么都不做，托盘菜单里能去打开（不主动弹窗）。
//! 全部只读，不碰任何窗口。

use std::ffi::c_void;
use std::ptr::{null_mut, NonNull};

use objc2_core_foundation::{CFBoolean, CFDictionary, CFRetained, CFString, CFType, CGPoint, CGRect, CGSize};

use super::desktop::{scale, to_rect};
use crate::desktop::Rect;

// AXUIElement.h / AXValue.h（ApplicationServices 框架）
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: &CFDictionary) -> u8;
    fn AXUIElementCreateSystemWide() -> *mut CFType;
    fn AXUIElementSetMessagingTimeout(element: &CFType, timeout_secs: f32) -> i32;
    fn AXUIElementCopyAttributeValue(element: &CFType, attribute: &CFString, value: *mut *mut CFType) -> i32;
    fn AXUIElementCopyParameterizedAttributeValue(
        element: &CFType,
        attribute: &CFString,
        parameter: &CFType,
        result: *mut *mut CFType,
    ) -> i32;
    fn AXValueGetValue(value: &CFType, value_type: u32, value_ptr: *mut c_void) -> u8;
    static kAXTrustedCheckOptionPrompt: &'static CFString;
}

const AX_VALUE_CGPOINT: u32 = 1;
const AX_VALUE_CGSIZE: u32 = 2;
const AX_VALUE_CGRECT: u32 = 3;

/// 整个框都要躲开的控件
const TEXT_ROLES: &[&str] = &["AXTextField", "AXTextArea", "AXComboBox"];

/// 用户给没给辅助功能权限（随时可能在系统设置里打开或关掉）
pub fn input_permission() -> Option<bool> {
    Some(unsafe { AXIsProcessTrusted() } != 0)
}

/// 托盘菜单里点了"躲开输入框"：系统会弹窗、并把本程序加进辅助功能列表（用户只要打开开关）；
/// 再直接打开那一页设置，免得系统这次不弹窗时什么都没发生。已经有权限时打开设置是为了能关掉它。
pub fn request_input_permission() {
    if input_permission() != Some(true) {
        let key = unsafe { kAXTrustedCheckOptionPrompt };
        let options = CFDictionary::<CFString, CFBoolean>::from_slices(&[key], &[CFBoolean::new(true)]);
        unsafe { AXIsProcessTrustedWithOptions(options.as_opaque()) };
    }
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

fn retained(ptr: *mut CFType) -> Option<CFRetained<CFType>> {
    // Copy/Create 开头的函数返回的已经 +1 了
    NonNull::new(ptr).map(|p| unsafe { CFRetained::from_raw(p) })
}

fn attr(element: &CFType, name: &str) -> Option<CFRetained<CFType>> {
    let mut out = null_mut();
    let err = unsafe { AXUIElementCopyAttributeValue(element, &CFString::from_str(name), &mut out) };
    if err != 0 {
        return None;
    }
    retained(out)
}

fn ax_value<T: Default>(value: &CFType, value_type: u32) -> Option<T> {
    let mut out = T::default();
    (unsafe { AXValueGetValue(value, value_type, &mut out as *mut T as *mut c_void) } != 0).then_some(out)
}

/// 在哪个线程用就在哪个线程建
pub struct InputProbe {
    system: Option<CFRetained<CFType>>,
}

impl InputProbe {
    pub fn new() -> Self {
        let system = retained(unsafe { AXUIElementCreateSystemWide() });
        // 要跨进程问别的程序：别的程序卡住时最多等 0.2 秒（默认要等 6 秒）
        if let Some(s) = &system {
            unsafe { AXUIElementSetMessagingTimeout(s, 0.2) };
        }
        InputProbe { system }
    }

    /// 焦点输入框（屏幕物理像素）。焦点永远不会在覆盖层上（点它不激活程序），用不到 skip。
    pub fn focused_input(&self, _skip: isize, area: Rect) -> Option<Rect> {
        if input_permission() != Some(true) {
            return None;
        }
        let focused = attr(self.system.as_ref()?, "AXFocusedUIElement")?;
        let s = scale();
        let role = attr(&focused, "AXRole").and_then(|r| r.downcast::<CFString>().ok()).map(|r| r.to_string());
        if role.as_deref().is_some_and(|r| TEXT_ROLES.contains(&r)) {
            // 超过工作区一半的"输入框"是编辑器、终端、整个网页：躲不开，只躲插入点附近
            if let Some(r) = frame(&focused).map(|f| to_rect(f, s)) {
                let big = (r.width() as i64) * (r.height() as i64) * 2 > (area.width() as i64) * (area.height() as i64);
                if r.width() > 0 && r.height() > 0 && !big {
                    return Some(r);
                }
            }
        }
        // 其他能打字的控件（网页里的可编辑区域之类）也有插入点；按钮、列表这些没有，就是 None
        let c = to_rect(caret(&focused)?, s);
        let (px, py) = ((160.0 * s) as i32, (24.0 * s) as i32);
        Some(Rect {
            left: c.left - px,
            top: c.top - py,
            right: c.right.max(c.left + 1) + px,
            bottom: c.bottom.max(c.top + 1) + py,
        })
    }
}

/// 控件的外框（左上角原点的全局点坐标）
fn frame(element: &CFType) -> Option<CGRect> {
    let origin: CGPoint = ax_value(&*attr(element, "AXPosition")?, AX_VALUE_CGPOINT)?;
    let size: CGSize = ax_value(&*attr(element, "AXSize")?, AX_VALUE_CGSIZE)?;
    Some(CGRect { origin, size })
}

/// 插入点（或选中文字）的外框。不支持的程序、没有插入点时返回 None。
fn caret(element: &CFType) -> Option<CGRect> {
    let range = attr(element, "AXSelectedTextRange")?;
    let mut out = null_mut();
    let err = unsafe {
        AXUIElementCopyParameterizedAttributeValue(element, &CFString::from_str("AXBoundsForRange"), &range, &mut out)
    };
    if err != 0 {
        return None;
    }
    let r: CGRect = ax_value(&*retained(out)?, AX_VALUE_CGRECT)?;
    // 有的程序拿不到时给一个全零的框
    (r.size.height > 0.0).then_some(r)
}
