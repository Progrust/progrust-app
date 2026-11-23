// 検査「要素と一覧」（documents/spec/content-guidelines/check-tool.md）。
// lesson.json を使わないので、準備中のレッスンがあっても検査する。
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { readChapterList, type ListedChapter } from "./curriculum-list.ts";
import type { Check, CheckOptions, Finding } from "./types.ts";

const NAME = "要素と一覧";
const FILE = "elements.json";
const ELEMENT_ID = /^[a-z][a-z0-9_]*\.[a-z][a-z0-9_]*$/;

interface Element {
  id?: unknown;
  chapter?: unknown;
  deprecated?: unknown;
}

export function checkElementsAndList({
  contentDir,
  curriculumDir,
}: CheckOptions): Finding[] {
  const out: Finding[] = [];
  const error = (message: string, file = FILE) =>
    out.push({ severity: "error", check: NAME, file, message });

  let listed: ListedChapter[];
  try {
    listed = readChapterList(curriculumDir);
  } catch (e) {
    error(
      `章とレッスンの一覧を読めない: ${(e as Error).message}`,
      curriculumDir,
    );
    return out;
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
  const elements = parsed as Element[];

  const seen = new Set<unknown>();
  for (const el of elements) {
    if (typeof el.id !== "string" || !ELEMENT_ID.test(el.id)) {
      error(
        `要素 ID ${JSON.stringify(el.id)} が「{分類}.{要素}」の snake_case の形でない`,
      );
    }
    if (seen.has(el.id)) error(`要素 ID ${String(el.id)} が重複している`);
    seen.add(el.id);
  }
  const byId = new Map<unknown, Element>();
  for (const el of elements) if (!byId.has(el.id)) byId.set(el.id, el);

  // 学習順で最初にその要素を主要素とするレッスンの章。
  const introChapter = new Map<string, string>();
  for (const chapter of listed) {
    for (const row of chapter.rows) {
      for (const id of row.primaryElements) {
        if (!introChapter.has(id)) introChapter.set(id, chapter.id);
        const el = byId.get(id);
        const lessonError = (message: string) =>
          out.push({
            severity: "error",
            check: NAME,
            file: chapter.file,
            lesson: row.id,
            message,
          });
        if (!el) lessonError(`主要素 ${id} が要素マスタにない`);
        else if (el.deprecated === true) {
          lessonError(`主要素 ${id} が要素マスタで deprecated になっている`);
        }
      }
    }
  }

  for (const el of elements) {
    if (typeof el.id !== "string" || el.deprecated === true) continue;
    const chapter = introChapter.get(el.id);
    if (chapter === undefined) {
      error(`要素 ${el.id} が、一覧のどのレッスンの「主要素」にもない`);
    } else if (el.chapter !== chapter) {
      error(
        `要素 ${el.id} の chapter が一覧と違う（一覧: ${chapter}、elements.json: ${JSON.stringify(el.chapter)}）`,
      );
    }
  }
  return out;
}

export const elementsCheck: Check = {
  name: NAME,
  run: checkElementsAndList,
};
