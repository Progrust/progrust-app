// 動きの補助（documents/spec/content-format/video/scenes.md「共通部品」、
// design-system/progrust-video/README.md「動きの種類と速さ」）。
// どの関数も、引数（現在のフレームなど）だけから結果が決まる。時計・乱数・CSS のアニメーションは使わない。

// 動きの長さ（30fps でのフレーム数。tokens.css の video-frames-*）。
export const FRAMES = {
  sceneFade: 10,
  enter: 16,
  exit: 8,
  draw: 12,
  lineDraw: 18,
  dim: 8,
  open: 8,
  move: 30,
  resize: 20,
  camera: 24,
} as const;

// 現れるときに上がる距離（基準の px。tokens.css の video-rise）。
export const RISE = 16;

export type Easing = (t: number) => number;

// 0〜1 に収める。
export function clamp01(t: number): number {
  return Math.min(1, Math.max(0, t));
}

// CSS の cubic-bezier(x1, y1, x2, y2) と同じ形のイージングを作る。
export function cubicBezier(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
): Easing {
  const coord = (a: number, b: number, s: number) =>
    3 * a * (1 - s) * (1 - s) * s + 3 * b * (1 - s) * s * s + s * s * s;
  return (t) => {
    const x = clamp01(t);
    if (x === 0 || x === 1) {
      return x;
    }
    // x に対応する s を二分法で求める（x は s について単調に増える）。
    let lo = 0;
    let hi = 1;
    for (let i = 0; i < 40; i++) {
      const mid = (lo + hi) / 2;
      if (coord(x1, x2, mid) < x) {
        lo = mid;
      } else {
        hi = mid;
      }
    }
    return coord(y1, y2, (lo + hi) / 2);
  };
}

// すっと止まる動き（video-ease-out）。現れる・消える・描く・淡くする・差し込む・入力するの既定。
export const easeOut: Easing = cubicBezier(0.16, 1, 0.3, 1);
// ゆっくり動き出して、ゆっくり止まる動き（video-ease-in-out）。移る・寄せる・幅を変える・線を描く。
export const easeInOut: Easing = cubicBezier(0.65, 0, 0.35, 1);
// 一定の速さ。
export const linear: Easing = (t) => clamp01(t);

// start フレームから duration フレームかけて 0 から 1 へ進む、進み具合。
export function progress(
  frame: number,
  start: number,
  duration: number,
  easing: Easing = linear,
): number {
  if (duration <= 0) {
    return frame >= start ? 1 : 0;
  }
  return easing(clamp01((frame - start) / duration));
}

// from から to へ、進み具合 t で補間する。
export function lerp(from: number, to: number, t: number): number {
  return from + (to - from) * t;
}

// 現れる動き: 不透明度 0 → 1、下から rise px 上がる。
export function enter(
  frame: number,
  start: number,
  options: { duration?: number; rise?: number } = {},
): { opacity: number; translateY: number } {
  const p = progress(frame, start, options.duration ?? FRAMES.enter, easeOut);
  return { opacity: p, translateY: (1 - p) * (options.rise ?? RISE) };
}

// 消える動き: 不透明度 1 → 0。
export function exit(
  frame: number,
  start: number,
  options: { duration?: number } = {},
): { opacity: number } {
  return {
    opacity:
      1 - progress(frame, start, options.duration ?? FRAMES.exit, easeOut),
  };
}

// 続けて現れる要素の開始フレーム（index 番目。gap フレームずつずらす）。
export function stagger(start: number, index: number, gap = 6): number {
  return start + index * gap;
}

// シーンの出入りの不透明度（10 フレームのフェード）。
// 最後のシーンは、終わりで消さない。
export function sceneOpacity(
  frame: number,
  durationInFrames: number,
  options: { last?: boolean } = {},
): number {
  const fadeIn = progress(frame, 0, FRAMES.sceneFade, easeOut);
  if (options.last === true) {
    return fadeIn;
  }
  const fadeOut =
    1 -
    progress(
      frame,
      durationInFrames - FRAMES.sceneFade,
      FRAMES.sceneFade,
      easeOut,
    );
  return Math.min(fadeIn, fadeOut);
}
