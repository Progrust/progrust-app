import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { Chapter, ChapterList } from "../commands/content";
import type { ChapterCount, Dashboard } from "../commands/learningRecord";
import { NavigationContext } from "../router";
import type { Route } from "../router";
import { Home } from "./HomeScreen";

function lesson(id: string, kind: "lesson" | "chapter_review" = "lesson") {
  return {
    id,
    kind,
    title: `タイトル ${id}`,
    summary: `概要 ${id}`,
    estimated_minutes: 15,
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

// 第 1〜11 章。第 11 章だけ、すべて準備中。
const chapterList: ChapterList = {
  chapters: Array.from({ length: 11 }, (_, i): Chapter => {
    const n = i + 1;
    return {
      id: `ch${n}`,
      title: `章${n}`,
      description: "",
      language: "rust",
      number: n,
      lessons:
        n === 11
          ? [preparing("p1"), preparing("p2")]
          : [lesson(`l${n}-1`), lesson(`l${n}-2`, "chapter_review")],
    };
  }),
};

function counts(from: number, to: number): ChapterCount[] {
  return chapterList.chapters.slice(from - 1, to).map((chapter) => ({
    id: chapter.id,
    title: chapter.title,
    language: chapter.language,
    number: chapter.number,
    completed: chapter.id === "ch11" ? 0 : 1,
    total: chapter.id === "ch11" ? 0 : 2,
  }));
}

const noAnalysis: Dashboard["analysis"] = {
  state: "none",
  weak_elements: [],
  review_due_elements: [],
  regressed_elements: [],
  last_4_weeks: [],
  summary_md: null,
};

function renderHome(dashboard: Dashboard) {
  const navigate = vi.fn<(route: Route) => void>();
  render(
    <NavigationContext value={{ route: { name: "home" }, navigate }}>
      <Home dashboard={dashboard} chapters={chapterList} />
    </NavigationContext>,
  );
  return navigate;
}

const dashboard: Dashboard = {
  next_lesson: {
    lesson_id: "l4-1",
    chapter_id: "ch4",
    chapter_title: "章4",
    language: "rust",
    chapter_number: 4,
    title: "タイトル l4-1",
    summary: "概要 l4-1。",
    started: true,
  },
  lesson_counts: { completed: 7, total: 20 },
  chapters: counts(2, 10),
  analysis: noAnalysis,
};

it("学習記録がない状態でも、エラーにならず、分析の欄に案内を表示する", () => {
  renderHome(dashboard);
  expect(screen.queryByRole("alert")).toBeNull();
  const panels = ["総評", "学習の分析"].map((name) =>
    screen.getByRole("region", { name }),
  );
  for (const panel of panels) {
    expect(
      within(panel).getByText("演習に正解すると、ここに分析が表示されます"),
    ).toBeInTheDocument();
  }
});

it("次にやるべきレッスンの欄に、章・番号・名前・概要・目安の時間を表示し、動画画面を開く", () => {
  const navigate = renderHome(dashboard);
  const hero = screen.getByRole("region", { name: "次のレッスン" });
  expect(hero).toHaveAttribute("data-chapter", "4");
  expect(hero.querySelector(".pg-emblem")?.textContent).toBe("第4章4-1");
  expect(hero.querySelector(".pg-hero-eyebrow")?.textContent).toBe(
    "次のレッスン　章4",
  );
  expect(
    within(hero).getByRole("heading", { name: "タイトル l4-1" }),
  ).toBeInTheDocument();
  expect(within(hero).getByText("概要 l4-1。目安 15 分。")).toBeInTheDocument();
  fireEvent.click(within(hero).getByRole("button", { name: "続きから" }));
  expect(navigate).toHaveBeenLastCalledWith({
    name: "video",
    lessonId: "l4-1",
  });
});

it("未着手なら「始める」を表示し、章末復習レッスンは演習画面を開く", () => {
  const navigate = renderHome({
    ...dashboard,
    next_lesson: {
      lesson_id: "l4-2",
      chapter_id: "ch4",
      chapter_title: "章4",
      language: "rust",
      chapter_number: 4,
      title: "復習",
      summary: "",
      started: false,
    },
  });
  fireEvent.click(screen.getByRole("button", { name: "始める" }));
  expect(navigate).toHaveBeenLastCalledWith({
    name: "exercise",
    lessonId: "l4-2",
  });
});

it("次にやるべきレッスンが第4章にあるとき、章ごとの完了率に第2〜10章の 9 章だけを表示する", () => {
  renderHome(dashboard);
  const tiles = within(
    screen.getByRole("navigation", { name: "章の一覧" }),
  ).getAllByRole("link");
  const chapterTiles = tiles.filter(
    (tile) => !tile.classList.contains("pg-tile--more"),
  );
  expect(
    chapterTiles.map((tile) => tile.querySelector(".pg-badge")?.textContent),
  ).toEqual(["2", "3", "4", "5", "6", "7", "8", "9", "10"]);
  const current = chapterTiles.filter(
    (tile) => tile.getAttribute("aria-current") === "true",
  );
  expect(current).toHaveLength(1);
  expect(current[0]).toHaveTextContent("学習中");
  expect(current[0]?.querySelector(".pg-badge")?.textContent).toBe("4");
  expect(chapterTiles[0]?.querySelector(".pg-tile-count")?.textContent).toBe(
    "1/2",
  );
  expect(
    screen.getByText("レッスン完了", { exact: false, selector: "span" })
      .textContent,
  ).toBe("7 / 20 レッスン完了");
});

it("準備中のレッスンしかない章は、完了率の代わりに「準備中」と表示する", () => {
  renderHome({
    ...dashboard,
    next_lesson: { ...dashboard.next_lesson!, chapter_id: "ch10" },
    chapters: counts(3, 11),
  });
  const tile = screen
    .getAllByRole("link")
    .find((link) => link.querySelector(".pg-badge")?.textContent === "11");
  expect(tile?.querySelector(".pg-tile-count")?.textContent).toBe("準備中");
});

it("章のタイルと「レッスン一覧を開く」で、レッスン一覧へ移る", () => {
  const navigate = renderHome(dashboard);
  fireEvent.click(screen.getByRole("link", { name: /章5/ }));
  expect(navigate).toHaveBeenLastCalledWith({ name: "lessons" });
  fireEvent.click(screen.getByRole("link", { name: "レッスン一覧を開く" }));
  expect(navigate).toHaveBeenCalledTimes(2);
  expect(navigate).toHaveBeenLastCalledWith({ name: "lessons" });
});

it("次にやるべきレッスンが無いときは、章の色を使わない欄と「問題を生成する」を表示する", () => {
  const navigate = renderHome({
    ...dashboard,
    next_lesson: null,
    lesson_counts: { completed: 20, total: 20 },
    chapters: counts(3, 11),
  });
  const hero = screen.getByRole("region", { name: "次にやること" });
  expect(hero).not.toHaveAttribute("data-chapter");
  expect(
    within(hero).getByRole("heading", {
      name: "すべてのレッスンを完了しました",
    }),
  ).toBeInTheDocument();
  fireEvent.click(within(hero).getByRole("button", { name: "問題を生成する" }));
  expect(navigate).toHaveBeenLastCalledWith({ name: "generated" });
});

afterEach(() => {
  clearMocks();
});

type CargoState = "found" | "not_found" | "version_too_old";
type ClaudeState = "found" | "not_found";
type CratesState = "not_started" | "preparing" | "ready" | "failed";

// 環境の状態を返すようにする。開いた導入手順と、呼ばれたコマンドを返す。
function mockEnvironment(
  cargo: CargoState,
  claude: ClaudeState = "found",
  crates: CratesState = "ready",
) {
  const opened: unknown[] = [];
  const calls: string[] = [];
  mockIPC(
    (command, args) => {
      calls.push(command);
      if (command === "plugin:opener|open_url") {
        opened.push(args);
      }
      return command === "get_environment_status"
        ? {
            cargo: {
              status: cargo,
              cargo_path: null,
              rustc_path: null,
              rustc_version: cargo === "version_too_old" ? "1.99.0" : null,
              required_rustc_version: "1.100.0",
            },
            claude: { status: claude, path: null, version: null },
            crates: { status: crates, prepared_at: null, error: null },
          }
        : null;
    },
    { shouldMockEvents: true },
  );
  return { opened, calls };
}

it("cargo が見つからないときは、rustup の導入手順を案内する（ほかが正常なら、ほかの案内は出さない）", async () => {
  const { opened } = mockEnvironment("not_found");
  renderHome(dashboard);
  const alert = await screen.findByRole("alert");
  expect(alert).toHaveTextContent("Rust ツールチェーンが見つかりません");
  fireEvent.click(
    within(alert).getByRole("button", { name: "rustup の導入手順" }),
  );
  await waitFor(() => {
    expect(opened).toEqual([{ url: "https://rustup.rs", with: undefined }]);
  });
  expect(screen.getAllByRole("alert")).toHaveLength(1);
  expect(screen.queryByText(/Claude Code/)).toBeNull();
  expect(screen.queryByText(/外部クレート/)).toBeNull();
});

it("rustc が古いときは、今のバージョンと必要なバージョンと更新の手順を案内する", async () => {
  mockEnvironment("version_too_old");
  renderHome(dashboard);
  const alert = await screen.findByRole("alert");
  expect(alert).toHaveTextContent("1.99.0");
  expect(alert).toHaveTextContent("1.100.0");
  expect(alert).toHaveTextContent("rustup update stable");
});

it("すべて見つかって準備ができているときは、案内を出さない", async () => {
  mockEnvironment("found");
  renderHome(dashboard);
  await screen.findByRole("region", { name: "次のレッスン" });
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(screen.queryByRole("alert")).toBeNull();
  expect(screen.queryByRole("status")).toBeNull();
});

it("初回準備が not_started のときは、案内を出さない", async () => {
  mockEnvironment("found", "found", "not_started");
  renderHome(dashboard);
  await screen.findByRole("region", { name: "次のレッスン" });
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(screen.queryByText(/外部クレート/)).toBeNull();
});

it("初回準備中は「外部クレートを準備しています」を表示する", async () => {
  mockEnvironment("found", "found", "preparing");
  renderHome(dashboard);
  expect(
    await screen.findByText("外部クレートを準備しています。"),
  ).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "再試行" })).toBeNull();
});

it("初回準備に失敗したときは、失敗を表示し、再試行のボタンで retry_crate_preparation を呼ぶ", async () => {
  const { calls } = mockEnvironment("found", "found", "failed");
  renderHome(dashboard);
  expect(
    await screen.findByText("外部クレートの準備に失敗しました。"),
  ).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "再試行" }));
  await waitFor(() => {
    expect(calls).toContain("retry_crate_preparation");
  });
});

it("claude が見つからないときは、導入手順とパスの指定を案内し、パスの指定で設定画面へ移る", async () => {
  const { opened } = mockEnvironment("found", "not_found");
  const navigate = renderHome(dashboard);
  const alert = await screen.findByRole("alert");
  expect(alert).toHaveTextContent("Claude Code が見つかりません");
  fireEvent.click(
    within(alert).getByRole("button", { name: /導入手順を見る/ }),
  );
  await waitFor(() => {
    expect(opened).toEqual([
      { url: "https://code.claude.com/docs/ja/setup", with: undefined },
    ]);
  });
  fireEvent.click(within(alert).getByRole("button", { name: "パスを指定" }));
  expect(navigate).toHaveBeenCalledWith({ name: "settings" });
});
