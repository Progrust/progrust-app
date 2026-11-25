import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { App } from "../App";
import type { EnvironmentStatus } from "../commands/environment";
import type { Settings } from "../commands/settings";

vi.mock("../editor", () => ({
  CodeEditor: () => null,
  KeyMod: { CtrlCmd: 2048, Shift: 1024 },
  KeyCode: { Enter: 3, KeyT: 50, Period: 89 },
}));

afterEach(() => {
  clearMocks();
});

function settings(firstLaunchCompleted: boolean): Settings {
  return {
    schema_version: 1,
    theme: "system",
    editor_font_size: 14,
    video: { volume: 1, muted: false, playback_rate: 1, captions: false },
    models: { chat: "sonnet", background: "opus" },
    tool_paths: { cargo: null, claude: null },
    first_launch_completed: firstLaunchCompleted,
  };
}

const FOUND: EnvironmentStatus = {
  cargo: {
    status: "found",
    cargo_path: "/c/cargo",
    rustc_path: "/c/rustc",
    rustc_version: "1.100.0",
    required_rustc_version: "1.100.0",
  },
  claude: { status: "found", path: "/c/claude", version: "2.0.1" },
  crates: { status: "ready", prepared_at: null, error: null },
};

type Call = { command: string; args: unknown };

// 画面ごとの命令に答える IPC の代わり。呼ばれた命令を calls に記録する。
function mock(first: boolean, env: EnvironmentStatus, calls: Call[] = []) {
  mockIPC((command, args) => {
    calls.push({ command, args });
    switch (command) {
      case "get_settings":
        return settings(first);
      case "update_settings":
        return (args as { settings: Settings }).settings;
      case "get_environment_status":
      case "redetect_tools":
        return env;
      case "get_dashboard":
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
      case "list_chapters":
        return { chapters: [] };
      default:
        return null;
    }
  });
  return calls;
}

// 環境の状態が届くまで待つ（届くまでは「確認しています。」と表示する）。
async function loaded() {
  await screen.findByText("Rust ツールチェーン");
  await waitFor(() => {
    expect(screen.queryByText("確認しています。", { exact: false })).toBeNull();
  });
}

const ONBOARDING = "Progrust で Rust を学ぶ";

it("first_launch_completed が false なら初回起動画面を表示し、ロゴだけのヘッダーを出す", async () => {
  mock(false, FOUND);
  render(<App />);
  expect(
    await screen.findByRole("heading", { level: 1, name: ONBOARDING }),
  ).toBeInTheDocument();
  const header = screen.getByRole("banner");
  expect(within(header).getByText("Progrust")).toBeInTheDocument();
  expect(screen.queryByRole("navigation")).toBeNull();
  expect(screen.getByText(/Anthropic に送信されます/)).toBeInTheDocument();
});

it("first_launch_completed が true ならトップページを表示する", async () => {
  mock(true, FOUND);
  render(<App />);
  expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent(
    "すべてのレッスンを完了しました",
  );
  expect(screen.queryByText(ONBOARDING)).toBeNull();
});

it("cargo と claude が見つかったときは、バージョンとパスと「使えます」を表示する", async () => {
  mock(false, FOUND);
  render(<App />);
  await loaded();
  const rust = screen.getByText("Rust ツールチェーン").closest("li");
  expect(rust).toHaveTextContent("rustc 1.100.0");
  expect(rust).toHaveTextContent("/c/cargo");
  expect(rust).toHaveTextContent("使えます");
  const claude = screen
    .getByText("Claude Code", { selector: "p" })
    .closest("li");
  expect(claude).toHaveTextContent("2.0.1");
  expect(claude).toHaveTextContent("/c/claude");
  expect(claude).toHaveTextContent("使えます");
  expect(
    screen.getByText("外部クレートの準備").closest("li"),
  ).toHaveTextContent("準備できました");
});

