// 検査「カリキュラム」のうち、chapters.json と章とレッスンの一覧の照合。
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { readChapterList, type ListedChapter } from "./curriculum-list.ts";
import type { Check, CheckOptions, Finding } from "./types.ts";

const NAME = "カリキュラム";
const FILE = "chapters.json";

interface JsonChapter {
  id?: unknown;
  title?: unknown;
  description?: unknown;
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
  return out;
}

export const curriculumCheck: Check = {
  name: NAME,
  run: checkChaptersJson,
};
