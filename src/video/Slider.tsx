// 棒の上のつまみで値を示す・変える部品（design-system/progrust/components/Slider）。
// 動画のプレーヤーの再生位置（シークバー）と音量に使う。
import { useRef } from "react";
import type { CSSProperties, PointerEvent } from "react";

export type SliderProps = {
  label: string;
  // 0 から 1 の割合。
  ratio: number;
  // aria-valuemin・aria-valuemax・aria-valuenow・aria-valuetext。
  min: number;
  max: number;
  now: number;
  valueText?: string;
  // シークバー（章の色）。付けないときは墨色。
  accent?: boolean;
  // 印の位置（0 から 1 の割合）。
  marks?: number[];
  disabled?: boolean;
  // 押した位置・ドラッグした位置の割合。
  onChange: (ratio: number) => void;
};

// 押した位置の、棒の左端からの割合。
function ratioAt(element: HTMLElement, clientX: number): number {
  const rect = element.getBoundingClientRect();
  if (rect.width <= 0) {
    return 0;
  }
  return Math.min(Math.max((clientX - rect.left) / rect.width, 0), 1);
}

export function Slider({
  label,
  ratio,
  min,
  max,
  now,
  valueText,
  accent = false,
  marks = [],
  disabled = false,
  onChange,
}: SliderProps) {
  const dragging = useRef(false);

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (disabled || event.button !== 0) {
      return;
    }
    dragging.current = true;
    // jsdom など、ポインタの捕捉が無い環境でも動くようにする。
    event.currentTarget.setPointerCapture?.(event.pointerId);
    onChange(ratioAt(event.currentTarget, event.clientX));
  };
  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (!dragging.current || disabled) {
      return;
    }
    onChange(ratioAt(event.currentTarget, event.clientX));
  };
  const endDrag = (event: PointerEvent<HTMLDivElement>) => {
    dragging.current = false;
    if (event.currentTarget.hasPointerCapture?.(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  };

  return (
    <div
      className={accent ? "pg-slider pg-slider--accent" : "pg-slider"}
      role="slider"
      tabIndex={disabled ? -1 : 0}
      aria-label={label}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={now}
      aria-valuetext={valueText}
      aria-disabled={disabled ? "true" : undefined}
      style={{ "--pg-slider-value": ratio } as CSSProperties}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
    >
      <span className="pg-slider-track">
        <span className="pg-slider-fill" />
        {marks.map((mark, index) => (
          <span
            key={index}
            className="pg-slider-mark"
            style={{ left: `${mark * 100}%` }}
          />
        ))}
      </span>
      <span className="pg-slider-thumb" />
    </div>
  );
}
