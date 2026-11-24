// 演習画面のキーボードショートカット（documents/spec/screens/exercise-screen.md「キーボードショートカット」、
// ADR 0111、ADR 0225）。押したキーがどのショートカットか、いま動かせるかを決める。
// 画面の組み立てから分けて、自動テストで確かめられるようにする。
import type { ExerciseControls } from "./exerciseState";

// 実行（Cmd+Enter）、テスト実行（Cmd+Shift+T）、提出（Cmd+Shift+Enter）、停止（Cmd+.）。
export type ShortcutAction = "run" | "test" | "submit" | "stop";

export const SHORTCUT_ACTIONS: readonly ShortcutAction[] = [
  "run",
  "test",
  "submit",
  "stop",
];

// Monaco のコマンドの一覧に出す名前。
export const SHORTCUT_LABELS: { [action in ShortcutAction]: string } = {
  run: "実行",
  test: "テスト実行",
  submit: "提出",
  stop: "停止",
};

// 判定に使う、キーのイベントの値。
export type ShortcutKey = Pick<
  KeyboardEvent,
  | "key"
  | "metaKey"
  | "ctrlKey"
  | "altKey"
  | "shiftKey"
  | "isComposing"
  | "keyCode"
>;

// 日本語の変換中のキーか。WKWebView では、変換中のキーが isComposing が false で、keyCode が 229 で
// 届くことがあるので、両方を見る（documents/verification/issue-21-monaco-in-tauri.md「日本語入力（IME）」）。
export function isComposingKey(event: ShortcutKey): boolean {
  return event.isComposing || event.keyCode === 229;
}

// 押したキーのショートカット。当てはまらないときと、変換中は null。
// 修飾キーは、Cmd（と、テスト実行・提出では Shift）だけを押しているときに限る。
export function shortcutForKey(event: ShortcutKey): ShortcutAction | null {
  if (isComposingKey(event)) {
    return null;
  }
  if (!event.metaKey || event.ctrlKey || event.altKey) {
    return null;
  }
  const key = event.key.toLowerCase();
  if (key === "enter") {
    return event.shiftKey ? "submit" : "run";
  }
  if (key === "t" && event.shiftKey) {
    return "test";
  }
  if (key === "." && !event.shiftKey) {
    return "stop";
  }
  return null;
}

// ショートカットを、いま動かせるか。ボタンの有効・無効と同じ。
export function shortcutEnabled(
  action: ShortcutAction,
  controls: ExerciseControls,
): boolean {
  switch (action) {
    case "run":
      return controls.runEnabled;
    case "test":
      return controls.testEnabled;
    case "submit":
      return controls.submitEnabled;
    case "stop":
      return controls.stopEnabled;
  }
}
