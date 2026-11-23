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

// エディタ上の全ファイル。キーは cargo パッケージのルートからの相対パス（例: "src/main.rs"）。
export type DraftFiles = { [path: string]: string };

// 演習コードの下書き（documents/spec/lesson-flow/drafts.md）。
export type Draft = {
  schema_version: number;
  lesson_id: string;
  // 保存した日時（RFC 3339）。バックエンドが付ける。
  updated_at: string;
  files: DraftFiles;
};

// open_lesson の戻り値。生成問題は、まだ入らない（いまは null）。
export type OpenedLesson = {
  // 開始または継続した試行セッションの ID。動画のみのレッスンでは null。
  session_id: string | null;
  progress: LessonProgress | null;
  generated: unknown | null;
  // 下書き。無いときと、演習の無いレッスンでは null。
  draft: Draft | null;
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

// エディタ上の全ファイルを下書きに保存する。ファイルパスが src/ か tests/ の下の .rs でないときは、
// invalid_argument で拒否される。
export function saveDraft(lessonId: string, files: DraftFiles): Promise<null> {
  return invoke<null>("save_draft", { lesson_id: lessonId, files });
}

// 下書きを教材の初期コードで上書きし、上書きした後の下書きを受け取る。
export function resetDraft(lessonId: string): Promise<Draft> {
  return invoke<Draft>("reset_draft", { lesson_id: lessonId });
}

// 進捗と推奨レッスンを、バックエンドから受け取る。
export function getProgress(): Promise<ProgressSummary> {
  return invoke<ProgressSummary>("get_progress");
}
