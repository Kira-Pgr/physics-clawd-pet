//! 小助手：日历文本解码、下载出错的提示（与平台无关）。键鼠空闲时长、下载、选文件这些系统能力在 platform 里。

use crate::lang::{colon, tr};
use crate::platform;

/// 日历文件最大多大（超过就当出错，免得把内存吃光）
pub const MAX_BODY: usize = 20 * 1024 * 1024;

/// 下载失败的原因：各平台把系统错误码翻成这个，提示文字只写一份
pub enum NetError {
    Timeout,
    Dns,
    Connect,
    Tls,
    BadUrl,
    /// 没单独翻译的系统错误码
    Code(i64),
}

/// "说明：原因"
pub fn net_error(what: &str, why: NetError) -> String {
    let why = match why {
        NetError::Timeout => tr("超时", "タイムアウトしました", "timed out"),
        NetError::Dns => tr("域名解析失败", "ホスト名を解決できませんでした", "could not resolve the host name"),
        NetError::Connect => tr("连接失败", "接続できませんでした", "connection failed"),
        NetError::Tls => tr("安全连接（TLS）失败", "セキュア接続（TLS）に失敗しました", "secure connection (TLS) failed"),
        NetError::BadUrl => tr("地址格式不对", "URLの形式が正しくありません", "invalid URL"),
        NetError::Code(code) => {
            tr(&format!("错误码 {code}"), &format!("エラーコード {code}"), &format!("error code {code}"))
        }
    };
    format!("{what}{}{why}", colon())
}

pub fn cannot_reach() -> String {
    tr("连不上服务器", "サーバーに接続できません", "Could not reach the server")
}

/// 地址格式错误（解析不了）
pub fn bad_url() -> String {
    tr("地址格式不对", "URLの形式が正しくありません", "Invalid URL")
}

pub fn unsupported_scheme() -> String {
    tr(
        "只支持 http / https / webcal 地址",
        "http / https / webcal のURLのみ対応しています",
        "Only http / https / webcal URLs are supported",
    )
}

pub fn http_status(status: i64) -> String {
    tr(
        &format!("服务器返回 HTTP {status}"),
        &format!("サーバーが HTTP {status} を返しました"),
        &format!("The server returned HTTP {status}"),
    )
}

pub fn too_large() -> String {
    tr("日历文件太大了", "カレンダーファイルが大きすぎます", "The calendar file is too large")
}

/// iCalendar 规定是 UTF-8，但现实里有不守规矩的：
/// UTF-16（记事本"Unicode"、PowerShell 5.1 的 `>`）带 BOM；
/// 老的グループウェア（サイボウズ、desknet's NEO）和旧版 Outlook 导出的是系统代码页（日文系统 Shift_JIS、中文系统 GBK）。
pub fn decode_text(bytes: Vec<u8>) -> String {
    let utf16 = |rest: &[u8], big_endian: bool| {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|c| if big_endian { u16::from_be_bytes([c[0], c[1]]) } else { u16::from_le_bytes([c[0], c[1]]) })
            .collect();
        String::from_utf16_lossy(&units)
    };
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return utf16(rest, false);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return utf16(rest, true);
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).map(<[u8]>::to_vec).unwrap_or(bytes);
    match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => {
            let bytes = e.into_bytes();
            // 不是合法 UTF-8：先当系统代码页试（严格模式，有非法字节就放弃），都不行才把坏字节替换掉
            platform::decode_system_codepage(&bytes).unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_is_sane() {
        // 跑测试的时候不可能几天没碰过电脑
        assert!(platform::idle_ms() < 7 * 24 * 3600 * 1000);
    }

    #[test]
    fn decode_strips_bom() {
        assert_eq!(decode_text(b"\xEF\xBB\xBFBEGIN:VCALENDAR".to_vec()), "BEGIN:VCALENDAR");
    }

    #[test]
    fn decode_utf8_japanese_is_untouched() {
        assert_eq!(decode_text("SUMMARY:会議 ☕\n".as_bytes().to_vec()), "SUMMARY:会議 ☕\n");
    }

    #[test]
    fn decode_utf16_with_bom() {
        let mut le = vec![0xFF, 0xFE];
        le.extend("SUMMARY:会議".encode_utf16().flat_map(u16::to_le_bytes));
        assert_eq!(decode_text(le), "SUMMARY:会議");
        let mut be = vec![0xFE, 0xFF];
        be.extend("SUMMARY:会議".encode_utf16().flat_map(u16::to_be_bytes));
        assert_eq!(decode_text(be), "SUMMARY:会議");
    }

    /// 需要联网：cargo test -- --ignored
    #[test]
    #[ignore]
    fn fetch_public_calendar() {
        let get = |url: &str| platform::http_get(url).map(decode_text);
        let text = get("https://calendars.icloud.com/holidays/cn_zh.ics").expect("download");
        assert!(text.contains("BEGIN:VCALENDAR"), "{}", &text[..text.len().min(200)]);
        assert!(get("https://calendars.icloud.com/holidays/does-not-exist.ics").is_err());
        assert!(get("ftp://example.com/x.ics").is_err());
    }
}