it("見つからないときは導入手順のボタンを出し、押すと導入手順を開く", async () => {
  const calls = mock(false, {
    cargo: {
      status: "not_found",
      cargo_path: null,
      rustc_path: null,
      rustc_version: null,
      required_rustc_version: "1.100.0",
    },
    claude: { status: "not_found", path: null, version: null },
    crates: { status: "not_started", prepared_at: null, error: null },
  });
  render(<App />);
  await loaded();
  const buttons = screen.getAllByRole("button", { name: /導入手順を見る/ });
  expect(buttons).toHaveLength(2);
  expect(
    screen.getByText("外部クレートの準備").closest("li"),
  ).toHaveTextContent("準備していません");
  await act(async () => {
    fireEvent.click(buttons[1]);
  });
  const opened = calls.find(
    (call) => call.command === "plugin:opener|open_url",
  );
  expect(opened?.args).toMatchObject({
    url: "https://code.claude.com/docs/ja/setup",
  });
});

it("rustc が古いときは、今のバージョンと必要なバージョンと rustup update stable を示す", async () => {
  mock(false, {
    ...FOUND,
    cargo: {
      ...FOUND.cargo,
      status: "version_too_old",
      rustc_version: "1.99.0",
    },
  });
  render(<App />);
  await loaded();
  const rust = screen.getByText("Rust ツールチェーン").closest("li");
  expect(rust).toHaveTextContent("1.99.0");
  expect(rust).toHaveTextContent("1.100.0 以上");
  expect(rust).toHaveTextContent("rustup update stable");
});

it("外部クレートが準備中のときは「準備中」を表示する", async () => {
  mock(false, {
    ...FOUND,
    crates: { status: "preparing", prepared_at: null, error: null },
  });
  render(<App />);
  await loaded();
  const row = screen.getByText("外部クレートの準備").closest("li");
  expect(row).toHaveTextContent("準備中");
  expect(within(row as HTMLElement).queryByRole("button")).toBeNull();
});

it("外部クレートの準備に失敗したときは再試行のボタンを出し、押すと retry_crate_preparation を呼ぶ", async () => {
  const calls = mock(false, {
    ...FOUND,
    crates: { status: "failed", prepared_at: null, error: "network" },
  });
  render(<App />);
  await loaded();
  const retry = screen.getByRole("button", { name: "再試行" });
  await act(async () => {
    fireEvent.click(retry);
  });
  expect(calls.map((call) => call.command)).toContain(
    "retry_crate_preparation",
  );
});

it("「再検出」を押すと redetect_tools を呼ぶ", async () => {
  const calls = mock(false, FOUND);
  render(<App />);
  await loaded();
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: "再検出" }));
  });
  expect(calls.map((call) => call.command)).toContain("redetect_tools");
});

it("「始める」で first_launch_completed を true にして保存し、トップページに移る。ツールが見つからなくても、準備中でも進める", async () => {
  const calls = mock(false, {
    cargo: {
      status: "not_found",
      cargo_path: null,
      rustc_path: null,
      rustc_version: null,
      required_rustc_version: "1.100.0",
    },
    claude: { status: "not_found", path: null, version: null },
    crates: { status: "preparing", prepared_at: null, error: null },
  });
  render(<App />);
  await loaded();
  fireEvent.click(screen.getByRole("button", { name: /始める/ }));
  expect(
    await screen.findByRole("heading", {
      level: 1,
      name: "すべてのレッスンを完了しました",
    }),
  ).toBeInTheDocument();
  const update = calls.find((call) => call.command === "update_settings");
  expect(
    (update?.args as { settings: Settings }).settings.first_launch_completed,
  ).toBe(true);
  expect(screen.queryByText(ONBOARDING)).toBeNull();
  expect(
    screen.getByRole("navigation", { name: "メインメニュー" }),
  ).toBeInTheDocument();
});

it("保存に失敗したときは、初回起動画面に残って知らせる", async () => {
  mockIPC((command) => {
    if (command === "get_settings") {
      return settings(false);
    }
    if (command === "update_settings") {
      throw new Error("save failed");
    }
    if (command === "get_environment_status") {
      return FOUND;
    }
    return null;
  });
  vi.spyOn(console, "error").mockImplementation(() => {});
  render(<App />);
  await loaded();
  fireEvent.click(screen.getByRole("button", { name: /始める/ }));
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "保存できませんでした",
  );
  expect(
    screen.getByRole("heading", { level: 1, name: ONBOARDING }),
  ).toBeInTheDocument();
});
