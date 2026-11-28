import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { App } from "./App";

// jsdom では Monaco を読み込めないので、演習画面のエディタを置き換える。
vi.mock("./editor", () => ({
  CodeEditor: () => null,
  KeyMod: { CtrlCmd: 2048, Shift: 1024 },
  KeyCode: { Enter: 3, KeyT: 50, Period: 89 },
}));

beforeEach(() => {
  mockIPC((command) => {
    if (command === "list_chapters") {
      return { chapters: [] };
    }
    if (command === "get_progress") {
      return { lessons: {}, recommended_lesson_id: null };
    }
    if (command === "get_dashboard") {
      return {
        next_lesson: null,
        lesson_counts: { completed: 0, total: 0 },
        chapters: [],
        analysis: {
          state: "none",
          weak_elements: [],
          review_due_elements: [],
          regressed_elements: [],
          last_4_weeks: [],
          summary_md: null,
        },
      };
    }
    if (command === "get_settings") {
      return {
        schema_version: 2,
        theme: "system",
        editor_font_size: 14,
        video: { volume: 1, muted: false, playback_rate: 1, captions: false },
        models: { chat: "sonnet", background: "opus" },
        tool_paths: { cargo: null, claude: null, clang: null },
        first_launch_completed: true,
      };
    }
    if (command === "get_app_version") {
      return { version: "0.1.0" };
    }
    return undefined;
  });
});

afterEach(() => {
  clearMocks();
});

const MENU_LABELS = ["ホーム", "レッスン", "問題生成", "設定"];

function menu() {
  return within(screen.getByRole("navigation", { name: "メインメニュー" }));
}

it("ヘッダーにロゴとメインメニューを表示し、最初はホームを選ぶ", async () => {
  render(<App />);
  expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent(
    "すべてのレッスンを完了しました",
  );
  const header = screen.getByRole("banner");
  expect(within(header).getByText("Progrust")).toBeInTheDocument();
  expect(
    menu()
      .getAllByRole("link")
      .map((link) => link.textContent),
  ).toEqual(MENU_LABELS);
  expect(menu().getByRole("link", { name: "ホーム" })).toHaveAttribute(
    "aria-current",
    "page",
  );
  expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent(
    "すべてのレッスンを完了しました",
  );
});

it("メニューで各画面に移り、選んでいる項目だけに aria-current を付ける", async () => {
  render(<App />);
  await screen.findByRole("heading", { level: 1 });
  const cases = [
    ["レッスン", "レッスン"],
    ["問題生成", "生成問題一覧"],
    ["設定", "設定"],
    ["ホーム", "すべてのレッスンを完了しました"],
  ] as const;
  for (const [label, title] of cases) {
    fireEvent.click(menu().getByRole("link", { name: label }));
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent(
      title,
    );
    // ヘッダーは、どの画面でも表示される。
    expect(screen.getByRole("banner")).toBeInTheDocument();
    const current = menu()
      .getAllByRole("link")
      .filter((link) => link.getAttribute("aria-current") === "page");
    expect(current.map((link) => link.textContent)).toEqual([label]);
  }
});

it("生成問題一覧は準備中と表示する", async () => {
  render(<App />);
  await screen.findByRole("heading", { level: 1 });
  fireEvent.click(menu().getByRole("link", { name: "問題生成" }));
  expect(screen.getByText("準備中")).toBeInTheDocument();
});
