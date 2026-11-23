// 検査「既習要素」「要素マスタ」「章末復習の主要素」
// （documents/spec/content-guidelines/check-tool.md）。
// 導入レッスンは、章とレッスンの一覧の「主要素」から求める（準備中のレッスンの主要素も一覧で決まっている）。
// lesson.json を読めないことや形の誤りは、「構成」「定義」が報告する。
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { readChapterList, type ListedChapter } from "./curriculum-list.ts";
import {
  isPrepared,
  readLessonEntries,
  readLessonJson,
  type LessonEntry,
} from "./lesson-files.ts";
import type { Check, CheckOptions, Finding } from "./types.ts";

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((v) => typeof v === "string");
}

// 一覧を読めないことは「カリキュラム」が報告するので、ここでは検査しない。
function tryReadList(curriculumDir: string): ListedChapter[] | undefined {
  try {
    return readChapterList(curriculumDir);
  } catch {
    return undefined;
  }
}

// 検査できる（準備中でなく、読める）レッスンの lesson.json。
function readLessons(
  contentDir: string,
): { entry: LessonEntry; value: Record<string, unknown> }[] {
  return readLessonEntries(contentDir).flatMap((entry) => {
    if (isPrepared(entry)) return [];
    const result = readLessonJson(entry);
    return result.state === "ok" ? [{ entry, value: result.value }] : [];
  });
}

// 既習要素: secondary_elements の各要素の導入レッスンが、学習順でそのレッスンより前にある。
export function checkLearnedElements({
  contentDir,
  curriculumDir,
}: CheckOptions): Finding[] {
  const list = tryReadList(curriculumDir);
  if (!list) return [];
  const order = list.flatMap((c) => c.rows.map((r) => r.id));
  // 要素 → 学習順で最初にその要素を主要素とするレッスンの位置。
  const introduced = new Map<string, number>();
  list
    .flatMap((c) => c.rows)
    .forEach((row, index) => {
      for (const id of row.primaryElements) {
        if (!introduced.has(id)) introduced.set(id, index);
      }
    });

  const out: Finding[] = [];
  for (const { entry, value } of readLessons(contentDir)) {
    const position = order.indexOf(entry.id);
    const secondary = value.secondary_elements;
    if (position < 0 || !isStringArray(secondary)) continue;
    for (const id of secondary) {
      const at = introduced.get(id);
      if (at !== undefined && at < position) continue;
      out.push({
        severity: "error",
        check: "既習要素",
        file: `${entry.rel}/lesson.json`,
        lesson: entry.id,
        message:
          at === undefined
            ? `secondary_elements の要素 ${id} が、一覧のどのレッスンの「主要素」にもなく、既習でない`
            : `secondary_elements の要素 ${id} が既習でない（導入レッスンは ${order[at]} で、学習順でこのレッスンより前にない）`,
      });
    }
  }
  return out;
}

// 要素マスタ: deprecated でない要素が、いずれかのレッスンの primary_elements に含まれている。
// 準備中のレッスンが 1 つでもある間は検査しない。
export function checkMasterElementsUsed({
  contentDir,
}: CheckOptions): Finding[] {
  const entries = readLessonEntries(contentDir);
  if (entries.length === 0 || entries.some(isPrepared)) return [];

  let parsed: unknown;
  try {
    parsed = JSON.parse(
      readFileSync(join(contentDir, "elements.json"), "utf8"),
    );
  } catch {
    // 読めないことは「要素と一覧」が報告する。
    return [];
  }
  if (!Array.isArray(parsed)) return [];

  const used = new Set<string>();
  for (const { value } of readLessons(contentDir)) {
    if (isStringArray(value.primary_elements)) {
      for (const id of value.primary_elements) used.add(id);
    }
  }
  const out: Finding[] = [];
  for (const el of parsed as ({
    id?: unknown;
    deprecated?: unknown;
  } | null)[]) {
    if (typeof el?.id !== "string" || el.deprecated === true) continue;
    if (used.has(el.id)) continue;
    out.push({
      severity: "error",
      check: "要素マスタ",
      file: "elements.json",
      message: `要素 ${el.id} が、どのレッスンの primary_elements にも含まれていない`,
    });
  }
  return out;
}

// 章末復習の主要素: 章末復習レッスンの primary_elements の各要素が、
// 一覧で、その章のレッスンの「主要素」になっている。
export function checkReviewPrimaryElements({
  contentDir,
  curriculumDir,
}: CheckOptions): Finding[] {
  const list = tryReadList(curriculumDir);
  if (!list) return [];
  const out: Finding[] = [];
  for (const { entry, value } of readLessons(contentDir)) {
    if (value.kind !== "chapter_review") continue;
    if (!isStringArray(value.primary_elements)) continue;
    const chapter = list.find((c) => c.id === entry.chapter);
    const own = new Set(
      (chapter?.rows ?? [])
        .filter((r) => r.id !== entry.id)
        .flatMap((r) => r.primaryElements),
    );
    for (const id of value.primary_elements) {
      if (own.has(id)) continue;
      out.push({
        severity: "error",
        check: "章末復習の主要素",
        file: `${entry.rel}/lesson.json`,
        lesson: entry.id,
        message: `primary_elements の要素 ${id} が、一覧で章 ${entry.chapter} のレッスンの「主要素」になっていない`,
      });
    }
  }
  return out;
}

export const learnedElementsCheck: Check = {
  name: "既習要素",
  run: checkLearnedElements,
};

export const masterElementsUsedCheck: Check = {
  name: "要素マスタ",
  run: checkMasterElementsUsed,
};

export const reviewPrimaryElementsCheck: Check = {
  name: "章末復習の主要素",
  run: checkReviewPrimaryElements,
};
