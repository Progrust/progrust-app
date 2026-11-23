// レッスン一覧の表示用の値を、章の並びと進捗から求める。
// 決まりは documents/spec/screens/lesson-list.md。
import type { LessonKind } from "../commands/content";
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
  number: number;
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

export type LessonListModel = {
  sections: ChapterSection[];
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
  const sections = chapters.map(({ chapter, number, color, lessons }) => {
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
      number,
      color,
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
  return {
    sections,
    done: doneAll,
    total: totalAll,
    recommendedChapter,
    recommendedRow,
  };
}
