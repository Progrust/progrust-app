import { expect, it } from "vitest";
import {
  chapterBadge,
  chapterColor,
  chapterLabel,
  findLesson,
  lessonNumber,
  numberChapters,
  numberWithinLanguage,
} from "./chapterNumbers";
import type { Chapter, Language } from "./commands/content";

function chapter(
  id: string,
  lessonIds: string[],
  language: Language = "rust",
  number = 1,
): Chapter {
  return {
    id,
    title: id,
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

// C の章 3 つの後に、Rust の章 4 つ（list_chapters が返す並びと number）。
const mixed = [
  chapter(
    "c01",
    ["c01-01", "c01-02", "c01-03", "c01-04", "c01-review"],
    "c",
    1,
  ),
  chapter("c02", ["c02-01", "c02-review"], "c", 2),
  chapter("c03", ["c03-01", "c03-review"], "c", 3),
  chapter("ch01", ["ch01-01", "ch01-review"], "rust", 1),
  chapter("ch02", ["ch02-01", "ch02-review"], "rust", 2),
  chapter("ch03", ["ch03-01", "ch03-review"], "rust", 3),
  chapter(
    "ch04",
    ["ch04-01", "ch04-02", "ch04-03", "ch04-04", "ch04-review"],
    "rust",
    4,
  ),
];

it("章カラーは 1〜12 で、13 章目から 1 色目に戻る", () => {
  expect([1, 2, 11, 12, 13, 14, 24, 25].map(chapterColor)).toEqual([
    1, 2, 11, 12, 1, 2, 12, 1,
  ]);
});

it("章とレッスンの番号の表示は、言語で形が変わる", () => {
  expect(chapterLabel("rust", 4)).toBe("第4章");
  expect(chapterLabel("c", 1)).toBe("C 第1章");
  expect(chapterBadge("rust", 4)).toBe("4");
  expect(chapterBadge("c", 1)).toBe("C1");
  expect(lessonNumber("rust", 4, 3)).toBe("4-4");
  expect(lessonNumber("rust", 12, 0)).toBe("12-1");
  expect(lessonNumber("c", 1, 3)).toBe("C1-4");
});

it("章の番号は、同じ言語の章の中での位置になる", () => {
  expect(
    numberWithinLanguage(["c", "c", "c", "rust", "rust", "rust", "rust"]),
  ).toEqual([1, 2, 3, 1, 2, 3, 4]);
  expect(numberWithinLanguage(["rust", "rust"])).toEqual([1, 2]);
  expect(numberWithinLanguage([])).toEqual([]);
});

it("C の章と Rust の章が並ぶと、番号と章カラーは、配列の位置ではなく章の number で決まる", () => {
  const numbered = numberChapters(mixed);
  expect(
    numbered.map((item) => [
      item.chapter.id,
      item.label,
      item.badge,
      item.color,
    ]),
  ).toEqual([
    ["c01", "C 第1章", "C1", 1],
    ["c02", "C 第2章", "C2", 2],
    ["c03", "C 第3章", "C3", 3],
    ["ch01", "第1章", "1", 1],
    ["ch02", "第2章", "2", 2],
    ["ch03", "第3章", "3", 3],
    ["ch04", "第4章", "4", 4],
  ]);
  // C の第1章の 4 番目のレッスンは C1-4、Rust の第4章の 4 番目のレッスンは 4-4。
  expect(findLesson(numbered, "c01-04")?.lesson.number).toBe("C1-4");
  expect(findLesson(numbered, "ch04-04")?.lesson.number).toBe("4-4");
  // C の第1章と Rust の第1章は同じ章カラー。
  expect(findLesson(numbered, "c01-01")?.chapter.color).toBe(
    findLesson(numbered, "ch01-01")?.chapter.color,
  );
  // 章末復習レッスンにも、同じ決め方で番号を付ける。
  expect(findLesson(numbered, "c01-review")?.lesson.number).toBe("C1-5");
  expect(findLesson(numbered, "ch04-review")?.lesson.number).toBe("4-5");
});

it("C の章が無いときの Rust の章の番号と章カラーは、C の章があるときと変わらない", () => {
  const withC = numberChapters(mixed);
  const rustOnly = numberChapters(mixed.filter((c) => c.language === "rust"));
  const view = (items: typeof withC) =>
    items
      .filter((item) => item.language === "rust")
      .map((item) => ({
        id: item.chapter.id,
        label: item.label,
        badge: item.badge,
        color: item.color,
        lessons: item.lessons.map((lesson) => lesson.number),
      }));
  expect(view(rustOnly)).toEqual(view(withC));
  expect(rustOnly[0]?.lessons.map((item) => item.number)).toEqual([
    "1-1",
    "1-2",
  ]);
});

it("13 章目の章カラーは 1 色目に戻る", () => {
  const chapters = Array.from({ length: 13 }, (_, index) =>
    chapter(`ch${index + 1}`, [`ch${index + 1}-01`], "rust", index + 1),
  );
  const numbered = numberChapters(chapters);
  expect(numbered[11]?.color).toBe(12);
  expect(numbered[12]?.number).toBe(13);
  expect(numbered[12]?.color).toBe(1);
  expect(numbered[12]?.lessons[0]?.number).toBe("13-1");
});

it("レッスン ID から、章とレッスンの番号を探す", () => {
  const numbered = numberChapters([
    chapter("ch01", ["ch01-01"], "rust", 1),
    chapter("ch02", ["ch02-01", "ch02-02"], "rust", 2),
  ]);
  const found = findLesson(numbered, "ch02-02");
  expect(found?.chapter.number).toBe(2);
  expect(found?.lesson.number).toBe("2-2");
  expect(findLesson(numbered, "ch99-01")).toBeNull();
});
