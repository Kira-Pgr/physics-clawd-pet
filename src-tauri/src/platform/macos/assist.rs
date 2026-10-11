//! 小助手要的系统能力（macOS）。TODO(macOS)：日历订阅、选 .ics 文件还是占位。

use objc2_core_graphics::{CGEventSource, CGEventSourceStateID, CGEventType};

/// 距离上一次键盘/鼠标操作多少毫秒（整个系统的，不只是本程序）。不需要任何权限。
pub fn idle_ms() -> u64 {
    // kCGAnyInputEventType：任何输入事件
    let any_input = CGEventType(u32::MAX);
    let secs = CGEventSource::seconds_since_last_event_type(CGEventSourceStateID::CombinedSessionState, any_input);
    (secs.max(0.0) * 1000.0) as u64
}

pub fn http_get(_url: &str) -> Result<Vec<u8>, String> {
    Err("unsupported platform".into())
}

pub fn decode_system_codepage(_bytes: &[u8]) -> Option<String> {
    None
}

pub fn pick_ics(_owner: Option<isize>, _title: String, _filter_name: String) -> Option<String> {
    None
}
