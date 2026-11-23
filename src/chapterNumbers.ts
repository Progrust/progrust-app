// 章とレッスンの番号と章カラー。
// 決まりは documents/spec/content-format/chapters-json.md「章とレッスンの番号」と、
// design-system/progrust/README.md「章カラー」。
import type { Chapter, LessonSummary } from "./commands/content";

// 章カラーの数。13 章目からは 1 色目に戻る。
export const CHAPTER_COLOR_COUNT = 12;

export type NumberedLesson = {
  lesson: LessonSummary;
  // 「{章の番号}-{章内の位置}」。例: "4-4"。
  number: string;
};

export type NumberedChapter = {
  chapter: Chapter;
  // 章の番号。最初の章が 1。
  number: number;
  // `data-chapter` に付ける値。
  color: number;
  lessons: NumberedLesson[];
};

// 章の番号から、章カラーの番号（1〜12）を求める。
export function chapterColor(chapterNumber: number): number {
  return ((chapterNumber - 1) % CHAPTER_COLOR_COUNT) + 1;
}

// レッスンの番号を、章の番号と章内の位置（0 始まり）から求める。
export function lessonNumber(
  chapterNumber: number,
  indexInChapter: number,
): string {
  return `${chapterNumber}-${indexInChapter + 1}`;
}

// `list_chapters` の結果に、章とレッスンの番号と章カラーを付ける。
export function numberChapters(chapters: Chapter[]): NumberedChapter[] {
  return chapters.map((chapter, chapterIndex) => {
    const number = chapterIndex + 1;
    return {
      chapter,
      number,
      color: chapterColor(number),
      lessons: chapter.lessons.map((lesson, lessonIndex) => ({
        lesson,
        number: lessonNumber(number, lessonIndex),
      })),
    };
  });
}

// レッスン ID から、そのレッスンのある章と番号を探す。無ければ null。
export function findLesson(
  numbered: NumberedChapter[],
  lessonId: string,
): { chapter: NumberedChapter; lesson: NumberedLesson } | null {
  for (const chapter of numbered) {
    const lesson = chapter.lessons.find((item) => item.lesson.id === lessonId);
    if (lesson !== undefined) {
      return { chapter, lesson };
    }
  }
  return null;
}
