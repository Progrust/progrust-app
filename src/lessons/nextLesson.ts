// 「次のレッスンへ」で開くレッスン（documents/spec/lesson-flow/order.md）。
// 動画画面と、M2 の演習画面で使う。
import type { Chapter } from "../commands/content";

// 学習順でそのレッスンより後にある、準備中でないレッスンのうち最初のものの ID。
// 無いときと、レッスンが章の並びにないときは null。
export function nextLessonId(
  chapters: Chapter[],
  lessonId: string,
): string | null {
  const lessons = chapters.flatMap((chapter) => chapter.lessons);
  const index = lessons.findIndex((lesson) => lesson.id === lessonId);
  if (index < 0) {
    return null;
  }
  const next = lessons
    .slice(index + 1)
    .find((lesson) => !lesson.in_preparation);
  return next?.id ?? null;
}
