import { describe, expect, it } from "vitest";
import type { RunFinished } from "../commands/run";
import {
  INITIAL_EXERCISE_STATE,
  activeRequestId,
  exerciseControls,
  exerciseReducer,
  type ExerciseState,
} from "./exerciseState";

function finished(
  requestId: string,
  kind: "run" | "submit",
  submitStatus?: "passed" | "wrong",
  error = false,
): RunFinished {
  return {
    request_id: requestId,
    kind,
    finished_at: "2026-11-24T10:00:00+09:00",
    error: error ? { code: "internal", message: "失敗", detail: null } : null,
    run:
      kind === "run" && !error
        ? { mode: "main", status: "success", exit_code: 0 }
        : null,
    submit:
      kind === "submit" && !error && submitStatus
        ? { status: submitStatus, passed: 0, total: 0, cases: [] }
        : null,
    progress: null,
  };
}

const running: ExerciseState = {
  kind: "running",
  requestId: "r1",
  stage: "build",
  stdinClosed: false,
};

describe("状態の移り変わり", () => {
  it("実行を始めるとビルド中になり、run_started が届くとプログラムの実行中になる", () => {
    let s = exerciseReducer(INITIAL_EXERCISE_STATE, {
      type: "started",
      kind: "run",
      mode: "main",
      requestId: "r1",
    });
    expect(s).toEqual(running);
    s = exerciseReducer(s, { type: "program_started", requestId: "r1" });
    expect(s).toMatchObject({ kind: "running", stage: "program" });
  });

  it("プロンプトを出さないプログラムでも、起動したら出力を待たずに入力欄が有効になる", () => {
    let s = exerciseReducer(INITIAL_EXERCISE_STATE, {
      type: "started",
      kind: "run",
      mode: "main",
      requestId: "r1",
    });
    expect(exerciseControls(s, true).stdinEnabled).toBe(false);
    s = exerciseReducer(s, { type: "program_started", requestId: "r1" });
    expect(exerciseControls(s, true)).toMatchObject({
      stdinVisible: true,
      stdinEnabled: true,
    });
  });

  it("ほかの要求の run_started と終了は無視する", () => {
    const s = exerciseReducer(running, {
      type: "program_started",
      requestId: "x",
    });
    expect(s).toBe(running);
    expect(
      exerciseReducer(running, {
        type: "finished",
        event: finished("x", "run"),
      }),
    ).toBe(running);
  });

  it("テスト実行は testing になり、終わると待機に戻る", () => {
    const s = exerciseReducer(INITIAL_EXERCISE_STATE, {
      type: "started",
      kind: "run",
      mode: "test",
      requestId: "t1",
    });
    expect(s).toEqual({ kind: "testing", requestId: "t1" });
    expect(
      exerciseReducer(s, { type: "finished", event: finished("t1", "run") }),
    ).toEqual(INITIAL_EXERCISE_STATE);
  });

  it("提出で正解すると解説表示、不正解なら待機に戻る", () => {
    const s = exerciseReducer(INITIAL_EXERCISE_STATE, {
      type: "started",
      kind: "submit",
      requestId: "s1",
    });
    expect(s).toEqual({ kind: "submitting", requestId: "s1" });
    expect(
      exerciseReducer(s, {
        type: "finished",
        event: finished("s1", "submit", "passed"),
      }),
    ).toEqual({ kind: "explanation" });
    expect(
      exerciseReducer(s, {
        type: "finished",
        event: finished("s1", "submit", "wrong"),
      }),
    ).toEqual(INITIAL_EXERCISE_STATE);
    expect(
      exerciseReducer(s, {
        type: "finished",
        event: finished("s1", "submit", undefined, true),
      }),
    ).toEqual(INITIAL_EXERCISE_STATE);
  });

  it("テスト実行中・提出中・待機と、既にプログラムの実行中のときは、run_started で状態を変えない", () => {
    for (const s of [
      { kind: "testing", requestId: "r1" } as const,
      { kind: "submitting", requestId: "r1" } as const,
      INITIAL_EXERCISE_STATE,
    ]) {
      expect(
        exerciseReducer(s, { type: "program_started", requestId: "r1" }),
      ).toBe(s);
    }
    const program: ExerciseState = { ...running, stage: "program" };
    expect(
      exerciseReducer(program, { type: "program_started", requestId: "r1" }),
    ).toBe(program);
  });

  it("処理中と解説表示では新しく始められない", () => {
    const action = {
      type: "started",
      kind: "submit",
      requestId: "n",
    } as const;
    expect(exerciseReducer(running, action)).toBe(running);
    expect(exerciseReducer({ kind: "explanation" }, action)).toEqual({
      kind: "explanation",
    });
  });

  it("コマンドが拒否されたら待機に戻る", () => {
    expect(
      exerciseReducer(running, { type: "start_failed", requestId: "r1" }),
    ).toEqual(INITIAL_EXERCISE_STATE);
  });

  it("動いている要求の ID を返す", () => {
    expect(activeRequestId(running)).toBe("r1");
    expect(activeRequestId(INITIAL_EXERCISE_STATE)).toBeNull();
    expect(activeRequestId({ kind: "explanation" })).toBeNull();
  });
});

describe("ボタンの有効・無効", () => {
  it("待機では実行・テスト・提出が有効で、停止は無効", () => {
    expect(exerciseControls(INITIAL_EXERCISE_STATE, true)).toMatchObject({
      runEnabled: true,
      testEnabled: true,
      submitEnabled: true,
      stopEnabled: false,
      stdinEnabled: false,
      editorReadOnly: false,
    });
  });

  it("提出中は停止ボタンが無効で、実行・テスト実行中は有効になる", () => {
    expect(
      exerciseControls({ kind: "submitting", requestId: "s" }, true)
        .stopEnabled,
    ).toBe(false);
    expect(exerciseControls(running, true).stopEnabled).toBe(true);
    expect(
      exerciseControls({ kind: "testing", requestId: "t" }, true).stopEnabled,
    ).toBe(true);
  });

  it("処理中は実行・テスト・提出が無効", () => {
    for (const s of [
      running,
      { kind: "testing", requestId: "t" } as const,
      { kind: "submitting", requestId: "s" } as const,
    ]) {
      expect(exerciseControls(s, true)).toMatchObject({
        runEnabled: false,
        testEnabled: false,
        submitEnabled: false,
      });
    }
  });

  it("入力欄は main の実行でプログラムが動いている間だけ有効で、テスト実行では表示しない", () => {
    expect(exerciseControls(running, true).stdinEnabled).toBe(false);
    const program: ExerciseState = { ...running, stage: "program" };
    expect(exerciseControls(program, true)).toMatchObject({
      stdinVisible: true,
      stdinEnabled: true,
    });
    expect(
      exerciseControls({ ...program, stdinClosed: true }, true).stdinEnabled,
    ).toBe(false);
    expect(
      exerciseControls({ kind: "testing", requestId: "t" }, true),
    ).toMatchObject({ stdinVisible: false, stdinEnabled: false });
  });

  it("解説表示ではエディタが読み取り専用で、提出できない", () => {
    expect(exerciseControls({ kind: "explanation" }, true)).toMatchObject({
      editorReadOnly: true,
      submitEnabled: false,
      runEnabled: false,
    });
  });

  it("cargo が見つからないときは実行・テスト・提出が無効", () => {
    expect(exerciseControls(INITIAL_EXERCISE_STATE, false)).toMatchObject({
      runEnabled: false,
      testEnabled: false,
      submitEnabled: false,
    });
  });
});
