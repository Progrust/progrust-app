// 演習画面の状態の移り変わりと、ボタンの有効・無効
// （documents/spec/screens/exercise-screen.md、documents/spec/lesson-flow/flow.md、
// documents/spec/architecture/commands/run.md）。画面の組み立てから分けて、自動テストで確かめられるようにする。
import type { RunFinished, RunMode } from "../commands/run";

// 演習画面の状態。
// - idle: 待機（解答中で、何も動いていない）。
// - running: 実行中（mode が main）。stage の build はビルド中、program はプログラムが動いている間。
// - testing: テスト実行中。標準入力は使わない。
// - submitting: 提出中。
// - explanation: 解説表示（正解の後）。
export type ExerciseState =
  | { kind: "idle" }
  | {
      kind: "running";
      requestId: string;
      stage: "build" | "program";
      // 「入力終了」で標準入力を閉じた後か。
      stdinClosed: boolean;
    }
  | { kind: "testing"; requestId: string }
  | { kind: "submitting"; requestId: string }
  | { kind: "explanation" };

export const INITIAL_EXERCISE_STATE: ExerciseState = { kind: "idle" };

export type ExerciseAction =
  // 「実行」「テスト」「提出」を押して、コマンドを呼んだ。
  | { type: "started"; kind: "run"; mode: RunMode; requestId: string }
  | { type: "started"; kind: "submit"; requestId: string }
  // 実行の出力が届いた。ビルドの出力はビルドが終わってから送られる（documents/spec/exercise-runner/run.md
  // 「ビルドの出力の表示」）ので、実行（main）では、出力が届いたら、プログラムが動いている段階に移る。
  | { type: "output"; requestId: string }
  // 「入力終了」で標準入力を閉じた。
  | { type: "stdin_closed"; requestId: string }
  // run_finished が届いた。
  | { type: "finished"; event: RunFinished }
  // 呼んだコマンドが拒否された（busy など）。始まらなかったので、待機に戻す。
  | { type: "start_failed"; requestId: string };

// 動いている要求の ID。無いときは null。
export function activeRequestId(state: ExerciseState): string | null {
  switch (state.kind) {
    case "running":
    case "testing":
    case "submitting":
      return state.requestId;
    case "idle":
    case "explanation":
      return null;
  }
}

export function exerciseReducer(
  state: ExerciseState,
  action: ExerciseAction,
): ExerciseState {
  switch (action.type) {
    case "started": {
      // 処理中と解説表示では、新しく始められない（ボタンも無効）。
      if (state.kind !== "idle") {
        return state;
      }
      if (action.kind === "submit") {
        return { kind: "submitting", requestId: action.requestId };
      }
      return action.mode === "main"
        ? {
            kind: "running",
            requestId: action.requestId,
            stage: "build",
            stdinClosed: false,
          }
        : { kind: "testing", requestId: action.requestId };
    }
    case "output":
      if (
        state.kind === "running" &&
        state.requestId === action.requestId &&
        state.stage === "build"
      ) {
        return { ...state, stage: "program" };
      }
      return state;
    case "stdin_closed":
      if (state.kind === "running" && state.requestId === action.requestId) {
        return { ...state, stdinClosed: true };
      }
      return state;
    case "start_failed":
      return activeRequestId(state) === action.requestId
        ? INITIAL_EXERCISE_STATE
        : state;
    case "finished": {
      const { event } = action;
      if (activeRequestId(state) !== event.request_id) {
        return state;
      }
      // 提出で正解したときだけ、解説表示に移る。ほかは待機に戻る。
      if (
        state.kind === "submitting" &&
        event.error === null &&
        event.submit?.status === "passed"
      ) {
        return { kind: "explanation" };
      }
      return INITIAL_EXERCISE_STATE;
    }
  }
}

// 画面のボタンなどの有効・無効。
export type ExerciseControls = {
  runEnabled: boolean;
  testEnabled: boolean;
  submitEnabled: boolean;
  // 実行・テスト実行のビルド中と実行中だけ有効。提出中は無効。
  stopEnabled: boolean;
  // 標準入力の入力欄を表示するか。テスト実行では表示しない。
  stdinVisible: boolean;
  // 入力欄に入力できるか。main の実行でプログラムが動いている間だけ。
  stdinEnabled: boolean;
  // 解説表示では、エディタを読み取り専用にする。
  editorReadOnly: boolean;
};

// cargoFound: cargo が見つかっているか。見つからないときは、実行・テスト・提出を無効にする。
export function exerciseControls(
  state: ExerciseState,
  cargoFound: boolean,
): ExerciseControls {
  const canStart = cargoFound && state.kind === "idle";
  return {
    runEnabled: canStart,
    testEnabled: canStart,
    submitEnabled: canStart,
    stopEnabled: state.kind === "running" || state.kind === "testing",
    stdinVisible: state.kind !== "testing",
    stdinEnabled:
      state.kind === "running" &&
      state.stage === "program" &&
      !state.stdinClosed,
    editorReadOnly: state.kind === "explanation",
  };
}
