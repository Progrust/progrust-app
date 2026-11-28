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

// C の第1章（C1-1〜C1-4 と章末復習）と第2章（すべて準備中）の後に、Rust の第1章と第4章。
const cChapter1: Chapter = {
  id: "c01",
  title: "C の基本",
  description: "C の基本の説明",
  language: "c",
  number: 1,
  lessons: [
    lesson("c01-01", "video_only"),
    lesson("c01-02", "lesson"),
    lesson("c01-03", "lesson"),
    lesson("c01-04", "lesson"),
    lesson("c01-review", "chapter_review"),
  ],
};
const cChapter2: Chapter = {
  id: "c02",
  title: "ポインタ",
  description: "ポインタの説明",
  language: "c",
  number: 2,
  lessons: [preparing("c02-01")],
};
const rustChapter1: Chapter = {
  id: "ch01",
  title: "はじめに",
  description: "はじめにの説明",
  language: "rust",
  number: 1,
  lessons: [
    lesson("ch01-01", "lesson"),
    lesson("ch01-review", "chapter_review"),
  ],
};
const rustChapter4: Chapter = {
  id: "ch04",
  title: "所有権",
  description: "所有権の説明",
  language: "rust",
  number: 4,
  lessons: [
    lesson("ch04-01", "video_only"),
    lesson("ch04-02", "lesson"),
    lesson("ch04-03", "lesson"),
    lesson("ch04-04", "lesson"),
  ],
};
const mixed: ChapterList = {
  chapters: [cChapter1, cChapter2, rustChapter1, rustChapter4],
};

function renderMixed(progress: ProgressSummary, list: ChapterList = mixed) {
  const navigate = vi.fn<(route: Route) => void>();
  render(
    <NavigationContext value={{ route: { name: "lessons" }, navigate }}>
      <LessonList chapters={list} progress={progress} />
    </NavigationContext>,
  );
  return navigate;
}

it("「C 言語編」の見出しの下に C の章を、その後の「Rust 編」の見出しの下に Rust の章を表示する", () => {
  renderMixed(empty);
  const parts = screen
    .getAllByRole("heading", { name: /編$/ })
    .map((heading) => heading.textContent);
  expect(parts).toEqual(["C 言語編", "Rust 編"]);
  const cPart = screen
    .getByRole("heading", { name: "C 言語編" })
    .closest("section") as HTMLElement;
  const rustPart = screen
    .getByRole("heading", { name: "Rust 編" })
    .closest("section") as HTMLElement;
  const chapterTitles = (part: HTMLElement) =>
    Array.from(part.querySelectorAll(".pg-chapter-head h2")).map(
      (heading) => heading.textContent,
    );
  expect(chapterTitles(cPart)).toEqual([
    "C 第1章　C の基本",
    "C 第2章　ポインタ",
  ]);
  expect(chapterTitles(rustPart)).toEqual(["第1章　はじめに", "第4章　所有権"]);
  // 「C 言語編」が先。
  expect(
    cPart.compareDocumentPosition(rustPart) & Node.DOCUMENT_POSITION_FOLLOWING,
  ).toBeTruthy();
  // 区切りの見出しは、章の色を持ち込まない。
  expect(cPart.closest("[data-chapter]")).toBeNull();
  expect(rustPart.closest("[data-chapter]")).toBeNull();
});

it("C のレッスンの番号は C1-2 の形、Rust のレッスンの番号は 4-4 の形で表示し、章の印と章カラーは章の番号で決まる", () => {
  renderMixed(empty);
  expect(
    screen.getByRole("link", { name: /^C1-2 タイトル c01-02/ }),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("link", { name: /^C1-4 タイトル c01-04/ }),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("link", { name: /^4-4 タイトル ch04-04/ }),
  ).toBeInTheDocument();
  expect(screen.getByText("C2-1").closest("li")?.textContent).toBe(
    "C2-1準備中",
  );
  const sections = Array.from(
    document.querySelectorAll<HTMLElement>(".lessons-chapters section[id]"),
  );
  expect(
    sections.map((section) => [
      section.querySelector(".pg-chapter-head .pg-badge")?.textContent,
      section.getAttribute("data-chapter"),
    ]),
  ).toEqual([
    ["C1", "1"],
    ["C2", "2"],
    ["1", "1"],
    ["4", "4"],
  ]);
  // 同じ番号の C の章と Rust の章も、別の区切りとして指せる。
  expect(new Set(sections.map((section) => section.id)).size).toBe(4);
});

