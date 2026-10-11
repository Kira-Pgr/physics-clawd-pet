//! 小助手要的几样系统能力（macOS）：键鼠空闲时长、下载日历订阅、选本地 .ics 文件、老编码解码。
//! 下载用系统自带的 NSURLSession（走系统代理设置、系统证书），不引入额外的 HTTP/TLS 依赖。

use std::sync::mpsc;
use std::time::Duration;

use block2::RcBlock;
use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
use objc2_core_foundation::{CFIndex, CFString, CFStringBuiltInEncodings, CFStringEncoding, CFStringEncodings};
use objc2_core_graphics::{CGEventSource, CGEventSourceStateID, CGEventType};
use objc2_foundation::{
    NSArray, NSData, NSError, NSHTTPURLResponse, NSString, NSURLResponse, NSURLSession, NSURLSessionConfiguration,
    NSURL,
};

use crate::assist::{bad_url, cannot_reach, http_status, net_error, too_large, unsupported_scheme, NetError, MAX_BODY};
use crate::lang::{lang, Lang};

/// 距离上一次键盘/鼠标操作多少毫秒（整个系统的，不只是本程序）。不需要任何权限。
pub fn idle_ms() -> u64 {
    // kCGAnyInputEventType：任何输入事件
    let any_input = CGEventType(u32::MAX);
    let secs = CGEventSource::seconds_since_last_event_type(CGEventSourceStateID::CombinedSessionState, any_input);
    (secs.max(0.0) * 1000.0) as u64
}

/// 完成回调交回来的结果：(HTTP 状态码, 正文) 或者 (错误域, 错误码)
type Outcome = Result<(Option<i64>, Vec<u8>), (String, i64)>;

/// GET 一个 http(s) 地址，返回原始字节（解码交给 assist::decode_text）。自动跟随重定向、解 gzip。
pub fn http_get(url: &str) -> Result<Vec<u8>, String> {
    let ns_url = NSURL::URLWithString(&NSString::from_str(url)).ok_or_else(bad_url)?;
    let scheme = ns_url.scheme().map(|s| s.to_string().to_ascii_lowercase());
    if !matches!(scheme.as_deref(), Some("http" | "https")) {
        return Err(unsupported_scheme());
    }

    // 临时会话：不往磁盘写缓存和 cookie，每次都拿最新的日历
    let config = NSURLSessionConfiguration::ephemeralSessionConfiguration();
    config.setTimeoutIntervalForRequest(15.0);
    config.setTimeoutIntervalForResource(60.0);
    let session = NSURLSession::sessionWithConfiguration(&config);

    let (tx, rx) = mpsc::channel::<Outcome>();
    // 回调在会话自己的后台队列上跑，这个线程只管等
    let handler = RcBlock::new(move |data: *mut NSData, response: *mut NSURLResponse, error: *mut NSError| {
        let outcome = match unsafe { error.as_ref() } {
            Some(e) => Err((e.domain().to_string(), e.code() as i64)),
            None => {
                let status = unsafe { response.as_ref() }
                    .and_then(|r| r.downcast_ref::<NSHTTPURLResponse>())
                    .map(|h| h.statusCode() as i64);
                Ok((status, unsafe { data.as_ref() }.map(|d| d.to_vec()).unwrap_or_default()))
            }
        };
        let _ = tx.send(outcome);
    });
    let task = unsafe { session.dataTaskWithURL_completionHandler(&ns_url, &handler) };
    task.resume();
    // 会话自己 60 秒就超时，这里多等一会儿兜底
    let outcome = rx.recv_timeout(Duration::from_secs(75));
    session.finishTasksAndInvalidate();

    match outcome {
        Ok(Ok((Some(200), body))) => {
            if body.len() > MAX_BODY {
                Err(too_large())
            } else {
                Ok(body)
            }
        }
        Ok(Ok((status, _))) => Err(http_status(status.unwrap_or(0))),
        Ok(Err((domain, code))) => Err(net_error(&cannot_reach(), url_error(&domain, code))),
        Err(_) => {
            task.cancel();
            Err(net_error(&cannot_reach(), NetError::Timeout))
        }
    }
}

