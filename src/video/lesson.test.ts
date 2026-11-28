import { describe, expect, it } from "vitest";
import type { Chapter, Language, LessonDefinition } from "../commands/content";
import { lessonInfo } from "./lesson";

function chapter(
  id: string,
  title: string,
  lessonIds: string[],
  language: Language,
  number: number,
): Chapter {
  return {
    id,
    title,
    description: "",
    language,
    number,
    lessons: lessonIds.map((lessonId) => ({
      id: lessonId,
      kind: null,
      title: null,
      summary: null,
      estimated_minutes: null,
      in_preparation: true,
    })),
  };
}

const rustChapters = [
  chapter("ch01", "はじめに", ["ch01-01", "ch01-review"], "rust", 1),
  chapter(
    "ch02",
    "変数",
    ["ch02-01", "ch02-02", "ch02-03", "ch02-review"],
    "rust",
    2,
  ),
];
// C の章を前に置いた並び。
const chapters = [
  chapter(
    "c01",
    "C の基本",
    ["c01-01", "c01-02", "c01-03", "c01-04", "c01-review"],
    "c",
    1,
  ),
  ...rustChapters,
];

function lesson(id: string, chapterId: string): LessonDefinition {
  return {
    id,
    chapter: chapterId,
    kind: "lesson",
    title: `${id} のレッスン名`,
    primary_elements: [],
    secondary_elements: [],
  };
}

describe("lessonInfo", () => {
  it("Rust の章の番号とレッスンの番号を、同じ言語の章の中での番号から決める", () => {
    expect(lessonInfo(chapters, lesson("ch02-03", "ch02"))).toEqual({
      language: "rust",
      chapterNumber: 2,
      chapterLabel: "第2章",
      chapterTitle: "変数",
      lessonNumber: "2-3",
      lessonTitle: "ch02-03 のレッスン名",
    });
  });

  it("C の章は「C 第1章」「C1-4」の形になる", () => {
    expect(lessonInfo(chapters, lesson("c01-04", "c01"))).toEqual({
      language: "c",
      chapterNumber: 1,
      chapterLabel: "C 第1章",
      chapterTitle: "C の基本",
      lessonNumber: "C1-4",
      lessonTitle: "c01-04 のレッスン名",
    });
  });

  it("C の章が無いときの Rust の動画の番号は、C の章があるときと変わらない", () => {
    expect(lessonInfo(rustChapters, lesson("ch02-03", "ch02"))).toEqual(
      lessonInfo(chapters, lesson("ch02-03", "ch02")),
    );
  });

  it("章末復習レッスンにも、同じ決め方で番号を付ける", () => {
    expect(
      lessonInfo(chapters, lesson("ch02-review", "ch02")).lessonNumber,
    ).toBe("2-4");
    expect(lessonInfo(chapters, lesson("c01-review", "c01")).lessonNumber).toBe(
      "C1-5",
    );
  });

  it("章の並びにない章とレッスンは、エラーになる", () => {
    expect(() => lessonInfo(chapters, lesson("ch03-01", "ch03"))).toThrow(
      /ch03/,
    );
    expect(() => lessonInfo(chapters, lesson("ch02-09", "ch02"))).toThrow(
      /ch02-09/,
    );
  });
});
