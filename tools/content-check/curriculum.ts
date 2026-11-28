// 検査「カリキュラム」のうち、chapters.json と章とレッスンの一覧の照合。
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import {
  languageOfChapterId,
  readChapterList,
  type ListedChapter,
} from "./curriculum-list.ts";
import type { Check, CheckOptions, Finding } from "./types.ts";

const NAME = "カリキュラム";
const FILE = "chapters.json";

interface JsonChapter {
  id?: unknown;
  title?: unknown;
  description?: unknown;
  language?: unknown;
  lessons?: unknown;
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((v) => typeof v === "string");
}

function sameItemsDifferentOrder(have: unknown[], want: string[]): boolean {
  return (
    have.length === want.length &&
    want.every((id) => have.includes(id)) &&
    have.some((id, i) => id !== want[i])
  );
}

function compareChapter(
  actual: JsonChapter,
  listed: ListedChapter,
  out: Finding[],
): void {
  const error = (message: string, lesson?: string) =>
    out.push({ severity: "error", check: NAME, file: FILE, lesson, message });
  const where = `章 ${listed.id}`;
  if (actual.title !== listed.title) {
    error(
      `${where} の title が一覧と違う（一覧: ${JSON.stringify(listed.title)}、chapters.json: ${JSON.stringify(actual.title)}）`,
    );
  }
  if (actual.description !== listed.description) {
    error(
      `${where} の description が一覧と違う（一覧: ${JSON.stringify(listed.description)}、chapters.json: ${JSON.stringify(actual.description)}）`,
    );
  }
  if (!isStringArray(actual.lessons)) {
    error(`${where} の lessons が文字列の配列でない`);
    return;
  }
  const have = actual.lessons;
  for (const id of listed.lessons) {
    if (!have.includes(id)) {
      error(`${where} に、一覧にあるレッスンがない`, id);
    }
  }
  for (const id of have) {
    if (!listed.lessons.includes(id)) {
      error(`${where} に、一覧にないレッスンがある`, id);
    }
  }
  if (sameItemsDifferentOrder(have, listed.lessons)) {
    error(
      `${where} のレッスンの並びが一覧と違う（一覧: ${listed.lessons.join("、")}、chapters.json: ${have.join("、")}）`,
    );
  }
}

const LANGUAGE_NAMES = { c: "C", rust: "Rust" } as const;

// 章の language（省略は rust）が章 ID の形から決まる言語と一致すること、
// C の章がすべての Rust の章より前にあること。
function checkLanguages(
  chapters: JsonChapter[],
  error: (message: string) => void,
): void {
  let firstRust: string | undefined;
  for (const chapter of chapters) {
    if (typeof chapter.id !== "string") continue;
    const fromId = languageOfChapterId(chapter.id);
    const declared = chapter.language ?? "rust";
    if (declared !== fromId) {
      error(
        `章 ${chapter.id} の language が章 ID の形と合わない（章 ID から決まる言語: ${LANGUAGE_NAMES[fromId]}、chapters.json: ${JSON.stringify(chapter.language ?? null)}）`,
      );
    }
    if (fromId === "rust") {
      firstRust ??= chapter.id;
    } else if (firstRust !== undefined) {
      error(`C の章 ${chapter.id} が Rust の章 ${firstRust} より後にある`);
    }
  }
}

