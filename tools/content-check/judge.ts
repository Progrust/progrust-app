// 検査「模範解答」「初期コード」（documents/spec/content-guidelines/check-tool.md）。
//
// アプリと同じ判定処理で判定するために、バックエンドのクレートに置いた判定だけを行うプログラム
// （src-tauri/src/bin/progrust-judge.rs）を `cargo run` で起動し、判定結果を JSON で受け取る（ADR 0208）。
//
// - 演習のあるレッスン（動画のみのレッスンと準備中のレッスンを除く）ごとに、模範解答（solution/）と
//   初期コード（starter/）を、この順に 1 つずつ判定する。作業領域が 1 つなので、同時には判定しない。
// - 模範解答の判定結果が passed でなければ「模範解答」、初期コードの判定結果が passed なら「初期コード」のエラーにする。
// - 判定できなかったとき（プログラムが 0 以外で終わった、JSON を読めないなど）も、その検査のエラーにする。
// - 許可クレートを使う演習（dependencies が空でない演習）も、ほかの演習と同じく判定する。判定の前に、判定だけを行う
//   プログラムが、判定の作業領域で初回準備をする（ADR 0242）。初回準備にはネットワークが要る。初回準備に失敗したときは、
//   プログラムが 0 以外で終わるので、その検査のエラーになる。
// - lesson.json が読めないレッスン、exercise が無いレッスン、solution/・starter/ が無いレッスンは判定しない
//   （「構成」「定義」が報告する）。
import { spawnSync } from "node:child_process";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  isDirectory,
  isPrepared,
  readLessonEntries,
  readLessonJson,
  type LessonEntry,
} from "./lesson-files.ts";
import type { CheckOptions, Finding } from "./types.ts";

export const SOLUTION_CHECK = "模範解答";
export const STARTER_CHECK = "初期コード";

const ROOT = fileURLToPath(new URL("../..", import.meta.url));
// 判定だけを行うプログラムの名前（src-tauri/src/bin/progrust-judge.rs）。
export const JUDGE_BIN = "progrust-judge";
// このプログラムをビルドする Cargo の feature。アプリの .app には入れない（ADR 0222）。
const JUDGE_FEATURE = "content-check";
// ビルドの出力を、エラーの文に入れる行数の上限。
const BUILD_OUTPUT_LINES = 20;

type Files = "solution" | "starter";

// 判定だけを行うプログラムが返す判定結果（spec/exercise-runner/submit.md「判定結果」に、ビルドの出力を足したもの）。
interface Judgement {
  status: string;
  passed: number;
  total: number;
  cases: { name: string; status: string; detail: unknown }[];
  build_output: string;
}

type JudgeRun =
  { ok: true; judgement: Judgement } | { ok: false; message: string };

// 判定の検査の対象にするか。対象にしないときは null。
function exerciseDir(entry: LessonEntry): string | null {
  if (isPrepared(entry)) return null;
  const lesson = readLessonJson(entry);
  if (lesson.state !== "ok") return null;
  if (lesson.value.kind === "video_only") return null;
  const exercise = lesson.value.exercise;
  if (typeof exercise !== "object" || exercise === null) return null;
  return join(entry.dir, "exercise");
}

function runJudge(
  entry: LessonEntry,
  files: Files,
  options: CheckOptions,
): JudgeRun {
  const workspace = options.judgeWorkspaceDir
    ? ["--workspace", options.judgeWorkspaceDir]
    : [];
  const result = spawnSync(
    "cargo",
    [
      "run",
      "--manifest-path",
      join(ROOT, "src-tauri/Cargo.toml"),
      "--locked",
      "--quiet",
      "--features",
      JUDGE_FEATURE,
      "--bin",
      JUDGE_BIN,
      "--",
      "--lesson",
      entry.dir,
      "--files",
      files,
      ...workspace,
    ],
    { cwd: ROOT, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  );
  if (result.error) {
    return {
      ok: false,
      message: `cargo を起動できない: ${result.error.message}`,
    };
  }
  if (result.status !== 0) {
    const stderr = result.stderr.trim();
    return {
      ok: false,
      message: stderr || `終了コード ${result.status ?? result.signal}`,
    };
  }
  try {
    const judgement = JSON.parse(result.stdout) as Judgement;
    if (typeof judgement?.status !== "string") {
      return { ok: false, message: "判定結果に status が無い" };
    }
    return { ok: true, judgement };
  } catch (e) {
    return {
      ok: false,
      message: `判定結果を JSON として読めない: ${(e as Error).message}`,
    };
  }
}

// 判定結果の要約（status、通過数、失敗したテストケース、ビルドの出力）。
function summary(judgement: Judgement): string {
  const parts = [
    `${judgement.status}、${judgement.passed}/${judgement.total} 件通過`,
  ];
  const failed = judgement.cases.filter((c) => c.status !== "pass");
  if (failed.length > 0) {
    parts.push(
      `失敗したテストケース: ${failed.map((c) => `${c.name}（${c.status}）`).join("、")}`,
    );
    const detail = failed[0].detail;
    if (detail !== null && detail !== undefined) {
      parts.push(
        `最初の失敗の詳細: ${typeof detail === "string" ? detail : JSON.stringify(detail)}`,
      );
    }
  }
  if (judgement.status === "compile_error") {
    const lines = judgement.build_output.trimEnd().split("\n");
    const shown = lines.slice(0, BUILD_OUTPUT_LINES).join("\n");
    const rest =
      lines.length > BUILD_OUTPUT_LINES
        ? `\n（ほか ${lines.length - BUILD_OUTPUT_LINES} 行）`
        : "";
    parts.push(`ビルドの出力:\n${shown}${rest}`);
  }
  return parts.join("。");
}

export function checkJudge(options: CheckOptions): Finding[] {
  const findings: Finding[] = [];
  for (const entry of readLessonEntries(options.contentDir)) {
    const dir = exerciseDir(entry);
    if (dir === null) continue;
    for (const files of ["solution", "starter"] as const) {
      if (!isDirectory(join(dir, files))) continue;
      const check = files === "solution" ? SOLUTION_CHECK : STARTER_CHECK;
      const error = (message: string) =>
        findings.push({
          severity: "error",
          check,
          file: `${entry.rel}/exercise/${files}`,
          lesson: entry.id,
          message,
        });
      const run = runJudge(entry, files, options);
      if (!run.ok) {
        error(`判定できなかった: ${run.message}`);
        continue;
      }
      const { judgement } = run;
      if (files === "solution" && judgement.status !== "passed") {
        error(
          `模範解答が判定ですべてのテストケースに通らない（${summary(judgement)}）`,
        );
      }
      if (files === "starter" && judgement.status === "passed") {
        error(`初期コードが判定で正解になる（${summary(judgement)}）`);
      }
    }
  }
  return findings;
}
