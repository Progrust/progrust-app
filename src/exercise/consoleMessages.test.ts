import { describe, expect, it } from "vitest";
import type { RunFinished } from "../commands/run";
import { busyText, finishedLines, separatorLine } from "./consoleMessages";

const base = {
  request_id: "r1",
  finished_at: "2026-11-24T10:00:00+09:00",
  error: null,
  run: null,
  submit: null,
  progress: null,
};

const run = (
  status: NonNullable<RunFinished["run"]>["status"],
  exitCode: number | null = null,
  mode: "main" | "test" = "main",
): RunFinished => ({
  ...base,
  kind: "run",
  run: { mode, status, exit_code: exitCode },
});

const submit = (
  status: NonNullable<RunFinished["submit"]>["status"],
  passed = 0,
  total = 0,
): RunFinished => ({
  ...base,
  kind: "submit",
  submit: { status, passed, total, cases: [] },
});

const texts = (event: RunFinished) => finishedLines(event).map((l) => l.text);

describe("区切りの行", () => {
  it("種類と、押したときの時と分を書く", () => {
    const at = new Date(2026, 10, 24, 9, 5);
    expect(separatorLine({ kind: "run", mode: "main" }, at)).toBe(
      "── 実行  09:05 ──",
    );
    expect(separatorLine({ kind: "run", mode: "test" }, at)).toBe(
      "── テスト  09:05 ──",
    );
    expect(separatorLine({ kind: "submit" }, at)).toBe("── 提出  09:05 ──");
  });
});

describe("終わりの行", () => {
  it("実行の終了コードと、ビルドの失敗と、停止を書く", () => {
    expect(texts(run("success", 0))).toEqual(["終了しました（終了コード 0）"]);
    expect(texts(run("runtime_error", 101))).toEqual([
      "終了しました（終了コード 101）",
    ]);
    expect(texts(run("runtime_error", null))).toEqual(["終了しました"]);
    expect(texts(run("compile_error"))).toEqual([
      "ビルドに失敗したため、実行していません。",
    ]);
    expect(texts(run("compile_error", null, "test"))).toEqual([
      "ビルドに失敗したため、テストを実行していません。",
    ]);
    expect(texts(run("stopped"))).toEqual(["停止しました"]);
    expect(finishedLines(run("success", 0))[0].kind).toBe("system");
  });

  it("タイムアウトと crates_unavailable は、バックエンドが知らせるので書かない", () => {
    expect(texts(run("timeout"))).toEqual([]);
    expect(texts(run("crates_unavailable"))).toEqual([]);
    expect(texts(submit("crates_unavailable"))).toEqual([]);
    expect(texts(submit("stopped"))).toEqual([]);
  });

  it("提出の結果を、経過の行として書く。不正解は赤の太字、正解は太字", () => {
    const wrong = finishedLines(submit("wrong", 2, 3));
    expect(wrong.map((l) => l.text)).toEqual([
      "テストを実行しました（3 / 3）",
      "3 件中 1 件のテストが失敗しました。",
    ]);
    expect(wrong.map((l) => l.kind)).toEqual(["progress", "progress"]);
    expect(wrong[1].style).toEqual({ color: "red", bold: true });
    const passed = finishedLines(submit("passed", 3, 3));
    expect(passed.map((l) => l.text)).toEqual([
      "テストを実行しました（3 / 3）",
      "3 件のテストがすべて通りました。",
    ]);
    expect(passed[1].style).toEqual({ color: null, bold: true });
    expect(texts(submit("compile_error"))).toEqual([
      "ビルドに失敗したため、テストを実行していません。",
    ]);
  });

  it("アプリの側の失敗は、エラーの message を書く", () => {
    expect(
      texts({
        ...base,
        kind: "submit",
        error: {
          code: "internal",
          message: "判定するプログラムが見つかりませんでした。",
          detail: null,
        },
      }),
    ).toEqual(["判定するプログラムが見つかりませんでした。"]);
  });
});

describe("見出しの行の進み具合", () => {
  it("提出は、ビルド中か、判定の completed / total を出す", () => {
    expect(busyText("running", null)).toBe("実行中");
    expect(busyText("testing", null)).toBe("テスト実行中");
    expect(busyText("submitting", null)).toBe("ビルド中");
    const progress = { request_id: "r1", completed: 0, total: null };
    expect(busyText("submitting", { ...progress, phase: "build" })).toBe(
      "ビルド中",
    );
    expect(busyText("submitting", { ...progress, phase: "judge" })).toBe(
      "判定中",
    );
    expect(
      busyText("submitting", {
        ...progress,
        phase: "judge",
        completed: 1,
        total: 3,
      }),
    ).toBe("判定中 1 / 3");
  });
});
