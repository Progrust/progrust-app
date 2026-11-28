import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it } from "vitest";
import type { Settings } from "../commands/settings";
import { resetTheme } from "../theme";
import { SettingsScreen } from "./SettingsScreen";

let stored: Settings;
// update_settings に渡された設定。
let saved: Settings[];
// update_settings が invalid_argument で断る理由。null なら保存できる。
let rejectReason: string | null;
// test_claude_connection の戻り値か、reject する値。
let connection: () => unknown;
let connectionCalls: number;
let opened: unknown[];

beforeEach(() => {
  resetTheme();
  saved = [];
  rejectReason = null;
  connectionCalls = 0;
  opened = [];
  connection = () => ({ ok: true, model: "sonnet", error: null });
  stored = {
    schema_version: 2,
    theme: "system",
    editor_font_size: 14,
    video: { volume: 1, muted: false, playback_rate: 1, captions: false },
    models: { chat: "sonnet", background: "opus" },
    tool_paths: { cargo: null, claude: null, clang: null },
    first_launch_completed: true,
  };
  mockIPC(
    (command, args) => {
      switch (command) {
        case "get_settings":
          return stored;
        case "get_app_version":
          return { version: "0.1.0" };
        case "get_environment_status":
          return undefined;
        case "update_settings": {
          if (rejectReason !== null) {
            return Promise.reject({
              code: "invalid_argument",
              message: rejectReason,
              detail: null,
            });
          }
          stored = (args as { settings: Settings }).settings;
          saved.push(stored);
          return stored;
        }
        case "test_claude_connection": {
          connectionCalls += 1;
          const value = connection();
          return value instanceof Promise ? value : Promise.resolve(value);
        }
        case "plugin:opener|open_url":
          opened.push(args);
          return null;
        default:
          return undefined;
      }
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => {
  clearMocks();
  resetTheme();
});

const chat = () => screen.findByRole("combobox", { name: "質疑応答のモデル" });
const background = () =>
  screen.findByRole("combobox", { name: "記録・分析・問題生成のモデル" });

it("「AI」の区分が、「動画」と「環境」の間にある", async () => {
  render(<SettingsScreen />);
  await chat();
  const headings = screen
    .getAllByRole("heading", { level: 2 })
    .map((heading) => heading.textContent);
  expect(headings.indexOf("AI")).toBe(headings.indexOf("動画") + 1);
  expect(headings.indexOf("環境")).toBe(headings.indexOf("AI") + 1);
});

it("選択欄に、保存したモデルが出る。選択肢は 3 つのエイリアスと「その他」", async () => {
  render(<SettingsScreen />);
  const select = await chat();
  expect(select).toHaveValue("sonnet");
  expect(await background()).toHaveValue("opus");
  expect(
    Array.from(select.querySelectorAll("option")).map((o) => o.textContent),
  ).toEqual(["opus", "sonnet", "haiku", "その他（モデル ID を入力）"]);
});

it("選ぶと、変えた後の設定が update_settings に渡る", async () => {
  render(<SettingsScreen />);
  fireEvent.change(await chat(), { target: { value: "haiku" } });
  await waitFor(() => {
    expect(saved).toHaveLength(1);
  });
  expect(saved[0].models).toEqual({ chat: "haiku", background: "opus" });
  fireEvent.change(await background(), { target: { value: "sonnet" } });
  await waitFor(() => {
    expect(saved).toHaveLength(2);
  });
  expect(saved[1].models).toEqual({ chat: "haiku", background: "sonnet" });
});

it("「その他」では入力欄が出て、入れたモデル ID が保存される", async () => {
  render(<SettingsScreen />);
  const select = await chat();
  expect(screen.queryByRole("textbox", { name: /モデル ID/ })).toBeNull();
  fireEvent.change(select, { target: { value: "other" } });
  // 入力するまでは保存しない。
  expect(saved).toHaveLength(0);
  const input = screen.getByRole("textbox", {
    name: "質疑応答のモデルのモデル ID",
  });
  fireEvent.change(input, { target: { value: " claude-opus-4-1 " } });
  fireEvent.blur(input);
  await waitFor(() => {
    expect(saved).toHaveLength(1);
  });
  expect(saved[0].models.chat).toBe("claude-opus-4-1");
});

it("保存済みの値がエイリアスでないときは、「その他」と、入力欄にその値が出る", async () => {
  stored.models.background = "claude-haiku-5-5";
  render(<SettingsScreen />);
  expect(await background()).toHaveValue("other");
  expect(
    screen.getByRole("textbox", {
      name: "記録・分析・問題生成のモデルのモデル ID",
    }),
  ).toHaveValue("claude-haiku-5-5");
});

it("保存できなかったとき（invalid_argument）は、理由を出し、保存した値に戻す", async () => {
  rejectReason = "models.chat は、`-` で始まらないものにしてください";
  render(<SettingsScreen />);
  const select = await chat();
  fireEvent.change(select, { target: { value: "other" } });
  const input = screen.getByRole("textbox", {
    name: "質疑応答のモデルのモデル ID",
  });
  fireEvent.change(input, { target: { value: "--bare" } });
  fireEvent.blur(input);
  expect(await screen.findByRole("alert")).toHaveTextContent(rejectReason);
  // 保存した値（sonnet）に戻る。
  expect(select).toHaveValue("sonnet");
  expect(screen.queryByRole("textbox", { name: /モデル ID/ })).toBeNull();
  // 次の保存は、保存できなかった値を含まない。
  rejectReason = null;
  fireEvent.change(await background(), { target: { value: "haiku" } });
  await waitFor(() => {
    expect(saved).toHaveLength(1);
  });
  expect(saved[0].models).toEqual({ chat: "sonnet", background: "haiku" });
});

it("接続テストが成功すると、使ったモデルを出す。テスト中はボタンを押せない", async () => {
  let finish: (value: unknown) => void = () => undefined;
  connection = () =>
    new Promise((resolve) => {
      finish = resolve;
    });
  render(<SettingsScreen />);
  await chat();
  const button = screen.getByRole("button", { name: "接続テスト" });
  fireEvent.click(button);
  expect(await screen.findByText("テスト中です")).toBeInTheDocument();
  expect(button).toBeDisabled();
  finish({ ok: true, model: "haiku", error: null });
  expect(
    await screen.findByText("応答がありました（haiku）"),
  ).toBeInTheDocument();
  expect(button).toBeEnabled();
  expect(connectionCalls).toBe(1);
});

it.each([
  [
    "not_logged_in",
    null,
    "ターミナルで `claude` を起動してログインしてください",
  ],
  [
    "rate_limited",
    null,
    "Claude の利用上限に達しました。時間をおいて再試行してください",
  ],
  ["timeout", null, "応答がありませんでした"],
])(
  "接続テストの失敗（%s）は、決まった文を出す",
  async (category, detail, message) => {
    connection = () => ({
      ok: false,
      model: "sonnet",
      error: { category, detail },
    });
    render(<SettingsScreen />);
    await chat();
    fireEvent.click(screen.getByRole("button", { name: "接続テスト" }));
    expect(await screen.findByText(message)).toBeInTheDocument();
    expect(screen.queryByText("詳細")).toBeNull();
  },
);

it("接続テストの失敗（その他）は、固定の文と、折りたたんだ詳細を出す", async () => {
  connection = () => ({
    ok: false,
    model: "no-such-model",
    error: { category: "other", detail: "There's an issue with the model" },
  });
  render(<SettingsScreen />);
  await chat();
  fireEvent.click(screen.getByRole("button", { name: "接続テスト" }));
  expect(
    await screen.findByText("AI の呼び出しに失敗しました"),
  ).toBeInTheDocument();
  const summary = screen.getByText("詳細");
  expect(summary.closest("details")).not.toHaveAttribute("open");
  expect(
    screen.getByText("There's an issue with the model"),
  ).toBeInTheDocument();
});

it("claude が見つからないときは、「Claude Code が見つかりません」と導入手順へのリンクを出す", async () => {
  connection = () =>
    Promise.reject({
      code: "claude_unavailable",
      message: "Claude Code が見つかりません。",
      detail: null,
    });
  render(<SettingsScreen />);
  await chat();
  fireEvent.click(screen.getByRole("button", { name: "接続テスト" }));
  expect(
    await screen.findByText("Claude Code が見つかりません"),
  ).toBeInTheDocument();
  const guides = screen.getAllByRole("button", { name: "導入手順を見る" });
  fireEvent.click(guides[guides.length - 1]);
  await waitFor(() => {
    expect(opened).toEqual([
      { url: "https://code.claude.com/docs/ja/setup", with: undefined },
    ]);
  });
});
