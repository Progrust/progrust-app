// 検査「定義」（documents/spec/content-guidelines/check-tool.md）。
// lesson.json が documents/spec/content-format/lesson-json.md の形式に合っていることと、
// 要素 ID が要素マスタにあることを確かめる。準備中のレッスンは検査しない。
// 一覧との照合は「カリキュラム」、既習・章末復習の主要素は別の項目が行う。
import { readFileSync } from "node:fs";
import { join } from "node:path";
import {
  isCLesson,
  isPrepared,
  readLessonEntries,
  readLessonJson,
  type LessonEntry,
} from "./lesson-files.ts";
import type { Check, CheckOptions, Finding } from "./types.ts";

const NAME = "定義";
const KINDS = ["lesson", "video_only", "chapter_review", "generated"];

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((v) => typeof v === "string");
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim() !== "";
}

// 主要素の件数の範囲。
function primaryRange(kind: string): [number, number] {
  if (kind === "video_only") return [0, 0];
  if (kind === "chapter_review") return [2, 5];
  return [1, 2];
}

// 要素マスタの要素 ID。読めないときは undefined（「要素と一覧」が報告する）。
function readElementIds(contentDir: string): Set<string> | undefined {
  try {
    const parsed: unknown = JSON.parse(
      readFileSync(join(contentDir, "elements.json"), "utf8"),
    );
    if (!Array.isArray(parsed)) return undefined;
    return new Set(
      parsed.flatMap((e: { id?: unknown } | null) =>
        typeof e?.id === "string" ? [e.id] : [],
      ),
    );
  } catch {
    return undefined;
  }
}

// C のレッスンの exercise（lesson-json.md「C のレッスン」）。
function checkCExercise(
  exercise: unknown,
  error: (message: string) => void,
): void {
  if (!isRecord(exercise)) {
    error("exercise がオブジェクトでない");
    return;
  }
  if (exercise.layout !== "single") {
    error(
      `C のレッスンの exercise.layout は single だけ（${JSON.stringify(exercise.layout)}）`,
    );
  }
  const judge = exercise.judge;
  if (!isRecord(judge)) {
    error("exercise.judge がオブジェクトでない");
  } else {
    if (judge.mode !== "stdout") {
      error(
        `C のレッスンの exercise.judge.mode は stdout だけ（${JSON.stringify(judge.mode)}）`,
      );
    }
    if (judge.target !== undefined && judge.target !== "main.c") {
      error(
        `C のレッスンの exercise.judge.target は main.c（${JSON.stringify(judge.target)}）`,
      );
    }
    if (judge.min_learner_tests !== undefined) {
      error("C のレッスンでは exercise.judge.min_learner_tests を書けない");
    }
  }
  for (const key of ["dependencies", "readonly_files"]) {
    const value = exercise[key];
    if (value !== undefined && !(Array.isArray(value) && value.length === 0)) {
      error(`C のレッスンの exercise.${key} は空の配列にする`);
    }
  }
  const labels = exercise.case_labels;
  if (
    labels !== undefined &&
    !(isRecord(labels) && Object.keys(labels).length === 0)
  ) {
    error("C のレッスンの exercise.case_labels は空のオブジェクトにする");
  }
}

function checkExercise(
  exercise: unknown,
  error: (message: string) => void,
): void {
  if (!isRecord(exercise)) {
    error("exercise がオブジェクトでない");
    return;
  }
  const layout = exercise.layout;
  if (layout !== "single" && layout !== "multi") {
    error(
      `exercise.layout が single か multi でない（${JSON.stringify(layout)}）`,
    );
  }
  const judge = exercise.judge;
  if (!isRecord(judge)) {
    error("exercise.judge がオブジェクトでない");
  } else {
    const { mode, target, min_learner_tests: minTests } = judge;
    if (mode !== "stdout" && mode !== "test") {
      error(
        `exercise.judge.mode が stdout か test でない（${JSON.stringify(mode)}）`,
      );
    } else if (mode === "stdout") {
      if (target !== undefined && target !== "src/main.rs") {
        error("stdout 方式の exercise.judge.target は src/main.rs 固定");
      }
      if (minTests !== undefined) {
        error("stdout 方式では exercise.judge.min_learner_tests を書けない");
      }
    } else {
      if (target !== "src/main.rs" && target !== "src/lib.rs") {
        error(
          `test 方式の exercise.judge.target が src/main.rs か src/lib.rs でない（${JSON.stringify(target)}）`,
        );
      } else if (layout === "single" && target === "src/lib.rs") {
        error("layout が single のとき、exercise.judge.target は src/main.rs");
      }
      if (
        minTests !== undefined &&
        !(
          typeof minTests === "number" &&
          Number.isInteger(minTests) &&
          minTests >= 1
        )
      ) {
        error("exercise.judge.min_learner_tests が 1 以上の整数でない");
      }
    }
  }
  if (
    exercise.dependencies !== undefined &&
    !isStringArray(exercise.dependencies)
  ) {
    error("exercise.dependencies が文字列の配列でない");
  }
  const readonlyFiles = exercise.readonly_files;
  if (readonlyFiles !== undefined) {
    if (!isStringArray(readonlyFiles)) {
      error("exercise.readonly_files が文字列の配列でない");
    } else if (readonlyFiles.length > 0 && layout !== "multi") {
      error("exercise.readonly_files は layout が multi のときだけ書ける");
    }
  }
  const labels = exercise.case_labels;
  if (labels !== undefined) {
    if (
      !isRecord(labels) ||
      !Object.values(labels).every((v) => typeof v === "string")
    ) {
      error("exercise.case_labels が、文字列から文字列への対応でない");
    } else if (
      isRecord(judge) &&
      judge.mode === "stdout" &&
      Object.keys(labels).length > 0
    ) {
      error("stdout 方式では exercise.case_labels を書けない");
    }
  }
}

