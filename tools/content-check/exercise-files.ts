// 検査「問題文・解説」「テストケース」「クレート」（documents/spec/content-guidelines/check-tool.md）。
// 動画のみのレッスンと準備中のレッスンは検査しない。ファイルが無いときは、「構成」が報告するので、ここでは何も言わない。
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  isFile,
  isPrepared,
  readLessonEntries,
  readLessonJson,
  type LessonEntry,
} from "./lesson-files.ts";
import type { Check, CheckOptions, Finding } from "./types.ts";

export const TEXT_CHECK = "問題文・解説";
export const CASES_CHECK = "テストケース";
export const CRATES_CHECK = "クレート";

const ROOT = fileURLToPath(new URL("../..", import.meta.url));
const DEFAULT_ALLOWED_CRATES = join(ROOT, "runner/allowed-crates.toml");

// ch01・ch02 のレッスン（章末復習レッスンを含む）は 1 件以上、ほかは 3 件以上（ADR 0146）。
const FEW_CASE_CHAPTERS = new Set(["ch01", "ch02"]);
const MIN_CASES = 3;
const MIN_CASES_EARLY = 1;

const PROBLEM_HEADINGS = ["問題", "出力", "例"];
const EXPLANATION_HEADINGS = [
  "考え方",
  "模範解答",
  "ポイント",
  "よくある間違い",
];

interface Exercise {
  dir: string;
  mode: string | undefined;
  dependencies: unknown;
  caseLabels: unknown;
}

// 検査の対象にするレッスンの演習。対象にしないときは null。
function exerciseOf(entry: LessonEntry): Exercise | null {
  if (isPrepared(entry)) return null;
  const lesson = readLessonJson(entry);
  if (lesson.state !== "ok") return null;
  if (lesson.value.kind === "video_only") return null;
  const exercise = lesson.value.exercise;
  if (typeof exercise !== "object" || exercise === null) return null;
  const e = exercise as {
    judge?: { mode?: unknown };
    dependencies?: unknown;
    case_labels?: unknown;
  };
  const mode =
    typeof e.judge === "object" && e.judge !== null
      ? (e.judge.mode as string | undefined)
      : undefined;
  return {
    dir: join(entry.dir, "exercise"),
    mode,
    dependencies: e.dependencies,
    caseLabels: e.case_labels,
  };
}

function finder(entry: LessonEntry, check: string, out: Finding[]) {
  return (file: string, message: string) =>
    out.push({
      severity: "error",
      check,
      file: `${entry.rel}/${file}`,
      lesson: entry.id,
      message,
    });
}

function readText(path: string): string | null {
  return isFile(path) ? readFileSync(path, "utf8") : null;
}

