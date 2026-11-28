// 動画に出す章とレッスンの番号と名前（documents/spec/content-format/chapters-json.md「章とレッスンの番号」）。
import { chapterLabel, lessonNumber } from "../chapterNumbers";
import type { Chapter, Language, LessonDefinition } from "../commands/content";

export type LessonInfo = {
  // レッスンの言語（属する章の言語）。
  language: Language;
  // 章の番号（同じ言語の章の中での位置。最初の章を 1 とする）。章カラーもこの番号で決まる。
  chapterNumber: number;
  // 章の番号の表示（例: "第4章"、"C 第1章"）。
  chapterLabel: string;
  chapterTitle: string;
  // レッスンの番号の表示（例: "4-4"、"C1-4"）。
  lessonNumber: string;
  // lesson.json の title。
  lessonTitle: string;
};

// 章の並び（list_chapters の chapters）とレッスン定義から、番号と名前を決める。
// 章の番号と言語は、章の number・language を使う。
export function lessonInfo(
  chapters: Chapter[],
  lesson: LessonDefinition,
): LessonInfo {
  const chapter = chapters.find((c) => c.id === lesson.chapter);
  if (chapter === undefined) {
    throw new Error(
      `レッスン ${lesson.id} の章 ${lesson.chapter} が、章の並びにない`,
    );
  }
  const lessonIndex = chapter.lessons.findIndex((l) => l.id === lesson.id);
  if (lessonIndex < 0) {
    throw new Error(`レッスン ${lesson.id} が、章 ${chapter.id} の並びにない`);
  }
  const { language, number } = chapter;
  return {
    language,
    chapterNumber: number,
    chapterLabel: chapterLabel(language, number),
    chapterTitle: chapter.title,
    lessonNumber: lessonNumber(language, number, lessonIndex),
    lessonTitle: lesson.title,
  };
}
