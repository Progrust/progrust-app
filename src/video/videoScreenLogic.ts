// 動画画面の計算（documents/spec/lesson-flow/progress.md、flow.md、
// documents/spec/screens/video-screen.md）。画面の組み立てから分けて、自動テストで確かめられるようにする。
import type { LessonKind } from "../commands/content";
import type { LessonProgress } from "../commands/progress";
import { sentenceStarts } from "./playerMath";
import type { Timeline } from "./timeline";

// 開いたときの再生位置（フレーム）。保存した `last_sentence_id` の文の頭。
// null のとき、台本にない ID のとき、台本の最後の文のときは、動画の先頭（ADR 0169）。
export function initialFrame(
  timeline: Timeline,
  lastSentenceId: string | null,
): number {
  if (lastSentenceId === null) {
    return 0;
  }
  const sentences = sentenceStarts(timeline);
  const index = sentences.findIndex((s) => s.id === lastSentenceId);
  if (index < 0 || index === sentences.length - 1) {
    return 0;
  }
  return sentences[index].from;
}

// 右端の行動。「演習へ」「次のレッスンへ」、または無し。
export type RightAction = "exercise" | "next_lesson" | null;

// 右端の行動を決める。
// - reachedEnd: この画面の中で、動画の末尾に着いたか（再生でもシークでも）。保存した video_watched と合わせて見る。
// - hasNextLesson: 「次のレッスンへ」で開くレッスンがあるか。
// 動画の無い種類（章末復習・生成問題）では、動画画面を使わないので無し。
export function rightAction(
  kind: LessonKind,
  progress: LessonProgress | null | undefined,
  reachedEnd: boolean,
  hasNextLesson: boolean,
): RightAction {
  const watched = reachedEnd || progress?.video_watched === true;
  if (kind === "video_only") {
    // 動画のみのレッスンは、最後まで再生すると完了になる。「演習へ」は出さない。
    return watched && hasNextLesson ? "next_lesson" : null;
  }
  if (kind === "lesson") {
    const completed = progress?.status === "completed";
    return watched || completed ? "exercise" : null;
  }
  return null;
}

// 再生位置を保存するきっかけ。
export type SaveTrigger = "sentence_changed" | "paused" | "ended" | "leave";

export type SaveVideoPositionArgs = {
  lesson_id: string;
  last_sentence_id: string;
  video_watched: boolean;
};

// きっかけから、`save_video_position` の引数を作る。
// 再生中の文が無いとき（台本に文が無いとき）は、保存するものが無いので null。
// 末尾に着いたあと（reachedEnd）は、画面を離れるときなども true を渡す。
// バックエンドは、一度 true になった値を false に戻さない。
export function savePositionArgs(
  lessonId: string,
  trigger: SaveTrigger,
  sentenceId: string | undefined,
  reachedEnd: boolean,
): SaveVideoPositionArgs | null {
  if (sentenceId === undefined) {
    return null;
  }
  return {
    lesson_id: lessonId,
    last_sentence_id: sentenceId,
    video_watched: trigger === "ended" || reachedEnd,
  };
}
