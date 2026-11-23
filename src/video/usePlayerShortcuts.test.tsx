import { fireEvent, render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { skipTarget } from "./playerMath";
import { usePlayerShortcuts } from "./usePlayerShortcuts";

function Harness(props: {
  onTogglePlay: () => void;
  onSkipBack: () => void;
  onSkipForward: () => void;
}) {
  usePlayerShortcuts(props);
  return (
    <div>
      <input aria-label="チャット" />
      <textarea aria-label="メモ" />
      <div contentEditable suppressContentEditableWarning data-testid="edit">
        <span data-testid="inner">編集</span>
      </div>
      <button>ボタン</button>
    </div>
  );
}

function setup() {
  const handlers = {
    onTogglePlay: vi.fn(),
    onSkipBack: vi.fn(),
    onSkipForward: vi.fn(),
  };
  const view = render(<Harness {...handlers} />);
  return { handlers, ...view };
}

describe("usePlayerShortcuts", () => {
  it("Space で再生と一時停止を切り替える", () => {
    const { handlers } = setup();
    fireEvent.keyDown(document.body, { code: "Space", key: " " });
    expect(handlers.onTogglePlay).toHaveBeenCalledTimes(1);
  });

  it("← で 5 秒戻り、→ で 5 秒進む", () => {
    const { handlers } = setup();
    fireEvent.keyDown(document.body, { code: "ArrowLeft", key: "ArrowLeft" });
    fireEvent.keyDown(document.body, { code: "ArrowRight", key: "ArrowRight" });
    expect(handlers.onSkipBack).toHaveBeenCalledTimes(1);
    expect(handlers.onSkipForward).toHaveBeenCalledTimes(1);
    expect(handlers.onTogglePlay).not.toHaveBeenCalled();
  });

  it("→ を押すと、いまの位置の 5 秒後に移る", () => {
    const timeline = { scenes: [], durationInFrames: 3000 };
    let frame = 600;
    const handlers = {
      onTogglePlay: vi.fn(),
      onSkipBack: vi.fn(),
      onSkipForward: () => {
        frame = skipTarget(timeline, frame, "forward");
      },
    };
    render(<Harness {...handlers} />);
    fireEvent.keyDown(document.body, { code: "ArrowRight", key: "ArrowRight" });
    expect(frame).toBe(750);
    expect(handlers.onTogglePlay).not.toHaveBeenCalled();
  });

  it("文字の入力欄にフォーカスがあるときは効かない", () => {
    const { handlers, getByLabelText, getByTestId } = setup();
    for (const element of [
      getByLabelText("チャット"),
      getByLabelText("メモ"),
      getByTestId("edit"),
      getByTestId("inner"),
    ]) {
      fireEvent.keyDown(element, { code: "Space", key: " " });
      fireEvent.keyDown(element, { code: "ArrowLeft", key: "ArrowLeft" });
      fireEvent.keyDown(element, { code: "ArrowRight", key: "ArrowRight" });
    }
    expect(handlers.onTogglePlay).not.toHaveBeenCalled();
    expect(handlers.onSkipBack).not.toHaveBeenCalled();
    expect(handlers.onSkipForward).not.toHaveBeenCalled();
  });

  it("修飾キーを押しているときは効かない", () => {
    const { handlers } = setup();
    fireEvent.keyDown(document.body, { code: "ArrowRight", metaKey: true });
    fireEvent.keyDown(document.body, { code: "ArrowLeft", shiftKey: true });
    expect(handlers.onSkipForward).not.toHaveBeenCalled();
    expect(handlers.onSkipBack).not.toHaveBeenCalled();
  });

  it("押しっぱなしの Space では、繰り返し切り替えない", () => {
    const { handlers } = setup();
    fireEvent.keyDown(document.body, { code: "Space", repeat: true });
    expect(handlers.onTogglePlay).not.toHaveBeenCalled();
  });

  it("取り外したあとは効かない", () => {
    const { handlers, unmount } = setup();
    unmount();
    fireEvent.keyDown(document.body, { code: "Space", key: " " });
    expect(handlers.onTogglePlay).not.toHaveBeenCalled();
  });
});
