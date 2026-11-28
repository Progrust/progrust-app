// 教材チェックツールの描画が、content/chapters.json から useLesson() の材料を作るときの番号
// （documents/spec/content-format/chapters-json.md「章とレッスンの番号」）のテスト。
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, expect, it } from "vitest";
import { lessonInfo } from "../../src/video/lesson.ts";
import { readChapters } from "./render.ts";

const dirs: string[] = [];

afterEach(() => {
  for (const dir of dirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

function contentWith(chapters: unknown[]): string {
  const dir = mkdtempSync(join(tmpdir(), "progrust-render-chapters-"));
  dirs.push(dir);
  writeFileSync(join(dir, "chapters.json"), JSON.stringify(chapters));
  return dir;
}

function definition(id: string, chapter: string) {
  return {
    id,
    chapter,
    kind: "lesson" as const,
    title: id,
    primary_elements: [],
    secondary_elements: [],
  };
}

const cChapters = [
  {
    id: "c01",
    title: "C の基本",
    description: "",
    language: "c",
    lessons: ["c01-01", "c01-02", "c01-03", "c01-04", "c01-review"],
  },
  {
    id: "c02",
    title: "ポインタとメモリ",
    description: "",
    language: "c",
    lessons: ["c02-01", "c02-review"],
  },
];
// language を省いた章は Rust の章。
const rustChapters = [
  {
    id: "ch01",
    title: "はじめに",
    description: "",
    lessons: ["ch01-01", "ch01-review"],
  },
  {
    id: "ch04",
    title: "所有権",
    description: "",
    language: "rust",
    lessons: ["ch04-01", "ch04-02", "ch04-03", "ch04-04", "ch04-review"],
  },
];

it("章の番号は、同じ言語の章の中での位置になる", () => {
  const chapters = readChapters(contentWith([...cChapters, ...rustChapters]));
  expect(chapters.map((c) => [c.id, c.language, c.number])).toEqual([
    ["c01", "c", 1],
    ["c02", "c", 2],
    ["ch01", "rust", 1],
    ["ch04", "rust", 2],
  ]);
  expect(lessonInfo(chapters, definition("c01-04", "c01"))).toMatchObject({
    chapterLabel: "C 第1章",
    lessonNumber: "C1-4",
  });
});

it("C の章が無いときも、Rust の章の番号は変わらない", () => {
  const withC = readChapters(contentWith([...cChapters, ...rustChapters]));
  const rustOnly = readChapters(contentWith(rustChapters));
  expect(rustOnly.map((c) => [c.id, c.number])).toEqual(
    withC.filter((c) => c.language === "rust").map((c) => [c.id, c.number]),
  );
  expect(lessonInfo(rustOnly, definition("ch04-04", "ch04"))).toEqual(
    lessonInfo(withC, definition("ch04-04", "ch04")),
  );
});
