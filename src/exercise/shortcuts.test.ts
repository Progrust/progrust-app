import { describe, expect, it } from "vitest";
import { exerciseControls } from "./exerciseState";
import type { ExerciseState } from "./exerciseState";
import { shortcutEnabled, shortcutForKey } from "./shortcuts";
import type { ShortcutAction, ShortcutKey } from "./shortcuts";

function key(value: string, overrides: Partial<ShortcutKey> = {}): ShortcutKey {
  return {
    key: value,
    metaKey: true,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    isComposing: false,
    keyCode: 0,
    ...overrides,
  };
}

describe("shortcutForKey", () => {
  it.each<[string, ShortcutKey, ShortcutAction]>([
    ["Cmd+Enter は実行", key("Enter"), "run"],
    ["Cmd+Shift+T はテスト実行", key("T", { shiftKey: true }), "test"],
    ["小文字の t でもテスト実行", key("t", { shiftKey: true }), "test"],
    ["Cmd+Shift+Enter は提出", key("Enter", { shiftKey: true }), "submit"],
    ["Cmd+. は停止", key("."), "stop"],
  ])("%s", (_, event, action) => {
    expect(shortcutForKey(event)).toBe(action);
  });

  it.each<[string, ShortcutKey]>([
    ["Cmd を押していない Enter", key("Enter", { metaKey: false })],
    ["Ctrl+Enter", key("Enter", { metaKey: false, ctrlKey: true })],
    ["Cmd+Ctrl+Enter", key("Enter", { ctrlKey: true })],
    ["Cmd+Option+Enter", key("Enter", { altKey: true })],
    ["Shift の無い Cmd+T", key("t")],
    ["Cmd+Shift+.", key(".", { shiftKey: true })],
    ["Cmd+Option+.", key(".", { altKey: true })],
    ["ほかのキー（Cmd+S）", key("s")],
    ["Cmd だけ", key("Meta")],
  ])("%s は、ショートカットではない", (_, event) => {
    expect(shortcutForKey(event)).toBeNull();
  });

  it.each<[string, ShortcutKey]>([
    ["isComposing が true", key("Enter", { isComposing: true })],
    ["keyCode が 229", key("Enter", { keyCode: 229 })],
    [
      "テスト実行のキーで keyCode が 229",
      key("t", { shiftKey: true, keyCode: 229 }),
    ],
    [
      "提出のキーで isComposing が true",
      key("Enter", { shiftKey: true, isComposing: true }),
    ],
    ["停止のキーで keyCode が 229", key(".", { keyCode: 229 })],
  ])("日本語の変換中（%s）は、ショートカットとして扱わない", (_, event) => {
    expect(shortcutForKey(event)).toBeNull();
  });
});

describe("shortcutEnabled", () => {
  const enabled = (state: ExerciseState, cargoFound = true) =>
    Object.fromEntries(
      (["run", "test", "submit", "stop"] as const).map((action) => [
        action,
        shortcutEnabled(action, exerciseControls(state, cargoFound)),
      ]),
    );

  it("待機中は、実行・テスト実行・提出だけを動かせる", () => {
    expect(enabled({ kind: "idle" })).toEqual({
      run: true,
      test: true,
      submit: true,
      stop: false,
    });
  });

  it("実行中とテスト実行中は、停止だけを動かせる", () => {
    const off = { run: false, test: false, submit: false, stop: true };
    expect(
      enabled({
        kind: "running",
        requestId: "r1",
        stage: "build",
        stdinClosed: false,
      }),
    ).toEqual(off);
    expect(enabled({ kind: "testing", requestId: "r1" })).toEqual(off);
  });

  it("提出中と解説表示では、どれも動かせない", () => {
    const off = { run: false, test: false, submit: false, stop: false };
    expect(enabled({ kind: "submitting", requestId: "r1" })).toEqual(off);
    expect(enabled({ kind: "explanation" })).toEqual(off);
  });

  it("cargo が見つからないときは、実行・テスト実行・提出を動かせない", () => {
    expect(enabled({ kind: "idle" }, false)).toEqual({
      run: false,
      test: false,
      submit: false,
      stop: false,
    });
  });
});
