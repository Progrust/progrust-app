import { describe, expect, it } from "vitest";
import type { Chapter, LessonDefinition } from "../commands/content";
import { lessonInfo } from "./lesson";

function chapter(id: string, title: string, lessonIds: string[]): Chapter {
  return {
    id,
    title,
    description: "",
    language: "rust",
    number: 1,
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

const chapters = [
  chapter("ch01", "はじめに", ["ch01-01", "ch01-review"]),
  chapter("ch02", "変数", ["ch02-01", "ch02-02", "ch02-03", "ch02-review"]),
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
  it("章の番号とレッスンの番号を、並びの位置から決める", () => {
    expect(lessonInfo(chapters, lesson("ch02-03", "ch02"))).toEqual({
      chapterNumber: 2,
      chapterTitle: "変数",
      lessonNumber: "2-3",
      lessonTitle: "ch02-03 のレッスン名",
    });
  });

  it("章末復習レッスンにも、同じ決め方で番号を付ける", () => {
    expect(
      lessonInfo(chapters, lesson("ch02-review", "ch02")).lessonNumber,
    ).toBe("2-4");
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
