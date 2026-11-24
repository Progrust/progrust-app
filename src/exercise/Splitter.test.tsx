import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { SPLIT_KEY_STEP, Splitter, clampSize } from "./Splitter";

function Harness({
  orientation,
  direction,
}: {
  orientation: "vertical" | "horizontal";
  direction: 1 | -1;
}) {
  const [value, setValue] = useState(300);
  return (
    <Splitter
      orientation={orientation}
      label="境界"
      value={value}
      min={200}
      max={() => 400}
      direction={direction}
      onChange={setValue}
    />
  );
}

function separator() {
  return screen.getByRole("separator", { name: "境界" });
}

describe("clampSize", () => {
  it("最小と最大の間に収め、最大が最小より小さいときは最小にする", () => {
    expect(clampSize(50, 100, 200)).toBe(100);
    expect(clampSize(150, 100, 200)).toBe(150);
    expect(clampSize(250, 100, 200)).toBe(200);
    expect(clampSize(150, 100, 80)).toBe(100);
  });
});

describe("Splitter", () => {
  it("左右の境界をドラッグすると、動かした分だけ大きさを変え、範囲に収める", () => {
    render(<Harness orientation="vertical" direction={1} />);
    const handle = separator();
    expect(handle).toHaveAttribute("aria-orientation", "vertical");
    expect(handle).toHaveAttribute("aria-valuenow", "300");
    fireEvent.pointerDown(handle, { button: 0, clientX: 500, pointerId: 1 });
    fireEvent.pointerMove(handle, { clientX: 550, pointerId: 1 });
    expect(handle).toHaveAttribute("aria-valuenow", "350");
    fireEvent.pointerMove(handle, { clientX: 900, pointerId: 1 });
    expect(handle).toHaveAttribute("aria-valuenow", "400");
    fireEvent.pointerMove(handle, { clientX: 0, pointerId: 1 });
    expect(handle).toHaveAttribute("aria-valuenow", "200");
    fireEvent.pointerUp(handle, { clientX: 0, pointerId: 1 });
    // 離した後は動かない。
    fireEvent.pointerMove(handle, { clientX: 550, pointerId: 1 });
    expect(handle).toHaveAttribute("aria-valuenow", "200");
  });

  it("上下の境界で下の段の大きさを変えるときは、下へ動かすと小さくなる", () => {
    render(<Harness orientation="horizontal" direction={-1} />);
    const handle = separator();
    fireEvent.pointerDown(handle, { button: 0, clientY: 500, pointerId: 1 });
    fireEvent.pointerMove(handle, { clientY: 540, pointerId: 1 });
    expect(handle).toHaveAttribute("aria-valuenow", "260");
  });

  it("矢印キーで動かす", () => {
    render(<Harness orientation="vertical" direction={1} />);
    const handle = separator();
    fireEvent.keyDown(handle, { key: "ArrowRight" });
    expect(handle).toHaveAttribute(
      "aria-valuenow",
      String(300 + SPLIT_KEY_STEP),
    );
    fireEvent.keyDown(handle, { key: "ArrowLeft" });
    fireEvent.keyDown(handle, { key: "ArrowLeft" });
    expect(handle).toHaveAttribute(
      "aria-valuenow",
      String(300 - SPLIT_KEY_STEP),
    );
    // 左右の境界では、上下の矢印キーで動かない。
    fireEvent.keyDown(handle, { key: "ArrowUp" });
    expect(handle).toHaveAttribute(
      "aria-valuenow",
      String(300 - SPLIT_KEY_STEP),
    );
  });
});
