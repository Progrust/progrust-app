import { describe, expect, it } from "vitest";
import type { Chapter, LessonSummary } from "../commands/content";
import { nextLessonId } from "./nextLesson";

function lesson(id: string, inPreparation = false): LessonSummary {
  return inPreparation
    ? {
        id,
        kind: null,
        title: null,
        summary: null,
        estimated_minutes: null,
        in_preparation: true,
      }
    : {
        id,
        kind: "lesson",
        title: id,
        summary: "",
        estimated_minutes: 5,
        in_preparation: false,
      };
}

function chapter(id: string, lessons: LessonSummary[]): Chapter {
  return { id, title: id, description: "", lessons };
}

describe("次のレッスンへ", () => {
  it("準備中のレッスンを飛ばして、次の準備中でないレッスンを開く", () => {
    const chapters = [
      chapter("ch04", [
        lesson("ch04-01"),
        lesson("ch04-02", true),
        lesson("ch04-03"),
      ]),
    ];
    expect(nextLessonId(chapters, "ch04-01")).toBe("ch04-03");
  });

  it("章をまたいで、次の章の最初のレッスンを開く", () => {
    const chapters = [
      chapter("ch01", [lesson("ch01-01")]),
      chapter("ch02", [lesson("ch02-01")]),
    ];
    expect(nextLessonId(chapters, "ch01-01")).toBe("ch02-01");
  });

  it("後にあるレッスンがすべて準備中なら、無し", () => {
    const chapters = [
      chapter("ch01", [lesson("ch01-01"), lesson("ch01-02", true)]),
      chapter("ch02", [lesson("ch02-01", true)]),
    ];
    expect(nextLessonId(chapters, "ch01-01")).toBeNull();
  });

  it("最後のレッスンでは、無し", () => {
    const chapters = [chapter("ch01", [lesson("ch01-01"), lesson("ch01-02")])];
    expect(nextLessonId(chapters, "ch01-02")).toBeNull();
  });

  it("章の並びにないレッスンでは、無し", () => {
    const chapters = [chapter("ch01", [lesson("ch01-01")])];
    expect(nextLessonId(chapters, "nope")).toBeNull();
  });
});
