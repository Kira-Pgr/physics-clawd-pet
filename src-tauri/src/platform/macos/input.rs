//! 焦点输入框（macOS）。TODO(macOS)：占位，宠物不会躲开正在打字的输入框。

use crate::desktop::Rect;

pub struct InputProbe;

impl InputProbe {
    pub fn new() -> Self {
        InputProbe
    }

    pub fn focused_input(&self, _skip: isize, _area: Rect) -> Option<Rect> {
        None
    }
}