it("章の目次も「C 言語編」「Rust 編」で区切り、押すとその章までスクロールする", () => {
  const scrollIntoView = vi.fn();
  window.HTMLElement.prototype.scrollIntoView = scrollIntoView;
  renderMixed(empty);
  const index = screen.getByRole("navigation", { name: "章" });
  const groups = Array.from(index.querySelectorAll(".lessons-index-part")).map(
    (group) => [
      group.querySelector(".lessons-index-part-title")?.textContent,
      Array.from(group.querySelectorAll(".pg-badge")).map(
        (badge) => badge.textContent,
      ),
    ],
  );
  expect(groups).toEqual([
    ["C 言語編", ["C1", "C2"]],
    ["Rust 編", ["1", "4"]],
  ]);
  // C の第1章と Rust の第1章は、それぞれの章の区切りを指す。
  const cLink = within(index).getByRole("link", { name: /C の基本/ });
  const rustLink = within(index).getByRole("link", { name: /はじめに/ });
  expect(cLink.getAttribute("href")).not.toBe(rustLink.getAttribute("href"));
  fireEvent.click(rustLink);
  const target = scrollIntoView.mock.contexts.at(-1) as HTMLElement;
  expect(target.querySelector("h2")?.textContent).toBe("第1章　はじめに");
  fireEvent.click(cLink);
  const cTarget = scrollIntoView.mock.contexts.at(-1) as HTMLElement;
  expect(cTarget.querySelector("h2")?.textContent).toBe("C 第1章　C の基本");
});

it("推奨レッスンが Rust の第1章にあるとき、C の第1章ではなく Rust の第1章までスクロールする", () => {
  const scrollIntoView = vi.fn();
  window.HTMLElement.prototype.scrollIntoView = scrollIntoView;
  renderMixed({ lessons: {}, recommended_lesson_id: "ch01-01" });
  expect(scrollIntoView).toHaveBeenCalledTimes(1);
  const target = scrollIntoView.mock.contexts[0] as HTMLElement;
  expect(target.querySelector("h2")?.textContent).toBe("第1章　はじめに");
  expect(
    screen.getByRole("link", { name: "次はここ1-1 タイトル ch01-01" }),
  ).toBeInTheDocument();
});

it("C の章のレッスンを 1 つも完了していなくても、Rust のレッスンを選ぶと開く", () => {
  const navigate = renderMixed(empty);
  fireEvent.click(screen.getByRole("link", { name: /タイトル ch04-02/ }));
  expect(navigate).toHaveBeenLastCalledWith({
    name: "video",
    lessonId: "ch04-02",
  });
  fireEvent.click(screen.getByRole("link", { name: /タイトル ch01-review/ }));
  expect(navigate).toHaveBeenLastCalledWith({
    name: "exercise",
    lessonId: "ch01-review",
  });
});

it("C の章が無いときは「C 言語編」の見出しを出さず、Rust の章の番号と章カラーは変わらない", () => {
  renderMixed(empty, { chapters: [rustChapter1, rustChapter4] });
  expect(screen.queryByRole("heading", { name: "C 言語編" })).toBeNull();
  expect(screen.getByRole("heading", { name: "Rust 編" })).toBeInTheDocument();
  const sections = Array.from(
    document.querySelectorAll<HTMLElement>(".lessons-chapters section[id]"),
  );
  expect(
    sections.map((section) => [
      section.querySelector(".pg-chapter-head h2")?.textContent,
      section.getAttribute("data-chapter"),
    ]),
  ).toEqual([
    ["第1章　はじめに", "1"],
    ["第4章　所有権", "4"],
  ]);
  expect(
    screen.getByRole("link", { name: /^4-4 タイトル ch04-04/ }),
  ).toBeInTheDocument();
});
