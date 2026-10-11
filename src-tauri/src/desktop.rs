//! 桌面几何：窗口矩形、可站立的窗口顶边。与平台无关，查询系统的部分在 platform 里。

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// 一个顶层窗口。`platform` 表示它的顶边可以站人；不能站的窗口仍然参与遮挡计算。
#[derive(Clone, Copy, Debug)]
pub struct WinInfo {
    pub id: isize,
    pub rect: Rect,
    pub platform: bool,
}

/// 可站立的一段窗口顶边（物理像素）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment {
    pub id: isize,
    pub x0: i32,
    pub x1: i32,
    pub y: i32,
}

/// 计算可站立的顶边：每个窗口的顶边减去被它上面（Z 序更高）的窗口挡住的部分，再裁到工作区内。
pub fn platform_segments(wins: &[WinInfo], area: Rect, min_len: i32) -> Vec<Segment> {
    let mut out = Vec::new();
    for (i, w) in wins.iter().enumerate() {
        if !w.platform {
            continue;
        }
        let y = w.rect.top;
        // 顶边贴着工作区顶部（比如最大化窗口）就没地方站了
        if y <= area.top || y >= area.bottom {
            continue;
        }
        let mut segs = vec![(w.rect.left.max(area.left), w.rect.right.min(area.right))];
        for above in &wins[..i] {
            let r = above.rect;
            if r.top < y && r.bottom >= y {
                segs = segs
                    .into_iter()
                    .flat_map(|(a, b)| {
                        let mut parts = Vec::with_capacity(2);
                        if r.left > a {
                            parts.push((a, b.min(r.left)));
                        }
                        if r.right < b {
                            parts.push((a.max(r.right), b));
                        }
                        parts
                    })
                    .filter(|(a, b)| b > a)
                    .collect();
                if segs.is_empty() {
                    break;
                }
            }
        }
        for (x0, x1) in segs {
            if x1 - x0 >= min_len {
                out.push(Segment { id: w.id, x0, x1, y });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(id: isize, l: i32, t: i32, r: i32, b: i32) -> WinInfo {
        WinInfo { id, rect: Rect { left: l, top: t, right: r, bottom: b }, platform: true }
    }
    const AREA: Rect = Rect { left: 0, top: 0, right: 1920, bottom: 1040 };

    #[test]
    fn top_edge_is_split_by_window_above() {
        // 窗口 2 在窗口 1 上面，盖住了窗口 1 顶边的中间一段
        let wins = [win(2, 400, 100, 700, 600), win(1, 200, 300, 1000, 800)];
        let segs = platform_segments(&wins, AREA, 10);
        assert_eq!(
            segs,
            vec![
                Segment { id: 2, x0: 400, x1: 700, y: 100 },
                Segment { id: 1, x0: 200, x1: 400, y: 300 },
                Segment { id: 1, x0: 700, x1: 1000, y: 300 },
            ]
        );
    }

    #[test]
    fn window_below_the_edge_does_not_occlude() {
        // 上层窗口的顶边比下层窗口顶边还低：挡不住下层窗口的顶边
        let wins = [win(2, 0, 500, 1920, 900), win(1, 200, 300, 1000, 800)];
        let segs = platform_segments(&wins, AREA, 10);
        assert!(segs.contains(&Segment { id: 1, x0: 200, x1: 1000, y: 300 }));
    }

    #[test]
    fn maximized_window_hides_everything_behind_it_and_has_no_room_on_top() {
        let mut max = win(9, 0, 0, 1920, 1040);
        max.platform = true;
        let wins = [max, win(1, 200, 300, 1000, 800)];
        assert!(platform_segments(&wins, AREA, 10).is_empty());
    }

    #[test]
    fn segments_are_clipped_to_work_area_and_min_length() {
        let wins = [win(1, -300, 200, 50, 500), win(2, 1900, 400, 2400, 700)];
        let segs = platform_segments(&wins, AREA, 30);
        assert_eq!(segs, vec![Segment { id: 1, x0: 0, x1: 50, y: 200 }]);
    }
}
