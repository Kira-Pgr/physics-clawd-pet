//! 界面语言：中文 / 日文 / 英文，其他语言用英文。托盘菜单、窗口标题和原生侧的错误提示都按它选。
//! （网页里的文字由前端按 navigator.language 选，规则一致。）

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Zh,
    Ja,
    En,
}

pub fn lang() -> Lang {
    static LANG: std::sync::OnceLock<Lang> = std::sync::OnceLock::new();
    *LANG.get_or_init(crate::platform::ui_language)
}

/// 按当前语言挑一句
pub fn tr(zh: &str, ja: &str, en: &str) -> String {
    match lang() {
        Lang::Zh => zh,
        Lang::Ja => ja,
        Lang::En => en,
    }
    .to_string()
}

/// "说明：原因" 里的冒号（英文用半角加空格）
pub fn colon() -> &'static str {
    match lang() {
        Lang::En => ": ",
        _ => "：",
    }
}
