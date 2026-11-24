// 演習画面の境界の取っ手（design-system/progrust/components/Splitter）。
// ドラッグと矢印キーで、隣の領域の大きさ（px）を変える。
import { useRef, type KeyboardEvent, type PointerEvent } from "react";

// 矢印キー 1 回で動かす量（px）。
export const SPLIT_KEY_STEP = 16;

// 大きさを min〜max に収める。max が min より小さいときは min にする。
export function clampSize(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(value, max));
}

export type SplitterProps = {
  // vertical は左右の境界（縦の棒）、horizontal は上下の境界（横の棒）。
  orientation: "vertical" | "horizontal";
  label: string;
  // 変える領域のいまの大きさ（px）。
  value: number;
  min: number;
  // いま取れる最大の大きさ（px）。ドラッグを始めるときと、キーを押したときに求める。
  max: () => number;
  // 境界を右（下）へ動かしたときに大きくなるなら 1、小さくなるなら -1。
  direction: 1 | -1;
  onChange: (value: number) => void;
};

export function Splitter({
  orientation,
  label,
  value,
  min,
  max,
  direction,
  onChange,
}: SplitterProps) {
  // ドラッグを始めたときのポインタの位置と大きさと最大。
  const drag = useRef<{ start: number; base: number; max: number } | null>(
    null,
  );
  const vertical = orientation === "vertical";
  const position = (event: PointerEvent) =>
    vertical ? event.clientX : event.clientY;

  const handlePointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault();
    // ポインタが取っ手から外れても（エディタの上に出ても）、離すまで追う。
    event.currentTarget.setPointerCapture?.(event.pointerId);
    drag.current = { start: position(event), base: value, max: max() };
  };

  const handlePointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const current = drag.current;
    if (current === null) {
      return;
    }
    const offset = (position(event) - current.start) * direction;
    onChange(clampSize(current.base + offset, min, current.max));
  };

  const handlePointerEnd = () => {
    drag.current = null;
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const keys: Record<string, number | undefined> = vertical
      ? { ArrowLeft: -1, ArrowRight: 1 }
      : { ArrowUp: -1, ArrowDown: 1 };
    const sign = keys[event.key];
    if (sign === undefined) {
      return;
    }
    event.preventDefault();
    onChange(clampSize(value + sign * direction * SPLIT_KEY_STEP, min, max()));
  };

  return (
    <div
      className={vertical ? "pg-split-v" : "pg-split-h"}
      role="separator"
      aria-orientation={orientation}
      aria-label={label}
      aria-valuenow={Math.round(value)}
      aria-valuemin={min}
      tabIndex={0}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerEnd}
      onPointerCancel={handlePointerEnd}
      onKeyDown={handleKeyDown}
    />
  );
}