export function checkChaptersJson({
  contentDir,
  curriculumDir,
}: CheckOptions): Finding[] {
  const out: Finding[] = [];
  const error = (message: string, lesson?: string) =>
    out.push({ severity: "error", check: NAME, file: FILE, lesson, message });

  let listed: ListedChapter[];
  try {
    listed = readChapterList(curriculumDir);
  } catch (e) {
    return [
      {
        severity: "error",
        check: NAME,
        file: curriculumDir,
        message: `章とレッスンの一覧を読めない: ${(e as Error).message}`,
      },
    ];
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(readFileSync(join(contentDir, FILE), "utf8"));
  } catch (e) {
    error(`${FILE} を読めない: ${(e as Error).message}`);
    return out;
  }
  if (!Array.isArray(parsed)) {
    error(`${FILE} が配列でない`);
    return out;
  }
  const chapters = parsed as JsonChapter[];

  const ids = chapters.map((c) => c.id);
  const listedIds = listed.map((c) => c.id);
  for (const id of listedIds) {
    if (!ids.includes(id)) error(`一覧にある章 ${id} がない`);
  }
  for (const id of ids) {
    if (typeof id !== "string" || !listedIds.includes(id)) {
      error(`一覧にない章 ${String(id)} がある`);
    }
  }
  if (sameItemsDifferentOrder(ids, listedIds)) {
    error(
      `章の並びが一覧と違う（一覧: ${listedIds.join("、")}、chapters.json: ${ids.join("、")}）`,
    );
  }

  for (const chapter of listed) {
    const actual = chapters.find((c) => c.id === chapter.id);
    if (actual) compareChapter(actual, chapter, out);
  }
  checkLanguages(chapters, error);
  return out;
}

function sameList(a: unknown, b: string[]): boolean {
  return (
    isStringArray(a) && a.length === b.length && a.every((v, i) => v === b[i])
  );
}

function show(value: unknown): string {
  return Array.isArray(value)
    ? value.join("、") || "（なし）"
    : JSON.stringify(value);
}

// 章末復習レッスンと準備中のレッスン（ディレクトリが無いレッスン）以外の
// lesson.json の title・kind・primary_elements を、一覧と照合する。
// primary_elements の照合は Rust のレッスンだけ（C のレッスンは主要素を持たない）。
export function checkLessonJsons({
  contentDir,
  curriculumDir,
}: CheckOptions): Finding[] {
  const out: Finding[] = [];
  let listed: ListedChapter[];
  try {
    listed = readChapterList(curriculumDir);
  } catch {
    // 一覧を読めないことは checkChaptersJson が報告する。
    return out;
  }
  for (const chapter of listed) {
    for (const row of chapter.rows) {
      if (row.id.endsWith("-review")) continue;
      const dir = join(contentDir, "lessons", row.id);
      if (!existsSync(dir)) continue;
      const file = `lessons/${row.id}/lesson.json`;
      const error = (message: string) =>
        out.push({
          severity: "error",
          check: NAME,
          file,
          lesson: row.id,
          message,
        });
      let lesson: {
        title?: unknown;
        kind?: unknown;
        primary_elements?: unknown;
      };
      try {
        lesson = JSON.parse(readFileSync(join(dir, "lesson.json"), "utf8"));
      } catch (e) {
        // ファイルが無いことは「構成」の検査の対象。
        if ((e as NodeJS.ErrnoException).code === "ENOENT") continue;
        error(`lesson.json を読めない: ${(e as Error).message}`);
        continue;
      }
      if (typeof lesson !== "object" || lesson === null) {
        error("lesson.json がオブジェクトでない");
        continue;
      }
      if (lesson.title !== row.title) {
        error(
          `title が一覧と違う（一覧: ${JSON.stringify(row.title)}、lesson.json: ${JSON.stringify(lesson.title)}）`,
        );
      }
      const isVideoOnly = lesson.kind === "video_only";
      if (isVideoOnly !== row.videoOnly) {
        error(
          row.videoOnly
            ? `一覧では動画のみのレッスンだが、kind が video_only でない（${JSON.stringify(lesson.kind)}）`
            : "kind が video_only だが、一覧では動画のみのレッスンでない",
        );
      }
      if (
        chapter.language === "rust" &&
        lesson.kind === "lesson" &&
        !sameList(lesson.primary_elements, row.primaryElements)
      ) {
        error(
          `primary_elements が一覧の「主要素」と違う（一覧: ${show(row.primaryElements)}、lesson.json: ${show(lesson.primary_elements)}）`,
        );
      }
    }
  }
  return out;
}

export const curriculumCheck: Check = {
  name: NAME,
  run: (options) => [
    ...checkChaptersJson(options),
    ...checkLessonJsons(options),
  ],
};
