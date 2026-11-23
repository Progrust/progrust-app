// 検査「台本」「音声」と、警告が共通で使う、動画のファイルの読み込み。
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import {
  isFile,
  isPrepared,
  readLessonEntries,
  readLessonJson,
  type LessonEntry,
} from "./lesson-files.ts";

export type JsonResult =
  | { state: "ok"; value: unknown }
  | { state: "missing" }
  | { state: "invalid"; message: string };

export function readJsonFile(path: string): JsonResult {
  if (!isFile(path)) return { state: "missing" };
  try {
    return { state: "ok", value: JSON.parse(readFileSync(path, "utf8")) };
  } catch (e) {
    return { state: "invalid", message: (e as Error).message };
  }
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

// 動画のあるレッスン（準備中でなく、章末復習でないレッスン）。
export function readVideoLessons(contentDir: string): LessonEntry[] {
  return readLessonEntries(contentDir)
    .filter((entry) => !isPrepared(entry))
    .filter((entry) => {
      const lesson = readLessonJson(entry);
      const kind =
        lesson.state === "ok" && typeof lesson.value.kind === "string"
          ? lesson.value.kind
          : undefined;
      return kind === undefined
        ? !entry.id.endsWith("-review")
        : kind !== "chapter_review";
    });
}

// 動画のファイルの、教材のディレクトリからの相対パス。
export function videoRel(entry: LessonEntry, file: string): string {
  return `${entry.rel}/video/${file}`;
}

export function videoPath(entry: LessonEntry, file: string): string {
  return join(entry.dir, "video", file);
}

export function exists(path: string): boolean {
  return existsSync(path);
}

// 文字数は Unicode のコードポイントで数える。
export function codePointLength(text: string): number {
  return [...text].length;
}

// 台本の文のうち、検査に使うもの。形の検査は「台本」が行うので、形が違うものは除く。
export interface ScriptSentence {
  id: string;
  text: string;
  say: string;
}

export function readSentences(script: unknown): ScriptSentence[] {
  if (!isRecord(script) || !Array.isArray(script.sentences)) return [];
  return script.sentences.flatMap((s: unknown) =>
    isRecord(s) &&
    typeof s.id === "string" &&
    typeof s.text === "string" &&
    typeof s.say === "string"
      ? [{ id: s.id, text: s.text, say: s.say }]
      : [],
  );
}
