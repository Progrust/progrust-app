import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import type { Chapter, ChapterList } from "../commands/content";
import type { LessonProgress, ProgressSummary } from "../commands/progress";
import { NavigationContext } from "../router";
import type { Route } from "../router";
import { LessonList } from "./LessonsScreen";

function lesson(id: string, kind: "lesson" | "video_only" | "chapter_review") {
  return {
    id,
    kind,
    title: `タイトル ${id}`,
    summary: `概要 ${id}`,
    estimated_minutes: 10,
    in_preparation: false,
  };
}

function preparing(id: string) {
  return {
    id,
    kind: null,
    title: null,
    summary: null,
    estimated_minutes: null,
    in_preparation: true,
  };
}

function progressOf(status: "in_progress" | "completed"): LessonProgress {
  return {
    status,
    first_completed_at: null,
    last_completed_at: null,
    completion_count: 0,
    last_opened_at: "2026-01-01T00:00:00Z",
    last_sentence_id: null,
    video_watched: false,
    last_passed_session_id: null,
  };
}

// 第 1 章: 7 レッスンのうち 2 つが準備中。第 2 章: すべて準備中。
const chapter1: Chapter = {
  id: "basics",
  title: "基本",
  description: "基本の説明",
  language: "rust",
  number: 1,
  lessons: [
    lesson("a1", "video_only"),
    lesson("a2", "lesson"),
    lesson("a3", "lesson"),
    lesson("a4", "lesson"),
    lesson("a5", "chapter_review"),
    preparing("a6"),
    preparing("a7"),
  ],
};
const chapter2: Chapter = {
  id: "later",
  title: "あと",
  description: "あとの説明",
  language: "rust",
  number: 2,
  lessons: [preparing("b1"), preparing("b2")],
};
const chapters: ChapterList = { chapters: [chapter1, chapter2] };

function renderList(progress: ProgressSummary) {
  const navigate = vi.fn<(route: Route) => void>();
  render(
    <NavigationContext value={{ route: { name: "lessons" }, navigate }}>
      <LessonList chapters={chapters} progress={progress} />
    </NavigationContext>,
  );
  return navigate;
}

const empty: ProgressSummary = { lessons: {}, recommended_lesson_id: null };

it("準備中のレッスンを除く、どのレッスンも選んで開ける。章末復習レッスンは演習画面を開く", () => {
  const navigate = renderList(empty);
  fireEvent.click(screen.getByRole("link", { name: /タイトル a2/ }));
  expect(navigate).toHaveBeenLastCalledWith({ name: "video", lessonId: "a2" });
  fireEvent.click(screen.getByRole("link", { name: /タイトル a1/ }));
  expect(navigate).toHaveBeenLastCalledWith({ name: "video", lessonId: "a1" });
  fireEvent.click(screen.getByRole("link", { name: /タイトル a5/ }));
  expect(navigate).toHaveBeenLastCalledWith({
    name: "exercise",
    lessonId: "a5",
  });
  expect(navigate).toHaveBeenCalledTimes(3);
});

it("準備中のレッスンは、番号と「準備中」だけを表示し、選んでも開かない", () => {
  const navigate = renderList(empty);
  const row = screen.getByText("1-6").closest("li");
  expect(row).not.toBeNull();
  expect(row?.textContent).toBe("1-6準備中");
  expect(within(row as HTMLElement).queryByRole("link")).toBeNull();
  fireEvent.click(row as HTMLElement);
  fireEvent.click(screen.getByText("1-7"));
  expect(navigate).not.toHaveBeenCalled();
});

it("章の完了数と全体の完了数に、準備中のレッスンを数えない", () => {
  renderList({
    lessons: {
      a1: progressOf("completed"),
      a2: progressOf("completed"),
      a3: progressOf("completed"),
      a4: progressOf("in_progress"),
    },
    recommended_lesson_id: null,
  });
  const section = screen
    .getByRole("heading", { name: /^第1章.基本$/ })
    .closest("section") as HTMLElement;
  expect(section.querySelector(".pg-chapter-count")?.textContent).toBe(
    "3 / 5 完了",
  );
  expect(
    screen.getByText(/レッスン完了。/, { selector: "p" }).textContent,
  ).toBe("3 / 5 レッスン完了。どのレッスンからでも始められます。");
});

it("すべてのレッスンが準備中の章は、完了数の代わりに「準備中」と表示する", () => {
  renderList(empty);
  const section = screen
    .getByRole("heading", { name: /^第2章.あと$/ })
    .closest("section") as HTMLElement;
  expect(section.querySelector(".pg-chapter-count")?.textContent).toBe(
    "準備中",
  );
});

it("レッスンの状態と、推奨レッスンの「次はここ」の印を表示し、推奨レッスンへのボタンを出す", () => {
  const scrollIntoView = vi.fn();
  window.HTMLElement.prototype.scrollIntoView = scrollIntoView;
  renderList({
    lessons: { a1: progressOf("completed"), a2: progressOf("in_progress") },
    recommended_lesson_id: "a2",
  });
  const a2 = screen.getByRole("link", { name: /^1-2 タイトル a2/ });
  expect(a2).toHaveAttribute("aria-current", "true");
  expect(within(a2).getByText("次はここ")).toBeInTheDocument();
  expect(within(a2).getByText("進行中")).toBeInTheDocument();
  expect(
    within(screen.getByRole("link", { name: /タイトル a1/ })).getByText("完了"),
  ).toBeInTheDocument();
  expect(
    within(screen.getByRole("link", { name: /タイトル a3/ })).getByText(
      "未着手",
    ),
  ).toBeInTheDocument();
  expect(screen.getByText("動画のみ")).toBeInTheDocument();
  expect(screen.getByText("復習")).toBeInTheDocument();
  expect(
    screen.getByRole("link", { name: "次はここ1-2 タイトル a2" }),
  ).toBeInTheDocument();
  // 初期表示で、推奨レッスンのある章までスクロールする。
  expect(scrollIntoView).toHaveBeenCalledTimes(1);
});

it("推奨レッスンが無いときは、推奨レッスンへのボタンを出さない", () => {
  renderList(empty);
  expect(screen.queryByText("次はここ")).toBeNull();
});