// `## ` の見出しの名前（コードブロックの中は除く）。
function headings(markdown: string): Set<string> {
  const found = new Set<string>();
  let fence: string | null = null;
  for (const line of markdown.split("\n")) {
    const f = /^\s*(`{3,}|~{3,})/.exec(line);
    if (f) {
      if (fence === null) fence = f[1][0];
      else if (f[1][0] === fence) fence = null;
      continue;
    }
    if (fence !== null) continue;
    const m = /^## +(.+?)\s*$/.exec(line);
    if (m) found.add(m[1]);
  }
  return found;
}

// stdout 方式の cases.json。読めないときは null（読めない理由は message に入れる）。
function readCases(
  path: string,
): { cases: { stdin?: unknown }[] } | { message: string } | null {
  const text = readText(path);
  if (text === null) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (e) {
    return {
      message: `cases.json を JSON として読めない: ${(e as Error).message}`,
    };
  }
  if (!Array.isArray(parsed)) {
    return { message: "cases.json が配列でない" };
  }
  return { cases: parsed as { stdin?: unknown }[] };
}

// judge_tests.rs の `#[test]` の関数名。
export function testFunctionNames(source: string): string[] {
  const names: string[] = [];
  const re =
    /#\[test\]\s*(?:(?:#\[[^\]]*\]|\/\/[^\n]*)\s*)*(?:pub\s+)?fn\s+(\w+)/g;
  for (let m = re.exec(source); m !== null; m = re.exec(source)) {
    names.push(m[1]);
  }
  return names;
}

function checkText(entry: LessonEntry, ex: Exercise): Finding[] {
  const out: Finding[] = [];
  const error = finder(entry, TEXT_CHECK, out);

  const problem = readText(join(ex.dir, "problem.md"));
  if (problem !== null) {
    const found = headings(problem);
    const required = [...PROBLEM_HEADINGS];
    // `## 入力` は、stdout 方式で、入力（stdin）があるときだけ必須。
    if (ex.mode === "stdout") {
      const cases = readCases(join(ex.dir, "cases.json"));
      if (
        cases !== null &&
        "cases" in cases &&
        cases.cases.some((c) => typeof c?.stdin === "string" && c.stdin !== "")
      ) {
        required.splice(1, 0, "入力");
      }
    }
    for (const h of required) {
      if (!found.has(h))
        error("exercise/problem.md", `problem.md に見出し「## ${h}」がない`);
    }
  }

  const explanation = readText(join(ex.dir, "explanation.md"));
  if (explanation !== null) {
    const found = headings(explanation);
    for (const h of EXPLANATION_HEADINGS) {
      if (!found.has(h)) {
        error(
          "exercise/explanation.md",
          `explanation.md に見出し「## ${h}」がない`,
        );
      }
    }
  }
  return out;
}

function checkCases(entry: LessonEntry, ex: Exercise): Finding[] {
  const out: Finding[] = [];
  const error = finder(entry, CASES_CHECK, out);
  const min = FEW_CASE_CHAPTERS.has(entry.chapter)
    ? MIN_CASES_EARLY
    : MIN_CASES;

  if (ex.mode === "stdout") {
    const cases = readCases(join(ex.dir, "cases.json"));
    if (cases === null) return out;
    if ("message" in cases) {
      error("exercise/cases.json", cases.message);
    } else if (cases.cases.length < min) {
      error(
        "exercise/cases.json",
        `テストケースが ${cases.cases.length} 件で、${min} 件以上ない`,
      );
    }
  } else if (ex.mode === "test") {
    const source = readText(join(ex.dir, "judge_tests.rs"));
    if (source === null) return out;
    const names = testFunctionNames(source);
    if (names.length < min) {
      error(
        "exercise/judge_tests.rs",
        `テストケースが ${names.length} 件で、${min} 件以上ない`,
      );
    }
    const labels = ex.caseLabels;
    if (
      typeof labels === "object" &&
      labels !== null &&
      !Array.isArray(labels)
    ) {
      for (const key of Object.keys(labels)) {
        if (!names.includes(key)) {
          error(
            "lesson.json",
            `case_labels のキー ${key} が、judge_tests.rs のテスト関数名にない`,
          );
        }
      }
    }
  }
  return out;
}

// runner/allowed-crates.toml の `[dependencies]` のクレート名。読めないときは null。
function readAllowedCrates(path: string): Set<string> | null {
  if (!existsSync(path)) return null;
  const names = new Set<string>();
  let inDependencies = false;
  for (const raw of readFileSync(path, "utf8").split("\n")) {
    const line = raw.replace(/#.*$/, "").trim();
    if (line === "") continue;
    const section = /^\[(.+)\]$/.exec(line);
    if (section) {
      inDependencies = section[1].trim() === "dependencies";
      continue;
    }
    const key = /^"?([A-Za-z0-9_-]+)"?\s*=/.exec(line);
    if (inDependencies && key) names.add(key[1]);
  }
  return names;
}

function checkCrates(
  entry: LessonEntry,
  ex: Exercise,
  allowed: Set<string> | null,
): Finding[] {
  const out: Finding[] = [];
  if (!Array.isArray(ex.dependencies)) return out;
  const error = finder(entry, CRATES_CHECK, out);
  for (const name of ex.dependencies) {
    if (typeof name !== "string") continue;
    if (allowed === null) {
      error(
        "lesson.json",
        `許可クレートの一覧を読めないので、${name} を確かめられない`,
      );
    } else if (!allowed.has(name)) {
      error(
        "lesson.json",
        `dependencies の ${name} が許可クレートの一覧にない`,
      );
    }
  }
  return out;
}

function forEachExercise(
  { contentDir }: CheckOptions,
  run: (entry: LessonEntry, ex: Exercise) => Finding[],
): Finding[] {
  return readLessonEntries(contentDir).flatMap((entry) => {
    const ex = exerciseOf(entry);
    return ex === null ? [] : run(entry, ex);
  });
}

export const textCheck: Check = {
  name: TEXT_CHECK,
  run: (options) => forEachExercise(options, checkText),
};

export const casesCheck: Check = {
  name: CASES_CHECK,
  run: (options) => forEachExercise(options, checkCases),
};

export const cratesCheck: Check = {
  name: CRATES_CHECK,
  run: (options) => {
    const allowed = readAllowedCrates(
      options.allowedCratesFile ?? DEFAULT_ALLOWED_CRATES,
    );
    return forEachExercise(options, (entry, ex) =>
      checkCrates(entry, ex, allowed),
    );
  },
};
