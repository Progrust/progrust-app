// 下書きを保存するきっかけ（documents/spec/lesson-flow/drafts.md）。
// 最後の編集から 1 秒後に 1 回、画面を離れるときに編集が残っていれば 1 回、保存する。
// 実行・提出の直前の保存は、run_code・submit_code が行う。
import { useEffect, useState } from "react";

export const DRAFT_SAVE_DELAY_MS = 1000;

export type DraftSaveTrigger = "idle" | "leave";

export type DraftSaveScheduler = {
  // 編集した。タイマーを 1 秒に延ばす。
  edited: () => void;
  // 画面を離れる。まだ保存していない編集があれば、すぐ保存する。
  leave: () => void;
  // 保存し終えた（実行・提出の直前の保存など）ので、待っている保存をやめる。
  cancel: () => void;
  // 保存する関数を差し替える（フックが、最新の関数を呼ぶために使う）。
  setSave: (save: (trigger: DraftSaveTrigger) => void) => void;
};

export function createDraftSaveScheduler(
  initialSave: (trigger: DraftSaveTrigger) => void,
  delayMs: number = DRAFT_SAVE_DELAY_MS,
): DraftSaveScheduler {
  let save = initialSave;
  let timer: ReturnType<typeof setTimeout> | null = null;
  const cancel = () => {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  };
  return {
    edited() {
      cancel();
      timer = setTimeout(() => {
        timer = null;
        save("idle");
      }, delayMs);
    },
    leave() {
      if (timer !== null) {
        cancel();
        save("leave");
      }
    },
    cancel,
    setSave(next) {
      save = next;
    },
  };
}

// 演習画面で使う。save は最新のものを呼ぶ。画面を離れる（アンマウントする）と、leave を行う。
export function useDraftSaveTrigger(
  save: (trigger: DraftSaveTrigger) => void,
): DraftSaveScheduler {
  const [scheduler] = useState(() => createDraftSaveScheduler(save));
  useEffect(() => {
    scheduler.setSave(save);
  });
  useEffect(() => () => scheduler.leave(), [scheduler]);
  return scheduler;
}
