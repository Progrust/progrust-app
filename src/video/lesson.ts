// 動画に出す章とレッスンの番号と名前（documents/spec/content-format/chapters-json.md「章とレッスンの番号」）。
import type { Chapter, LessonDefinition } from "../commands/content";

export type LessonInfo = {
  // 章の番号（最初の章を 1 とする）。
  chapterNumber: number;
  chapterTitle: string;
  // レッスンの番号（例: "4-4"）。
  lessonNumber: string;
  // lesson.json の title。
  lessonTitle: string;
};

// 章の並び（list_chapters の chapters）とレッスン定義から、番号と名前を決める。
export function lessonInfo(
  chapters: Chapter[],
  lesson: LessonDefinition,
): LessonInfo {
  const chapterIndex = chapters.findIndex((c) => c.id === lesson.chapter);
  if (chapterIndex < 0) {
    throw new Error(
      `レッスン ${lesson.id} の章 ${lesson.chapter} が、章の並びにない`,
    );
  }
  const chapter = chapters[chapterIndex];
  const lessonIndex = chapter.lessons.findIndex((l) => l.id === lesson.id);
  if (lessonIndex < 0) {
    throw new Error(`レッスン ${lesson.id} が、章 ${chapter.id} の並びにない`);
  }
  const chapterNumber = chapterIndex + 1;
  return {
    chapterNumber,
    chapterTitle: chapter.title,
    lessonNumber: `${chapterNumber}-${lessonIndex + 1}`,
    lessonTitle: lesson.title,
  };
}