// Rust のレッスンと生成問題の要素（要素マスタにあること、主要素の件数、video_only の副要素）。
function checkElements(
  lesson: Record<string, unknown>,
  knownKind: string | undefined,
  lists: [string, unknown][],
  elementIds: Set<string> | undefined,
  error: (message: string) => void,
): void {
  for (const [key, value] of lists) {
    if (!isStringArray(value)) {
      error(`${key} が文字列の配列でない`);
      continue;
    }
    if (elementIds) {
      for (const id of value) {
        if (!elementIds.has(id))
          error(`${key} の要素 ${id} が要素マスタにない`);
      }
    }
  }
  if (isStringArray(lesson.primary_elements) && knownKind !== undefined) {
    const [min, max] = primaryRange(knownKind);
    const n = lesson.primary_elements.length;
    if (n < min || n > max) {
      error(
        `primary_elements の件数が ${knownKind} の範囲（${min === max ? `${min} 件` : `${min}〜${max} 件`}）にない（${n} 件）`,
      );
    }
  }
  if (
    knownKind === "video_only" &&
    isStringArray(lesson.secondary_elements) &&
    lesson.secondary_elements.length > 0
  ) {
    error("video_only の secondary_elements は空の配列にする");
  }
}

function checkLesson(
  entry: LessonEntry,
  elementIds: Set<string> | undefined,
): Finding[] {
  const out: Finding[] = [];
  const file = `${entry.rel}/lesson.json`;
  const error = (message: string) =>
    out.push({
      severity: "error",
      check: NAME,
      file,
      lesson: entry.id,
      message,
    });

  const result = readLessonJson(entry);
  // ファイルが無いことは「構成」が報告する。
  if (result.state === "missing") return out;
  if (result.state === "invalid") {
    error(`lesson.json を読めない: ${result.message}`);
    return out;
  }
  const lesson = result.value;

  if (lesson.id !== entry.id) {
    error(
      `id がレッスンの ID と違う（${JSON.stringify(lesson.id)}、期待: ${entry.id}）`,
    );
  }
  if (lesson.chapter !== entry.chapter) {
    error(
      `chapter が chapters.json と違う（${JSON.stringify(lesson.chapter)}、期待: ${entry.chapter}）`,
    );
  }
  for (const key of ["title", "summary"]) {
    if (!isNonEmptyString(lesson[key])) error(`${key} が空でない文字列でない`);
  }
  const minutes = lesson.estimated_minutes;
  if (
    typeof minutes !== "number" ||
    !Number.isFinite(minutes) ||
    minutes <= 0
  ) {
    error("estimated_minutes が正の数でない");
  }

  const kind = lesson.kind;
  if (typeof kind !== "string" || !KINDS.includes(kind)) {
    error(
      `kind が ${KINDS.join("・")} のどれでもない（${JSON.stringify(kind)}）`,
    );
  }
  const knownKind =
    typeof kind === "string" && KINDS.includes(kind) ? kind : undefined;
  const isC = isCLesson(entry);
  if (isC && knownKind === "generated") {
    error("C のレッスンの kind に generated は使えない");
  }

  // tags
  const tags = lesson.tags;
  if (knownKind === "lesson" || knownKind === "video_only") {
    if (!isStringArray(tags) || tags.length === 0) {
      error("tags が 1 件以上の文字列の配列でない");
    } else {
      if (tags.some((t) => t === "")) error("tags に空の文字列がある");
      if (new Set(tags).size !== tags.length) error("tags に重複がある");
    }
  } else if (knownKind !== undefined && tags !== undefined) {
    error(`kind が ${knownKind} の lesson.json は tags を持たない`);
  }

  // 要素
  const lists: [string, unknown][] = [
    ["primary_elements", lesson.primary_elements],
    ["secondary_elements", lesson.secondary_elements],
  ];
  if (isC) {
    // C のレッスンは要素を持たない。
    for (const [key, value] of lists) {
      if (!isStringArray(value)) {
        error(`${key} が文字列の配列でない`);
      } else if (value.length > 0) {
        error(`C のレッスンの ${key} は空の配列にする`);
      }
    }
  } else {
    checkElements(lesson, knownKind, lists, elementIds, error);
  }

  // exercise
  if (knownKind === "video_only") {
    if (lesson.exercise !== undefined)
      error("video_only の lesson.json は exercise を持たない");
  } else if (knownKind !== undefined) {
    if (lesson.exercise === undefined)
      error(`kind が ${knownKind} の lesson.json に exercise がない`);
    else if (isC) checkCExercise(lesson.exercise, error);
    else checkExercise(lesson.exercise, error);
  }
  return out;
}

export function checkDefinitions({ contentDir }: CheckOptions): Finding[] {
  const elementIds = readElementIds(contentDir);
  return readLessonEntries(contentDir)
    .filter((entry) => !isPrepared(entry))
    .flatMap((entry) => checkLesson(entry, elementIds));
}

export const definitionCheck: Check = { name: NAME, run: checkDefinitions };
