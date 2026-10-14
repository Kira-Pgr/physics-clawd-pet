// 宠物的世界：一块或几块屏幕的工作区（覆盖层 CSS 像素）拼起来。
// 拼好的区域的边界就是能撞、能站、能爬的"屏幕边框"；两块屏幕挨着的地方是开着的，可以走过去、飞过去。
//
// 边界拆成一段段横平竖直的线段（Seg），每段知道自己两头接着哪一段、拐角是凹的还是凸的：
// - 凹角：地面撞上往上的墙（和以前单屏的四个墙角一样），可以爬上去；
// - 凸角：边沿、台阶（比如矮一点的屏幕的地面到头了），从地面走到这儿会跳下去，从墙上爬到这儿会翻过去。

import { clamp } from './math';

/**
 * 宠物贴着的面：屏幕边框的四种边。站在其他窗口顶上时也是 'floor'，
 * 只是 Pet.support 指向那个窗口的顶边。
 */
export type Side = 'floor' | 'ceiling' | 'left' | 'right';

export interface Bounds {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** 一段边界：side 决定外面在哪边（地面 = 外面在下面，左墙 = 外面在左边） */
export interface Seg {
  side: Side;
  /** 固定的那个坐标：地面/天花板是 y，墙是 x */
  c: number;
  /** 沿线段方向的范围（地面/天花板是 x，墙是 y），a < b */
  a: number;
  b: number;
  /** 两头接着的线段（a 那头、b 那头） */
  lo: Link | null;
  hi: Link | null;
}

export interface Link {
  seg: Seg;
  /** true = 凸角（边沿、台阶），false = 凹角（墙角） */
  convex: boolean;
}

export interface World {
  screens: Bounds[];
  segs: Seg[];
}

/** 坐标误差：两块屏幕的边差这么多以内算挨着 */
const EPS = 1;

const horizontal = (side: Side) => side === 'floor' || side === 'ceiling';

/** 区间 [a, b] 减去一堆区间 */
function subtract(a: number, b: number, cuts: [number, number][]): [number, number][] {
  let parts: [number, number][] = [[a, b]];
  for (const [c0, c1] of cuts) {
    parts = parts.flatMap(([p0, p1]): [number, number][] => {
      if (c1 <= p0 || c0 >= p1) return [[p0, p1]];
      const out: [number, number][] = [];
      if (c0 > p0) out.push([p0, c0]);
      if (c1 < p1) out.push([c1, p1]);
      return out;
    });
  }
  return parts.filter(([p0, p1]) => p1 - p0 >= EPS);
}

/** 拼出边界。屏幕之间挨着（或重叠）的地方不算边界 */
export function buildWorld(input: Bounds[]): World {
  let screens = input.filter((s) => s.right - s.left >= EPS && s.bottom - s.top >= EPS);
  // 窗口还没显示出来时尺寸可能是 0：先给一块 1×1 的，等真正的屏幕尺寸来了再换
  if (!screens.length) {
    const o = input[0] ?? { left: 0, top: 0 };
    screens = [{ left: o.left, top: o.top, right: o.left + 1, bottom: o.top + 1 }];
  }
  const raw: Seg[] = [];
  const push = (side: Side, c: number, a: number, b: number, cuts: [number, number][]) => {
    for (const [p0, p1] of subtract(a, b, cuts)) raw.push({ side, c, a: p0, b: p1, lo: null, hi: null });
  };
  for (const r of screens) {
    const others = screens.filter((q) => q !== r);
    // 每条边：外面紧挨着的那一侧落在别的屏幕里的部分去掉
    push('floor', r.bottom, r.left, r.right,
      others.filter((q) => q.top <= r.bottom + EPS && q.bottom > r.bottom).map((q) => [q.left, q.right]));
    push('ceiling', r.top, r.left, r.right,
      others.filter((q) => q.bottom >= r.top - EPS && q.top < r.top).map((q) => [q.left, q.right]));
    push('left', r.left, r.top, r.bottom,
      others.filter((q) => q.right >= r.left - EPS && q.left < r.left).map((q) => [q.top, q.bottom]));
    push('right', r.right, r.top, r.bottom,
      others.filter((q) => q.left <= r.right + EPS && q.right > r.right).map((q) => [q.top, q.bottom]));
  }

  // 同一条线上首尾相接的段合并（两块屏幕底边对齐时，地面是一整条）
  const line = (s: Seg) => Math.round(s.c);
  raw.sort((p, q) => (p.side < q.side ? -1 : p.side > q.side ? 1 : line(p) - line(q) || p.a - q.a));
  const segs: Seg[] = [];
  for (const s of raw) {
    const last = segs[segs.length - 1];
    if (last && last.side === s.side && line(last) === line(s) && s.a <= last.b + EPS) {
      last.b = Math.max(last.b, s.b);
    } else {
      segs.push(s);
    }
  }

  // 拐角：每段的两头找垂直的那段。地面右端接"往上的右墙"是凹角，接"往下的左墙"是凸角，其余类推。
  for (const s of segs) {
    for (const end of ['lo', 'hi'] as const) {
      const at = end === 'lo' ? s.a : s.b;
      let found: Link | null = null;
      for (const t of segs) {
        if (horizontal(t.side) === horizontal(s.side) || Math.abs(t.c - at) > EPS) continue;
        const touchesA = Math.abs(t.a - s.c) <= EPS;
        const touchesB = Math.abs(t.b - s.c) <= EPS;
        if (!touchesA && !touchesB) continue;
        const link = { seg: t, convex: isConvex(s.side, end, t.side) };
        // 两块屏幕只在一个角上碰着时两种都有：优先凹角（留在本屏）
        if (!found || (found.convex && !link.convex)) found = link;
      }
      s[end] = found;
    }
  }
  return { screens, segs };
}

/** 从 side 的 end 那头拐到 next 这种面，是不是凸角 */
function isConvex(side: Side, end: 'lo' | 'hi', next: Side): boolean {
  switch (side) {
    case 'floor':
      return next === (end === 'lo' ? 'right' : 'left');
    case 'ceiling':
      return next === (end === 'lo' ? 'right' : 'left');
    case 'left':
      return next === (end === 'lo' ? 'floor' : 'ceiling');
    case 'right':
      return next === (end === 'lo' ? 'floor' : 'ceiling');
  }
}

/** 点所在的屏幕（含边上），不在任何屏幕里为 null */
export function screenAt(w: World, x: number, y: number): Bounds | null {
  return w.screens.find((s) => x >= s.left && x <= s.right && y >= s.top && y <= s.bottom) ?? null;
}

/** 离点最近的屏幕 */
export function nearestScreen(w: World, x: number, y: number): Bounds {
  let best = w.screens[0];
  let bestD = Infinity;
  for (const s of w.screens) {
    const d = Math.hypot(x - clamp(x, s.left, s.right), y - clamp(y, s.top, s.bottom));
    if (d < bestD) {
      bestD = d;
      best = s;
    }
  }
  return best;
}

/**
 * 某个方向上最近的一段边界：从 (x, y) 出发往 side 的外面那边看，范围盖住这个点、并且还没越过的那段。
 * from 是"上一步在哪儿"（同一轴上的坐标），用来防止一步走太远直接穿过去。
 */
export function nearestSeg(w: World, side: Side, x: number, y: number, from?: number): Seg | null {
  const along = horizontal(side) ? x : y;
  const across = from ?? (horizontal(side) ? y : x);
  // 地面在下面、右墙在右边：坐标比自己大的里面挑最小的；天花板、左墙反过来
  const ahead = side === 'floor' || side === 'right';
  let best: Seg | null = null;
  for (const s of w.segs) {
    if (s.side !== side || along < s.a || along > s.b) continue;
    if (ahead ? s.c < across : s.c > across) continue;
    if (!best || (ahead ? s.c < best.c : s.c > best.c)) best = s;
  }
  return best;
}

/** 正下方的地面（屏幕底边），没有为 null */
export function floorBelow(w: World, x: number, y: number): Seg | null {
  return nearestSeg(w, 'floor', x, y);
}

/** 正上方的天花板 */
export function ceilingAbove(w: World, x: number, y: number): Seg | null {
  return nearestSeg(w, 'ceiling', x, y);
}

/** 把一个半宽 ex、半高 ey 的盒子（中心 p）推回世界里面 */
export function keepInside(w: World, p: { x: number; y: number }, ex: number, ey: number): { x: number; y: number } {
  let { x, y } = p;
  if (!screenAt(w, x, y)) {
    const s = nearestScreen(w, x, y);
    x = clamp(x, s.left + Math.min(ex, (s.right - s.left) / 2), s.right - Math.min(ex, (s.right - s.left) / 2));
    y = clamp(y, s.top + Math.min(ey, (s.bottom - s.top) / 2), s.bottom - Math.min(ey, (s.bottom - s.top) / 2));
  }
  const f = nearestSeg(w, 'floor', x, y);
  if (f && y + ey > f.c) y = f.c - ey;
  const c = nearestSeg(w, 'ceiling', x, y);
  if (c && y - ey < c.c) y = c.c + ey;
  const l = nearestSeg(w, 'left', x, y);
  if (l && x - ex < l.c) x = l.c + ex;
  const r = nearestSeg(w, 'right', x, y);
  if (r && x + ex > r.c) x = r.c - ex;
  return { x, y };
}

/** 段离一个点有多远（法线方向 + 超出范围的部分），找"宠物原来贴着的是哪段"用 */
export function segDistance(s: Seg, x: number, y: number): number {
  const along = horizontal(s.side) ? x : y;
  const across = horizontal(s.side) ? y : x;
  return Math.abs(across - s.c) + Math.max(0, s.a - along, along - s.b);
}

/**
 * 窗口顶边裁到屏幕里面：只留下顶边所在那块屏幕横向范围内的部分，太短的扔掉。
 * 横跨两块并排屏幕的窗口，两边裁出来的再接回一整段（不然走到屏幕交界会当成窗口边缘）。
 */
export function clipToScreens<P extends { x0: number; x1: number; y: number }>(
  list: readonly P[],
  screens: readonly Bounds[],
  minLen: number,
): P[] {
  const out: P[] = [];
  for (const p of list) {
    const pieces: [number, number][] = [];
    for (const s of screens) {
      // 顶边贴着屏幕顶（最大化的窗口）没地方站
      if (p.y <= s.top || p.y >= s.bottom) continue;
      const x0 = Math.max(p.x0, s.left);
      const x1 = Math.min(p.x1, s.right);
      if (x1 > x0) pieces.push([x0, x1]);
    }
    pieces.sort((a, b) => a[0] - b[0]);
    const merged: [number, number][] = [];
    for (const piece of pieces) {
      const last = merged[merged.length - 1];
      if (last && piece[0] <= last[1] + EPS) last[1] = Math.max(last[1], piece[1]);
      else merged.push([...piece]);
    }
    for (const [x0, x1] of merged) if (x1 - x0 >= minLen) out.push({ ...p, x0, x1 });
  }
  return out;
}
