import { expect, it } from "vitest";
import {
  chapterColor,
  findLesson,
  lessonNumber,
  numberChapters,
} from "./chapterNumbers";
import type { Chapter } from "./commands/content";

function chapter(id: string, lessonIds: string[]): Chapter {
  return {
    id,
    title: id,
    description: "",
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

it("章カラーは 1〜12 で、13 章目から 1 色目に戻る", () => {
  expect([1, 2, 11, 12, 13, 14, 24, 25].map(chapterColor)).toEqual([
    1, 2, 11, 12, 1, 2, 12, 1,
  ]);
});

it("レッスンの番号は「章の番号-章内の位置」になる", () => {
  expect(lessonNumber(4, 3)).toBe("4-4");
  expect(lessonNumber(12, 0)).toBe("12-1");
});

it("章の番号は配列の位置、レッスンの番号は章内の位置から付く", () => {
  const numbered = numberChapters([
    chapter("ch01", ["ch01-01", "ch01-review"]),
    chapter("ch02", ["ch02-01", "ch02-02", "ch02-review"]),
  ]);
  expect(numbered.map((item) => [item.number, item.color])).toEqual([
    [1, 1],
    [2, 2],
  ]);
  expect(numbered[0]?.lessons.map((item) => item.number)).toEqual([
    "1-1",
    "1-2",
  ]);
  // 章末復習レッスンにも、同じ決め方で番号を付ける。
  expect(numbered[1]?.lessons.map((item) => item.number)).toEqual([
    "2-1",
    "2-2",
    "2-3",
  ]);
});

it("13 章目の章カラーは 1 色目に戻る", () => {
  const chapters = Array.from({ length: 13 }, (_, index) =>
    chapter(`ch${index + 1}`, [`ch${index + 1}-01`]),
  );
  const numbered = numberChapters(chapters);
  expect(numbered[11]?.color).toBe(12);
  expect(numbered[12]?.number).toBe(13);
  expect(numbered[12]?.color).toBe(1);
  expect(numbered[12]?.lessons[0]?.number).toBe("13-1");
});

it("レッスン ID から、章とレッスンの番号を探す", () => {
  const numbered = numberChapters([
    chapter("ch01", ["ch01-01"]),
    chapter("ch02", ["ch02-01", "ch02-02"]),
  ]);
  const found = findLesson(numbered, "ch02-02");
  expect(found?.chapter.number).toBe(2);
  expect(found?.lesson.number).toBe("2-2");
  expect(findLesson(numbered, "ch99-01")).toBeNull();
});
