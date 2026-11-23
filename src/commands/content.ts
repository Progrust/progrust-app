// 教材のコマンド（documents/spec/architecture/commands/content.md）。
import { invoke } from "@tauri-apps/api/core";

export type LessonKind =
  "lesson" | "video_only" | "chapter_review" | "generated";

// 準備中のレッスンでは、in_preparation 以外が null になる。
export type LessonSummary = {
  id: string;
  kind: LessonKind | null;
  title: string | null;
  summary: string | null;
  estimated_minutes: number | null;
  in_preparation: boolean;
};

export type Chapter = {
  id: string;
  title: string;
  description: string;
  lessons: LessonSummary[];
};

export type ChapterList = {
  chapters: Chapter[];
};

// レッスン定義（documents/spec/content-format/lesson-json.md）。
export type LessonDefinition = {
  id: string;
  chapter: string;
  kind: LessonKind;
  title: string;
  summary?: string;
  tags?: string[];
  primary_elements: string[];
  secondary_elements: string[];
  estimated_minutes?: number;
  exercise?: {
    layout: "single" | "multi";
    judge: {
      mode: "stdout" | "test";
      target: string;
      min_learner_tests?: number;
    };
    dependencies: string[];
    readonly_files: string[];
    case_labels: { [name: string]: string };
  };
};

// 動画の台本（documents/spec/content-format/video/script-json.md）。
export type Script = {
  ai_note?: string;
  code_blocks?: {
    id: string;
    label?: string;
    language: string;
    code: string;
    start_line?: number;
    diff?: boolean;
  }[];
  scenes: { id: string; name: string; lead?: number; tail?: number }[];
  sentences: {
    id: string;
    scene: string;
    text: string;
    say: string;
    pause_after?: number;
  }[];
};

export type Exercise = {
  problem_md: string;
  explanation_md: string;
  starter_files: { [path: string]: string };
};

export type LessonDetail = {
  lesson: LessonDefinition;
  script: Script | null;
  voice_dir: string | null;
  exercise: Exercise | null;
};

// 章とレッスンの並びを、バックエンドから受け取る。
export function listChapters(): Promise<ChapterList> {
  return invoke<ChapterList>("list_chapters");
}

// レッスンの定義・台本・音声のディレクトリ・演習の内容を、バックエンドから受け取る。
export function getLesson(lessonId: string): Promise<LessonDetail> {
  return invoke<LessonDetail>("get_lesson", { lesson_id: lessonId });
}
