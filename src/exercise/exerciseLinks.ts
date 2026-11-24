// 演習画面の左上のリンクと「動画を見直す」（documents/spec/screens/exercise-screen.md、ADR 0204）。
import type { LessonKind } from "../commands/content";
import type { Route } from "../router";

export type ExerciseLinks = {
  // 左上のリンクの表示名と行き先。
  backLabel: string;
  backRoute: Route;
  // 「動画を見直す」の行き先。動画の無い演習（章末復習レッスン・生成問題）では null で、表示しない。
  reviewVideoRoute: Route | null;
};

// レッスン・章末復習レッスンは「レッスン一覧」、生成問題は「生成問題一覧」に戻る。
// 動画があるのは、kind が lesson のレッスンだけ。
export function exerciseLinks(
  kind: LessonKind,
  lessonId: string,
): ExerciseLinks {
  return {
    backLabel: kind === "generated" ? "生成問題一覧" : "レッスン一覧",
    backRoute:
      kind === "generated" ? { name: "generated" } : { name: "lessons" },
    reviewVideoRoute: kind === "lesson" ? { name: "video", lessonId } : null,
  };
}