/// NSURLErrorDomain 的错误码（NSURLError.h），常见的几个自己翻
fn url_error(domain: &str, code: i64) -> NetError {
    if domain != "NSURLErrorDomain" {
        return NetError::Code(code);
    }
    match code {
        -1001 => NetError::Timeout,
        -1003 | -1006 => NetError::Dns,
        -1004 | -1005 | -1009 => NetError::Connect,
        -1206..=-1200 => NetError::Tls,
        -1000 | -1002 => NetError::BadUrl,
        _ => NetError::Code(code),
    }
}

/// 不是合法 UTF-8 的文本，按老软件最可能用的编码严格解码，有非法字节返回 None。
/// macOS 没有"系统代码页"，就按界面语言挑，和同语言的 Windows 一样：日文 Shift_JIS（CP932）、中文 GB18030、其他 CP1252。
pub fn decode_system_codepage(bytes: &[u8]) -> Option<String> {
    let encoding = match lang() {
        Lang::Ja => CFStringEncodings::DOSJapanese.0 as CFStringEncoding,
        Lang::Zh => CFStringEncodings::GB_18030_2000.0 as CFStringEncoding,
        Lang::En => CFStringBuiltInEncodings::EncodingWindowsLatin1.0,
    };
    decode_with(bytes, encoding)
}

fn decode_with(bytes: &[u8], encoding: CFStringEncoding) -> Option<String> {
    if bytes.is_empty() {
        return Some(String::new());
    }
    // 转不了（有非法字节）时 CoreFoundation 返回空
    let s = unsafe { CFString::with_bytes(None, bytes.as_ptr(), bytes.len() as CFIndex, encoding, false) }?;
    Some(s.to_string())
}

/// 弹出"打开文件"面板选一个 .ics，返回路径；取消返回 None。
/// 面板只能在主线程上开：调用方在后台线程，这里同步派到主线程等用户选完。
pub fn pick_ics(_owner: Option<isize>, title: String, _filter_name: String) -> Option<String> {
    dispatch2::run_on_main(move |mtm| {
        let panel = NSOpenPanel::openPanel(mtm);
        // macOS 的打开面板没有标题栏，标题写在面板顶上的说明里
        panel.setMessage(Some(&NSString::from_str(&title)));
        panel.setCanChooseDirectories(false);
        panel.setAllowsMultipleSelection(false);
        let types = NSArray::from_retained_slice(&["ics", "ical", "ifb"].map(NSString::from_str));
        // 新接口要 UTType（多一个框架依赖），按扩展名过滤的老接口一直能用
        #[allow(deprecated)]
        panel.setAllowedFileTypes(Some(&types));
        if panel.runModal() != NSModalResponseOK {
            return None; // 取消
        }
        panel.URL()?.path().map(|p| p.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_shift_jis_strictly() {
        let sjis = CFStringEncodings::DOSJapanese.0 as CFStringEncoding;
        // "会議" = 89 EF 8B 63（和 Windows 版的测试同一组字节）
        assert_eq!(decode_with(b"SUMMARY:\x89\xEF\x8B\x63", sjis).as_deref(), Some("SUMMARY:会議"));
        // 严格模式：孤立的前导字节算非法
        assert_eq!(decode_with(b"\x89", sjis), None);
    }

    #[test]
    fn decode_gb18030() {
        let gb = CFStringEncodings::GB_18030_2000.0 as CFStringEncoding;
        // "会议" = BB E1 D2 E9
        assert_eq!(decode_with(b"SUMMARY:\xBB\xE1\xD2\xE9", gb).as_deref(), Some("SUMMARY:会议"));
    }

    #[test]
    fn url_errors_map_to_reasons() {
        assert!(matches!(url_error("NSURLErrorDomain", -1001), NetError::Timeout));
        assert!(matches!(url_error("NSURLErrorDomain", -1202), NetError::Tls));
        assert!(matches!(url_error("NSURLErrorDomain", -1009), NetError::Connect));
        assert!(matches!(url_error("NSPOSIXErrorDomain", -1001), NetError::Code(-1001)));
    }
}
