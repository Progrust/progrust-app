import { describe, expect, it } from "vitest";
import type { Chapter, LessonSummary } from "../commands/content";
import { lessonRoute, nextLesson, nextLessonId } from "./nextLesson";

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
  return {
    id,
    title: id,
    description: "",
    language: "rust",
    number: 1,
    lessons,
  };
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

  it("C の最後のレッスン c03-review の次は、Rust の最初のレッスン ch01-01 になる", () => {
    const chapters = [
      {
        ...chapter("c01", [lesson("c01-01"), lesson("c01-review")]),
        language: "c" as const,
      },
      {
        ...chapter("c03", [lesson("c03-01"), lesson("c03-review")]),
        language: "c" as const,
        number: 3,
      },
      chapter("ch01", [lesson("ch01-01"), lesson("ch01-review")]),
    ];
    expect(nextLessonId(chapters, "c03-review")).toBe("ch01-01");
    // ch01-01 が準備中なら、その次の準備中でないレッスン。
    chapters[2] = chapter("ch01", [
      lesson("ch01-01", true),
      lesson("ch01-review"),
    ]);
    expect(nextLessonId(chapters, "c03-review")).toBe("ch01-review");
  });

  it("章の並びにないレッスンでは、無し", () => {
    const chapters = [chapter("ch01", [lesson("ch01-01")])];
    expect(nextLessonId(chapters, "nope")).toBeNull();
  });
});

describe("レッスンを開く画面", () => {
  it("章末復習レッスンは演習画面、ほかは動画画面", () => {
    expect(lessonRoute("ch04-07", "chapter_review")).toEqual({
      name: "exercise",
      lessonId: "ch04-07",
    });
    expect(lessonRoute("ch04-01", "lesson")).toEqual({
      name: "video",
      lessonId: "ch04-01",
    });
    expect(lessonRoute("ch04-02", "video_only")).toEqual({
      name: "video",
      lessonId: "ch04-02",
    });
  });

  it("nextLesson は、次のレッスンの種類も返す", () => {
    const review = { ...lesson("ch04-02"), kind: "chapter_review" as const };
    const chapters = [chapter("ch04", [lesson("ch04-01"), review])];
    expect(nextLesson(chapters, "ch04-01")).toEqual(review);
  });
});
