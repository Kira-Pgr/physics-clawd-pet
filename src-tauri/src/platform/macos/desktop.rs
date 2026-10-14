//! 桌面状态查询（macOS）：光标、可见窗口、工作区、全屏检测。
//! 全部只读，不移动、不修改任何其他程序的窗口。
//!
//! 坐标和 Tauri 一致："物理像素" = 点 × 主屏的 backingScaleFactor，原点在主屏左上角。
//! CoreGraphics 给的都是这种左上角原点的点坐标，乘上主屏缩放就行；
//! 宠物只在主屏上活动，别的屏缩放不一样也没关系。窗口 id 是 CGWindowID。
//!
//! 为什么不跨屏：macOS 默认"每块显示器有单独的空间"，一个窗口只会出现在一块屏幕上，
//! 铺满几块屏幕的覆盖层在这儿行不通。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use objc2_app_kit::NSScreen;
use objc2_core_foundation::{CFArray, CFBoolean, CFDictionary, CFNumber, CFRetained, CFString, CFType, CGRect};
use objc2_core_graphics::{
    kCGWindowAlpha, kCGWindowBounds, kCGWindowIsOnscreen, kCGWindowLayer, kCGWindowNumber, kCGWindowOwnerPID,
    CGDisplayBounds, CGEvent, CGMainDisplayID, CGRectMakeWithDictionaryRepresentation, CGWindowListCopyWindowInfo,
    CGWindowListOption,
};

use crate::desktop::{Rect, Screen, WinInfo};

/// 普通程序窗口的层级（kCGNormalWindowLevel）
const NORMAL_LAYER: i64 = 0;
/// 程序坞的层级（kCGDockWindowLevel）。比它高的是程序坞、菜单栏、通知、弹出菜单之类
const DOCK_LAYER: i64 = 20;

/// 主屏缩放的缓存：光标每帧都要换算，不能每次都跑到主线程去问 NSScreen
static SCALE: AtomicU64 = AtomicU64::new(0);

/// 主屏（有菜单栏的那块）的工作区（去掉菜单栏和程序坞，物理像素）和缩放，算法和 Tauri 的 work_area 一样。
/// NSScreen 只能在主线程用：不在主线程就同步派过去（观察线程一秒才问一次）。
fn primary_screen() -> Option<(Rect, f64)> {
    let (r, s) = dispatch2::run_on_main(|mtm| {
        let screen = NSScreen::screens(mtm).firstObject()?;
        let (full, vis) = (screen.frame(), screen.visibleFrame());
        // AppKit 是左下角原点、y 朝上：换成左上角原点
        let x = vis.origin.x - full.origin.x;
        let y = (full.origin.y + full.size.height) - (vis.origin.y + vis.size.height);
        Some((CGRect::new(objc2_core_foundation::CGPoint::new(x, y), vis.size), screen.backingScaleFactor()))
    })?;
    SCALE.store(s.to_bits(), Ordering::Relaxed);
    let (left, top) = ((r.origin.x * s).round() as i32, (r.origin.y * s).round() as i32);
    Some((
        Rect {
            left,
            top,
            right: left + (r.size.width * s).round() as i32,
            bottom: top + (r.size.height * s).round() as i32,
        },
        s,
    ))
}

pub(super) fn scale() -> f64 {
    match f64::from_bits(SCALE.load(Ordering::Relaxed)) {
        s if s > 0.0 => s,
        _ => primary_screen().map_or(1.0, |(_, s)| s),
    }
}

/// 点 → 物理像素
pub(super) fn to_rect(b: CGRect, s: f64) -> Rect {
    Rect {
        left: (b.origin.x * s).round() as i32,
        top: (b.origin.y * s).round() as i32,
        right: ((b.origin.x + b.size.width) * s).round() as i32,
        bottom: ((b.origin.y + b.size.height) * s).round() as i32,
    }
}

