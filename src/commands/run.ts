// 実行のコマンドとイベント（documents/spec/architecture/commands/run.md）。
// 提出（submit_code・submit_progress）と、run_finished の submit の形は、提出を作るときに足す。
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

// イベントの名前。受け取り（listen）を始めてから、run_code を呼ぶ。
export const RUN_OUTPUT = "run_output";
export const RUN_FINISHED = "run_finished";

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
  submit: unknown | null;
  // 提出で正解してレッスンを完了にしたときの、更新した後の進捗。
  progress: LessonProgress | null;
};

// 「実行」（mode が main）と「テスト」（test）。files を下書きに保存してから実行を始め、始めたら null で解決する。
// 出力は run_output、結果は run_finished で届く。ほかの実行・提出が動いているときは busy で拒否される。
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

// 入力欄の 1 行に改行（\n）を付けて、実行中のプログラムの標準入力に書く。
// ビルド中・終わった後・標準入力を閉じた後は invalid_state で拒否される。
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
