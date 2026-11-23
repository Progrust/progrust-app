// 進捗のコマンド（documents/spec/architecture/commands/progress.md）。
import { invoke } from "@tauri-apps/api/core";

export type LessonStatus = "in_progress" | "completed";

// 1 つのレッスンの進捗（documents/spec/lesson-flow/progress.md）。どのフィールドも省かれない。
// 日時は RFC 3339 の文字列。
export type LessonProgress = {
  status: LessonStatus;
  first_completed_at: string | null;
  last_completed_at: string | null;
  completion_count: number;
  last_opened_at: string;
  last_sentence_id: string | null;
  video_watched: boolean;
  last_passed_session_id: string | null;
};

export type ProgressSummary = {
  // レッスン ID から進捗への対応。一度も開いていないレッスンのキーは無い。
  lessons: { [lessonId: string]: LessonProgress };
  // 推奨レッスンの ID。無いときは null。
  recommended_lesson_id: string | null;
};

// open_lesson の戻り値。試行セッション・生成問題・下書きは、M2 以降で入る（いまは null）。
export type OpenedLesson = {
  session_id: string | null;
  progress: LessonProgress | null;
  generated: unknown | null;
  draft: unknown | null;
};

// レッスンを開いたことを記録し、更新した後の進捗を受け取る。
export function openLesson(lessonId: string): Promise<OpenedLesson> {
  return invoke<OpenedLesson>("open_lesson", { lesson_id: lessonId });
}

// 動画の再生位置と、最後まで再生したかを保存し、更新した後の進捗を受け取る。
export function saveVideoPosition(
  lessonId: string,
  lastSentenceId: string,
  videoWatched: boolean,
): Promise<LessonProgress> {
  return invoke<LessonProgress>("save_video_position", {
    lesson_id: lessonId,
    last_sentence_id: lastSentenceId,
    video_watched: videoWatched,
  });
}

// 進捗と推奨レッスンを、バックエンドから受け取る。
export function getProgress(): Promise<ProgressSummary> {
  return invoke<ProgressSummary>("get_progress");
}