pub fn cursor_pos() -> Option<(i32, i32)> {
    // 空事件的位置就是当前光标位置（左上角原点的全局点坐标）
    let event = CGEvent::new(None)?;
    let p = CGEvent::location(Some(&event));
    let s = scale();
    Some(((p.x * s).round() as i32, (p.y * s).round() as i32))
}

/// 主屏工作区（不含菜单栏和程序坞），物理像素
fn primary_work_area() -> Option<Rect> {
    primary_screen().map(|(r, _)| r)
}

/// 主屏整屏，物理像素
fn primary_monitor() -> Rect {
    to_rect(CGDisplayBounds(CGMainDisplayID()), scale())
}

/// 宠物能去的屏幕：只有主屏（见文件开头）
pub fn screens() -> Vec<Screen> {
    primary_work_area()
        .map(|work| vec![Screen { monitor: primary_monitor(), work, primary: true }])
        .unwrap_or_default()
}

/// 覆盖层只在主屏上，就是主屏的缩放
pub fn window_scale(_id: isize) -> f64 {
    primary_screen().map_or(1.0, |(_, s)| s)
}

type WindowInfo = CFDictionary<CFString, CFType>;

/// CGWindowListCopyWindowInfo：每个窗口一个字典，按 Z 序从上到下。
/// 拿窗口位置、层级、所属进程不需要"屏幕录制"权限（只有窗口标题要）。
fn window_list(option: CGWindowListOption, relative_to: isize) -> Vec<CFRetained<WindowInfo>> {
    let Some(list) = CGWindowListCopyWindowInfo(option, relative_to as u32) else { return vec![] };
    // 文档保证每一项都是 CFString → CFType 的字典
    let list: CFRetained<CFArray<WindowInfo>> = unsafe { CFRetained::cast_unchecked(list) };
    (0..list.len()).filter_map(|i| list.get(i)).collect()
}

fn on_screen_windows() -> Vec<CFRetained<WindowInfo>> {
    window_list(CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements, 0)
}

fn int(d: &WindowInfo, key: &CFString) -> Option<i64> {
    d.get(key)?.downcast::<CFNumber>().ok()?.as_i64()
}

fn float(d: &WindowInfo, key: &CFString) -> Option<f64> {
    d.get(key)?.downcast::<CFNumber>().ok()?.as_f64()
}

/// 窗口的外框（点，不含阴影）
fn bounds(d: &WindowInfo) -> Option<CGRect> {
    let b = d.get(unsafe { kCGWindowBounds })?.downcast::<CFDictionary>().ok()?;
    let mut r = CGRect::default();
    unsafe { CGRectMakeWithDictionaryRepresentation(Some(&b), &mut r) }.then_some(r)
}

/// 窗口外框。窗口没了、最小化了、切到别的桌面（Space）了、所在程序被隐藏了时返回 None。
pub fn frame_bounds(id: isize) -> Option<Rect> {
    if id <= 0 {
        return None;
    }
    let d = window_list(CGWindowListOption::OptionIncludingWindow, id).into_iter().next()?;
    // 不在屏幕上的窗口这个键要么是 false 要么干脆没有
    let on_screen = d
        .get(unsafe { kCGWindowIsOnscreen })
        .and_then(|v| v.downcast::<CFBoolean>().ok())
        .is_some_and(|b| b.as_bool());
    if !on_screen {
        return None;
    }
    let r = to_rect(bounds(&d)?, scale());
    (r.width() > 0 && r.height() > 0).then_some(r)
}

