// 学習記録・分析のコマンド（documents/spec/architecture/commands/learning-record.md）。
import { invoke } from "@tauri-apps/api/core";
import type { Language } from "./content";

export type NextLesson = {
  lesson_id: string;
  chapter_id: string;
  chapter_title: string;
  // レッスンの言語と、属する章の番号（同じ言語の章の中での位置）。
  language: Language;
  chapter_number: number;
  title: string;
  summary: string;
  // 進捗があれば true。false は未着手（「始める」）。
  started: boolean;
};

export type ChapterCount = {
  id: string;
  title: string;
  language: Language;
  number: number;
  // 準備中のレッスンを数えない。準備中でないレッスンが 1 つも無い章は total が 0。
  completed: number;
  total: number;
};

export type AnalysisState = "none" | "ok" | "unreadable";

export type ElementSummary = {
  id: string;
  name: string;
  level: number;
  provisional: boolean;
};

export type DashboardAnalysis = {
  state: AnalysisState;
  weak_elements: ElementSummary[];
  review_due_elements: ElementSummary[];
  regressed_elements: ElementSummary[];
  last_4_weeks: number[];
  summary_md: string | null;
};

export type Dashboard = {
  // すべて完了していて無いときは null。
  next_lesson: NextLesson | null;
  // どちらも準備中のレッスンを数えない。
  lesson_counts: { completed: number; total: number };
  // 章ごとの完了率に表示する章（9 章まで）。学習順。
  chapters: ChapterCount[];
  // 分析に関する項目。M5 までは、state が none で、ほかは空。
  analysis: DashboardAnalysis;
};

// トップページに表示する値を、バックエンドから受け取る。
export function getDashboard(): Promise<Dashboard> {
  return invoke<Dashboard>("get_dashboard");
}
