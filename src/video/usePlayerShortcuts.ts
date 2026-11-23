// 動画画面のキーボードショートカット（documents/spec/screens/video-screen.md「キーボードショートカット」）。
// Space で再生と一時停止、← / → で 5 秒戻る・進む。
import { useEffect, useRef } from "react";

export type PlayerShortcutHandlers = {
  onTogglePlay: () => void;
  onSkipBack: () => void;
  onSkipForward: () => void;
};

// 文字の入力欄（input・textarea・contenteditable の要素）か。
function isTextInput(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target.isContentEditable ||
    target.closest("[contenteditable]:not([contenteditable='false'])") !== null
  );
}

export function usePlayerShortcuts(handlers: PlayerShortcutHandlers): void {
  // 毎回 window に付け直さないよう、最新の関数を ref に持つ。
  const ref = useRef(handlers);
  useEffect(() => {
    ref.current = handlers;
  });

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (
        event.defaultPrevented ||
        event.ctrlKey ||
        event.metaKey ||
        event.altKey ||
        event.shiftKey ||
        isTextInput(event.target)
      ) {
        return;
      }
      switch (event.code) {
        case "Space":
          if (event.repeat) {
            return;
          }
          event.preventDefault();
          ref.current.onTogglePlay();
          return;
        case "ArrowLeft":
          event.preventDefault();
          ref.current.onSkipBack();
          return;
        case "ArrowRight":
          event.preventDefault();
          ref.current.onSkipForward();
          return;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}
