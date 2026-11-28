// 章とレッスンの番号と、その表示の形と、章カラー。
// 決まりは documents/spec/content-format/chapters-json.md「章とレッスンの番号」と、
// design-system/progrust/README.md「章カラー」、design-system/progrust/components/ChapterBadge/README.md。
// 画面と動画は、番号の表示の形をここの関数で作り、画面ごとに組み立てない。
import type { Chapter, Language, LessonSummary } from "./commands/content";

// 章カラーの数。13 章目からは 1 色目に戻る。
export const CHAPTER_COLOR_COUNT = 12;

export type NumberedLesson = {
  lesson: LessonSummary;
  // レッスンの番号の表示。例: "4-4"、"C1-4"。
  number: string;
};

export type NumberedChapter = {
  chapter: Chapter;
  language: Language;
  // 章の番号（同じ言語の章の中での位置。最初の章が 1）。
  number: number;
  // 章の表示。例: "第4章"、"C 第1章"。
  label: string;
  // 章の番号の印（pg-badge）の中の文字。例: "4"、"C1"。
  badge: string;
  // `data-chapter` に付ける値。
  color: number;
  lessons: NumberedLesson[];
};

// 章の番号から、章カラーの番号（1〜12）を求める。C の第1章は、Rust の第1章と同じ色になる。
export function chapterColor(chapterNumber: number): number {
  return ((chapterNumber - 1) % CHAPTER_COLOR_COUNT) + 1;
}

// 章の表示（Rust は「第4章」、C は「C 第1章」）。
export function chapterLabel(
  language: Language,
  chapterNumber: number,
): string {
  return language === "c" ? `C 第${chapterNumber}章` : `第${chapterNumber}章`;
}

// 章の番号の印の中の文字（Rust は「4」、C は「C1」）。
export function chapterBadge(
  language: Language,
  chapterNumber: number,
): string {
  return language === "c" ? `C${chapterNumber}` : `${chapterNumber}`;
}

// レッスンの番号の表示を、章の言語と番号と、章内の位置（0 始まり）から求める（Rust は「4-4」、C は「C1-4」）。
export function lessonNumber(
  language: Language,
  chapterNumber: number,
  indexInChapter: number,
): string {
  return `${chapterBadge(language, chapterNumber)}-${indexInChapter + 1}`;
}

// 章の並びの言語から、それぞれの章の番号（同じ言語の章の中での位置。1 から）を求める。
// アプリは list_chapters の number を使う。content/chapters.json を直接読む開発用ツールが使う。
export function numberWithinLanguage(languages: Language[]): number[] {
  const counts: { [language in Language]: number } = { c: 0, rust: 0 };
  return languages.map((language) => {
    counts[language] += 1;
    return counts[language];
  });
}

// `list_chapters` の結果に、章とレッスンの番号の表示と章カラーを付ける。
// 章の番号は、配列の位置ではなく、list_chapters の number を使う。
export function numberChapters(chapters: Chapter[]): NumberedChapter[] {
  return chapters.map((chapter) => {
    const { language, number } = chapter;
    return {
      chapter,
      language,
      number,
      label: chapterLabel(language, number),
      badge: chapterBadge(language, number),
      color: chapterColor(number),
      lessons: chapter.lessons.map((lesson, lessonIndex) => ({
        lesson,
        number: lessonNumber(language, number, lessonIndex),
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
