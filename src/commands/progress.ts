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

// 進捗と推奨レッスンを、バックエンドから受け取る。
export function getProgress(): Promise<ProgressSummary> {
  return invoke<ProgressSummary>("get_progress");
}
