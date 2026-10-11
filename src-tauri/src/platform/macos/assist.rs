//! 小助手要的系统能力（macOS）。TODO(macOS)：占位，休息提醒、日历订阅、选 .ics 文件都不能用。

pub fn idle_ms() -> u64 {
    0
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
