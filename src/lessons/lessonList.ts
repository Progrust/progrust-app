// レッスン一覧の表示用の値を、章の並びと進捗から求める。
// 決まりは documents/spec/screens/lesson-list.md。
import type { Language, LessonKind } from "../commands/content";
import type { ProgressSummary } from "../commands/progress";
import type { NumberedChapter } from "../chapterNumbers";

export type LessonState = "todo" | "doing" | "done";

export type LessonRow =
  | { preparing: true; id: string; number: string }
  | {
      preparing: false;
      id: string;
      number: string;
      title: string;
      summary: string;
      minutes: number | null;
      kind: LessonKind;
      state: LessonState;
      recommended: boolean;
    };

export type ChapterSection = {
  language: Language;
  number: number;
  // 章の表示（「第4章」「C 第1章」）と、章の番号の印の中の文字（「4」「C1」）。
  label: string;
  badge: string;
  color: number;
  id: string;
  title: string;
  description: string;
  // 準備中でないレッスンのうち、完了したレッスンの数。
  done: number;
  // 準備中でないレッスンの数。0 のときは、完了数の代わりに「準備中」と表示する。
  total: number;
  rows: LessonRow[];
};

// 「C 言語編」「Rust 編」の区切り。章の無い言語の区切りは作らない。
export type LanguagePart = {
  language: Language;
  title: string;
  sections: ChapterSection[];
};

// 区切りの見出しと、表示する順（C 言語編が先）。
const PARTS: { language: Language; title: string }[] = [
  { language: "c", title: "C 言語編" },
  { language: "rust", title: "Rust 編" },
];

export type LessonListModel = {
  parts: LanguagePart[];
  done: number;
  total: number;
  // 推奨レッスンのある章。推奨レッスンが無いときは null。
  recommendedChapter: ChapterSection | null;
  recommendedRow: Extract<LessonRow, { preparing: false }> | null;
};

export function buildLessonList(
  chapters: NumberedChapter[],
  progress: ProgressSummary,
): LessonListModel {
  let doneAll = 0;
  let totalAll = 0;
  let recommendedChapter: ChapterSection | null = null;
  let recommendedRow: LessonListModel["recommendedRow"] = null;
  const sections = chapters.map((numbered) => {
    const { chapter, lessons } = numbered;
    let done = 0;
    let total = 0;
    const rows = lessons.map(({ lesson, number: lessonNo }): LessonRow => {
      if (lesson.in_preparation) {
        return { preparing: true, id: lesson.id, number: lessonNo };
      }
      const status = progress.lessons[lesson.id]?.status;
      const state: LessonState =
        status === "completed"
          ? "done"
          : status === "in_progress"
            ? "doing"
            : "todo";
      total += 1;
      if (state === "done") {
        done += 1;
      }
      return {
        preparing: false,
        id: lesson.id,
        number: lessonNo,
        title: lesson.title ?? "",
        summary: lesson.summary ?? "",
        minutes: lesson.estimated_minutes,
        kind: lesson.kind ?? "lesson",
        state,
        recommended: lesson.id === progress.recommended_lesson_id,
      };
    });
    doneAll += done;
    totalAll += total;
    const section: ChapterSection = {
      language: numbered.language,
      number: numbered.number,
      label: numbered.label,
      badge: numbered.badge,
      color: numbered.color,
      id: chapter.id,
      title: chapter.title,
      description: chapter.description,
      done,
      total,
      rows,
    };
    for (const row of rows) {
      if (!row.preparing && row.recommended) {
        recommendedChapter = section;
        recommendedRow = row;
      }
    }
    return section;
  });
  const parts = PARTS.map(({ language, title }) => ({
    language,
    title,
    sections: sections.filter((section) => section.language === language),
  })).filter((part) => part.sections.length > 0);
  return {
    parts,
    done: doneAll,
    total: totalAll,
    recommendedChapter,
    recommendedRow,
  };
}
