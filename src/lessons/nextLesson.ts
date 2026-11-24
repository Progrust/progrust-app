// 「次のレッスンへ」で開くレッスン（documents/spec/lesson-flow/order.md）。
// 動画画面と演習画面で使う。
import type { Chapter, LessonKind, LessonSummary } from "../commands/content";
import type { Route } from "../router";

// 学習順でそのレッスンより後にある、準備中でないレッスンのうち最初のもの。
// 無いときと、レッスンが章の並びにないときは null。
export function nextLesson(
  chapters: Chapter[],
  lessonId: string,
): LessonSummary | null {
  const lessons = chapters.flatMap((chapter) => chapter.lessons);
  const index = lessons.findIndex((lesson) => lesson.id === lessonId);
  if (index < 0) {
    return null;
  }
  return (
    lessons.slice(index + 1).find((lesson) => !lesson.in_preparation) ?? null
  );
}

// nextLesson の ID。
export function nextLessonId(
  chapters: Chapter[],
  lessonId: string,
): string | null {
  return nextLesson(chapters, lessonId)?.id ?? null;
}

// レッスンを開くときの画面。章末復習レッスンは演習から始まり（documents/spec/lesson-flow/flow.md）、
// ほかのレッスンは動画画面から始まる。
export function lessonRoute(lessonId: string, kind: LessonKind | null): Route {
  return kind === "chapter_review"
    ? { name: "exercise", lessonId }
    : { name: "video", lessonId };
}