/// 所有可见的窗口，按 Z 序从上到下排列。`skip` 是我们自己的覆盖层。
pub fn visible_windows(skip: isize) -> Vec<WinInfo> {
    let s = scale();
    let area = primary_work_area();
    let mut out = Vec::new();
    for d in on_screen_windows() {
        let Some(id) = int(&d, unsafe { kCGWindowNumber }).map(|n| n as isize) else { continue };
        if id == skip {
            continue;
        }
        // 程序坞、菜单栏、通知、弹出菜单：不能站，也不当遮挡物（基本都在工作区外面）
        let layer = int(&d, unsafe { kCGWindowLayer }).unwrap_or(NORMAL_LAYER);
        if !(NORMAL_LAYER..DOCK_LAYER).contains(&layer) {
            continue;
        }
        // 完全透明的窗口
        if float(&d, unsafe { kCGWindowAlpha }).is_some_and(|a| a <= 0.0) {
            continue;
        }
        let Some(b) = bounds(&d) else { continue };
        let rect = to_rect(b, s);
        if rect.width() <= 0 || rect.height() <= 0 {
            continue;
        }
        // 浮在普通窗口上面、却盖满整个工作区的：多半是别的透明覆盖层（其他桌宠、录屏标注……），
        // 它们点击穿透，当遮挡物会把所有窗口顶边都挡掉。macOS 查不到窗口是否穿透，只能这样猜。
        if layer > NORMAL_LAYER
            && area.is_some_and(|a| {
                rect.left <= a.left && rect.top <= a.top && rect.right >= a.right && rect.bottom >= a.bottom
            })
        {
            continue;
        }
        // 普通窗口能站；浮动面板之类会遮挡但不能站
        let platform = layer == NORMAL_LAYER && rect.width() >= 120 && rect.height() >= 60;
        out.push(WinInfo { id, rect, platform });
    }
    out
}

/// 按顺序看这些进程（通常是某个进程的父、祖父……），返回第一个拥有可见普通窗口的进程的那个窗口。
/// 同一个进程有好几个窗口时取 Z 序最上面的。用来找"在跑 Claude Code 的是哪个终端/编辑器窗口"。
pub fn window_for_pids(pids: &[u32]) -> Option<isize> {
    if pids.is_empty() {
        return None;
    }
    let mut by_pid: HashMap<u32, isize> = HashMap::new();
    for d in on_screen_windows() {
        if int(&d, unsafe { kCGWindowLayer }) != Some(NORMAL_LAYER) {
            continue;
        }
        let (Some(id), Some(pid)) = (int(&d, unsafe { kCGWindowNumber }), int(&d, unsafe { kCGWindowOwnerPID }))
        else {
            continue;
        };
        by_pid.entry(pid as u32).or_insert(id as isize);
    }
    pids.iter().find_map(|p| by_pid.get(p).copied())
}

/// 有全屏程序的屏幕（只看主屏），返回整屏矩形
pub fn fullscreen_monitors(skip: isize) -> Vec<Rect> {
    if fullscreen_on_primary(skip) {
        vec![primary_monitor()]
    } else {
        vec![]
    }
}

/// 主屏上是否有全屏程序在前台（全屏视频、演示、游戏）。
/// 原生全屏的程序在自己的 Space 里，覆盖层本来就不会出现在那儿；认出来是为了让前端暂停，和 Windows 一致。
/// 最大化（zoom）的窗口不盖菜单栏，不算全屏。
fn fullscreen_on_primary(skip: isize) -> bool {
    let display = CGDisplayBounds(CGMainDisplayID());
    let covers = |b: &CGRect| {
        b.origin.x <= display.origin.x
            && b.origin.y <= display.origin.y
            && b.origin.x + b.size.width >= display.origin.x + display.size.width
            && b.origin.y + b.size.height >= display.origin.y + display.size.height
    };
    // 只看最上面的普通窗口：浮动层上盖满屏幕的多半是别的透明覆盖层（见 visible_windows）
    on_screen_windows()
        .iter()
        .filter(|d| int(d, unsafe { kCGWindowNumber }) != Some(skip as i64))
        .filter(|d| int(d, unsafe { kCGWindowLayer }) == Some(NORMAL_LAYER))
        .find_map(|d| bounds(d))
        .is_some_and(|b| covers(&b))
}
