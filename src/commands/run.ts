// 実行・提出のコマンドとイベント（documents/spec/architecture/commands/run.md）。
import { invoke } from "@tauri-apps/api/core";

import type { DraftFiles, LessonProgress } from "./progress";

// main は実行、test はテスト実行（学習者が書いたテストを cargo test で動かす）。
export type RunMode = "main" | "test";

// 実行の結果の状態（documents/spec/exercise-runner/run.md「結果の状態」）。
export type RunStatus =
  | "success"
  | "compile_error"
  | "runtime_error"
  | "timeout"
  | "stopped"
  | "crates_unavailable";

// コマンドが失敗したときの値と、run_finished の error の形
// （documents/spec/architecture/commands/conventions.md「エラー」）。
export type CommandError = {
  code:
    | "invalid_argument"
    | "not_found"
    | "invalid_state"
    | "busy"
    | "cargo_unavailable"
    | "claude_unavailable"
    | "crates_not_ready"
    | "io_error"
    | "internal";
  // 学習者にそのまま見せてよい日本語の説明。
  message: string;
  detail: string | null;
};

// イベントの名前。受け取り（listen）を始めてから、run_code・submit_code を呼ぶ。
export const RUN_STARTED = "run_started";
export const RUN_OUTPUT = "run_output";
export const SUBMIT_PROGRESS = "submit_progress";
export const RUN_FINISHED = "run_finished";

// イベント run_started の内容。実行（mode が main）で学習者のプログラムを起動したときに 1 回届く。
// 届いた後は、プログラムが終わるか標準入力を閉じるまで、sendStdin で書ける。テスト実行と提出と、
// プログラムを起動しなかったとき（ビルドの失敗など）は届かない。プログラムの出力の run_output より後に届くことがある。
export type RunStarted = {
  request_id: string;
};

// run_output の断片。stdout・stderr は学習者のプログラム（テスト実行では cargo test）とビルドの出力、
// system はアプリが出す知らせ（日本語）。ANSI エスケープを含むことがある。
export type RunOutputChunk = {
  stream: "stdout" | "stderr" | "system";
  text: string;
};

// イベント run_output の内容。
export type RunOutput = {
  request_id: string;
  // 受け取った順に並ぶ。
  chunks: RunOutputChunk[];
};

// イベント submit_progress の内容。提出の判定の段階が変わったときと、テストケースが 1 件終わるたびに届く。
export type SubmitProgress = {
  request_id: string;
  // build はビルド中、judge はテストケースの実行中。
  phase: "build" | "judge";
  // 結果が決まったテストケースの数。build では 0。
  completed: number;
  // 全テストケースの数。決まる前（build と、test 方式で隠しテストの名前を得る前）は null。
  total: number | null;
};

// 提出の判定結果の状態（documents/spec/exercise-runner/submit.md「判定結果」）。
// crates_unavailable は判定をしていない提出、stopped は途中で止めた提出。
export type SubmitStatus =
  "passed" | "wrong" | "compile_error" | "crates_unavailable" | "stopped";

// テストケースの結果の状態。
export type CaseStatus = "pass" | "fail" | "timeout" | "runtime_error";

// stdout 方式の fail の detail。どれも正規化する前のもの。
export type StdoutFailDetail = {
  // テストケースの標準入力。
  input: string;
  // 期待する出力。
  expected: string;
  // 実際の出力。出力過多のときは、上限までの標準出力。
  actual: string;
};

// テストケース 1 件の結果。
export type CaseResult = {
  // 表示名。
  name: string;
  status: CaseStatus;
  // pass では null。stdout 方式の fail ではオブジェクト。ほか（test 方式の fail の panic メッセージ、
  // 学習者のテストの fail の説明、timeout・runtime_error の標準エラー出力の先頭）は文字列。
  detail: StdoutFailDetail | string | null;
};

// run_finished の submit（提出の判定結果）。
export type SubmitResult = {
  status: SubmitStatus;
  // 通過したテストケースの数。
  passed: number;
  // 全テストケースの数。compile_error・crates_unavailable・stopped では passed とともに 0。
  total: number;
  // テストケースごとの結果。compile_error・crates_unavailable・stopped では空。
  cases: CaseResult[];
};

// run_finished の run。
export type RunResult = {
  mode: RunMode;
  status: RunStatus;
  // 学習者のプログラム（テスト実行では cargo test）の終了コード。起動していないときと、
  // シグナルで終わったとき（停止・時間切れ・出力量の上限を含む）は null。
  exit_code: number | null;
};

// イベント run_finished の内容。実行・提出が終わったときに 1 回届く。
export type RunFinished = {
  request_id: string;
  kind: "run" | "submit";
  // 終わった日時（RFC 3339）。
  finished_at: string;
  // アプリの側の失敗で、結果の状態を決められずに終わったときのエラー。このとき run・submit・progress は null。
  error: CommandError | null;
  // kind が run のときの結果。
  run: RunResult | null;
  // kind が submit のときの判定結果。
  submit: SubmitResult | null;
  // 提出で正解してレッスンを完了にしたときの、更新した後の進捗。
  progress: LessonProgress | null;
};

// 「実行」（mode が main）と「テスト」（test）。files を下書きに保存してから実行を始め、始めたら null で解決する。
// main では、プログラムを起動したことが run_started で届く。出力は run_output、結果は run_finished で届く。
// ほかの実行・提出が動いているときは busy で拒否される。
export function runCode(
  requestId: string,
  lessonId: string,
  mode: RunMode,
  files: DraftFiles,
): Promise<null> {
  return invoke<null>("run_code", {
    request_id: requestId,
    lesson_id: lessonId,
    mode,
    files,
  });
}

// 「提出」。files を下書きに保存してから提出を始め、始めたら null で解決する。ビルドの出力は run_output、
// 判定の進み具合は submit_progress、判定結果は run_finished で届く。正解すると、run_finished の progress に
// 完了にした後の進捗が入る。ほかの実行・提出が動いているときは busy で拒否される。
export function submitCode(
  requestId: string,
  lessonId: string,
  files: DraftFiles,
): Promise<null> {
  return invoke<null>("submit_code", {
    request_id: requestId,
    lesson_id: lessonId,
    files,
  });
}

// 入力欄の 1 行に改行（\n）を付けて、実行中のプログラムの標準入力に書く。
// ビルド中（run_started が届く前）・終わった後・標準入力を閉じた後は invalid_state で拒否される。
export function sendStdin(requestId: string, text: string): Promise<null> {
  return invoke<null>("send_stdin", { request_id: requestId, text });
}

// 「入力終了」で、実行中のプログラムの標準入力を閉じる。拒否される場合は sendStdin と同じ。
export function closeStdin(requestId: string): Promise<null> {
  return invoke<null>("close_stdin", { request_id: requestId });
}

// 停止ボタンと、演習画面を離れるときに、実行・テスト実行・提出を止める。止めた結果は run_finished で届く。
// 動いていない ID のときは、何もせずに null で解決する。
export function stopRun(requestId: string): Promise<null> {
  return invoke<null>("stop_run", { request_id: requestId });
}
