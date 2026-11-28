// 検査「構成」「定義」が共通で使う、chapters.json のレッスンと lesson.json の読み込み。
import { existsSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

export interface LessonEntry {
  id: string;
  // chapters.json でそのレッスンを載せている章の ID。
  chapter: string;
  // レッスンのディレクトリ（content/lessons/{lesson_id}/）。
  dir: string;
  // 教材のディレクトリからの相対パス（lessons/{lesson_id}/）。
  rel: string;
}

// chapters.json の全レッスン。読めないときや形が違うときは空（「カリキュラム」が報告する）。
export function readLessonEntries(contentDir: string): LessonEntry[] {
  let parsed: unknown;
  try {
    parsed = JSON.parse(
      readFileSync(join(contentDir, "chapters.json"), "utf8"),
    );
  } catch {
    return [];
  }
  if (!Array.isArray(parsed)) return [];
  const out: LessonEntry[] = [];
  for (const chapter of parsed as { id?: unknown; lessons?: unknown }[]) {
    if (typeof chapter !== "object" || chapter === null) continue;
    if (typeof chapter.id !== "string" || !Array.isArray(chapter.lessons)) {
      continue;
    }
    for (const id of chapter.lessons) {
      if (typeof id !== "string") continue;
      out.push({
        id,
        chapter: chapter.id,
        dir: join(contentDir, "lessons", id),
        rel: `lessons/${id}`,
      });
    }
  }
  return out;
}

// C の章のレッスン（章 ID が c{NN} の形）。chapters.json の language と章 ID の一致は「カリキュラム」が検査する。
export function isCLesson(entry: LessonEntry): boolean {
  return /^c\d+$/.test(entry.chapter);
}

// レッスンのディレクトリが無いレッスンは、準備中のレッスン。
export function isPrepared(entry: LessonEntry): boolean {
  return !existsSync(entry.dir);
}

export function isFile(path: string): boolean {
  return existsSync(path) && statSync(path).isFile();
}

export function isDirectory(path: string): boolean {
  return existsSync(path) && statSync(path).isDirectory();
}

export type LessonJsonResult =
  | { state: "ok"; value: Record<string, unknown> }
  | { state: "missing" }
  | { state: "invalid"; message: string };

export function readLessonJson(entry: LessonEntry): LessonJsonResult {
  const path = join(entry.dir, "lesson.json");
  if (!isFile(path)) return { state: "missing" };
  let parsed: unknown;
  try {
    parsed = JSON.parse(readFileSync(path, "utf8"));
  } catch (e) {
    return { state: "invalid", message: (e as Error).message };
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    return { state: "invalid", message: "オブジェクトでない" };
  }
  return { state: "ok", value: parsed as Record<string, unknown> };
}
