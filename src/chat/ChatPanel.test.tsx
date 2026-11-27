import { act, fireEvent, render, screen } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, expect, it, vi } from "vitest";

import type { ClaudeStatus } from "../commands/environment";
import { ChatPanel } from "./ChatPanel";
import type { ChatExchange, UseChat } from "./useChat";

afterEach(() => {
  clearMocks();
});

const FOUND: ClaudeStatus = { status: "found", path: "/c", version: "1" };
const NOT_FOUND: ClaudeStatus = {
  status: "not_found",
  path: null,
  version: null,
};

function exchange(change: Partial<ChatExchange> = {}): ChatExchange {
  return {
    requestId: "r",
    question: "質問",
    answer: "",
    status: "answering",
    error: null,
    sentenceId: null,
    ...change,
  };
}

function makeChat(exchanges: ChatExchange[], change: Partial<UseChat> = {}) {
  return {
    exchanges,
    answering: exchanges.some((e) => e.status === "answering"),
    send: vi.fn(() => true),
    stop: vi.fn(),
    retry: vi.fn(() => true),
    ...change,
  } satisfies UseChat;
}

function renderPanel(chat: UseChat, claude: ClaudeStatus | null = FOUND) {
  const view = render(
    <ChatPanel
      chat={chat}
      title="質問する"
      placeholder="この動画について質問"
      claude={claude}
    />,
  );
  return {
    ...view,
    rerenderWith: (next: UseChat) =>
      view.rerender(
        <ChatPanel
          chat={next}
          title="質問する"
          placeholder="この動画について質問"
          claude={claude}
        />,
      ),
  };
}

it("回答の断片が届くたびに伸び、Markdown と Rust の色分けが付く", () => {
  const first = "**太字**です。\n\n```rust\nfn main() {";
  const view = renderPanel(makeChat([exchange({ answer: first })]));
  expect(screen.getByText("太字").tagName).toBe("STRONG");
  // 閉じていないコードブロックも、崩れずに色分けする
  expect(document.querySelector(".pg-code .tk-k")).toHaveTextContent("fn");
  view.rerenderWith(
    makeChat([
      exchange({
        answer: `${first}\n    println!("hi");\n}\n\`\`\`\n続き`,
      }),
    ]),
  );
  expect(document.querySelector(".pg-code .tk-m")).toHaveTextContent(
    "println!",
  );
  expect(document.querySelector(".pg-code .tk-s")).toHaveTextContent('"hi"');
  expect(screen.getByText("続き")).toBeInTheDocument();
  expect(screen.getByText("回答を作成しています")).toBeInTheDocument();
});

it("回答中は「停止」が出て、押すと停止の操作が呼ばれ、入力欄は出ない", () => {
  const chat = makeChat([exchange()]);
  renderPanel(chat);
  expect(screen.queryByRole("textbox")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: /停止/ }));
  expect(chat.stop).toHaveBeenCalledTimes(1);
});

it("止めた回答は残して「（停止しました）」を出す", () => {
  renderPanel(
    makeChat([exchange({ answer: "途中の回答", status: "stopped" })]),
  );
  expect(screen.getByText("途中の回答")).toBeInTheDocument();
  expect(screen.getByText("（停止しました）")).toBeInTheDocument();
  expect(screen.getByRole("textbox")).toBeInTheDocument();
});

it("AI のエラーは分類ごとの文と「再試行」を出し、押すと再試行の操作が呼ばれる", () => {
  const chat = makeChat([
    exchange({
      status: "error",
      error: { kind: "ai", error: { category: "rate_limited", detail: null } },
    }),
  ]);
  renderPanel(chat);
  expect(screen.getByRole("alert")).toHaveTextContent(
    "Claude の利用上限に達しました。時間をおいて再試行してください",
  );
  fireEvent.click(screen.getByRole("button", { name: "再試行" }));
  expect(chat.retry).toHaveBeenCalledTimes(1);
});

it("send_chat のエラーは、そのエラーの message を出す", () => {
  renderPanel(
    makeChat([
      exchange({
        status: "error",
        error: { kind: "command", code: null, message: "送れませんでした" },
      }),
    ]),
  );
  expect(screen.getByRole("alert")).toHaveTextContent("送れませんでした");
});

it("「その他」のエラーの詳細は折りたたんで出す", () => {
  renderPanel(
    makeChat([
      exchange({
        status: "error",
        error: {
          kind: "ai",
          error: { category: "other", detail: "詳しい説明" },
        },
      }),
    ]),
  );
  expect(screen.getByText("AI の呼び出しに失敗しました")).toBeInTheDocument();
  expect(screen.getByText("詳しい説明").closest("details")).not.toBeNull();
});

it("最後でないエラーの往復には「再試行」を出さない", () => {
  renderPanel(
    makeChat([
      exchange({
        requestId: "a",
        status: "error",
        error: { kind: "ai", error: { category: "timeout", detail: null } },
      }),
      exchange({ requestId: "b", status: "completed", answer: "答え" }),
    ]),
  );
  expect(screen.queryByRole("button", { name: "再試行" })).toBeNull();
});

it("入力欄は 2,000 文字までで、Cmd+Enter で送信し、送信したら空にする", () => {
  const chat = makeChat([]);
  renderPanel(chat);
  const input = screen.getByRole("textbox");
  expect(input).toHaveAttribute("maxlength", "2000");
  fireEvent.change(input, { target: { value: "あ".repeat(2500) } });
  expect((input as HTMLTextAreaElement).value).toHaveLength(2000);
  fireEvent.change(input, { target: { value: "行1\n行2" } });
  // Enter だけでは送信しない（改行）
  fireEvent.keyDown(input, { key: "Enter" });
  expect(chat.send).not.toHaveBeenCalled();
  fireEvent.keyDown(input, { key: "Enter", metaKey: true });
  expect(chat.send).toHaveBeenCalledWith("行1\n行2");
  expect((input as HTMLTextAreaElement).value).toBe("");
});

it("日本語の変換中の Cmd+Enter では送信しない", () => {
  const chat = makeChat([]);
  renderPanel(chat);
  const input = screen.getByRole("textbox");
  fireEvent.change(input, { target: { value: "しつもん" } });
  fireEvent.keyDown(input, { key: "Enter", metaKey: true, isComposing: true });
  fireEvent.keyDown(input, { key: "Enter", metaKey: true, keyCode: 229 });
  expect(chat.send).not.toHaveBeenCalled();
});

it("空白だけの質問は送信しない", () => {
  const chat = makeChat([]);
  renderPanel(chat);
  const input = screen.getByRole("textbox");
  fireEvent.change(input, { target: { value: " \n " } });
  fireEvent.keyDown(input, { key: "Enter", metaKey: true });
  expect(chat.send).not.toHaveBeenCalled();
  expect(screen.getByRole("button", { name: "送信" })).toBeDisabled();
});

it("送信ボタンでも送信できる", () => {
  const chat = makeChat([]);
  renderPanel(chat);
  fireEvent.change(screen.getByRole("textbox"), { target: { value: "質問" } });
  fireEvent.click(screen.getByRole("button", { name: "送信" }));
  expect(chat.send).toHaveBeenCalledWith("質問");
});

it("claude が見つからないときは入力欄を出さず、導入手順を開ける", async () => {
  const opened: unknown[] = [];
  mockIPC((command, args) => {
    if (command === "plugin:opener|open_url") {
      opened.push(args);
    }
    return null;
  });
  renderPanel(makeChat([]), NOT_FOUND);
  expect(screen.queryByRole("textbox")).toBeNull();
  expect(screen.getByRole("note")).toHaveTextContent(
    "Claude Code が見つかりません",
  );
  await act(async () => {
    fireEvent.click(screen.getByRole("link", { name: "導入手順を見る" }));
  });
  expect(opened).toEqual([
    { url: "https://code.claude.com/docs/ja/setup", with: undefined },
  ]);
});

it("見出し・空の状態・閉じるボタンは置く側が渡す", () => {
  const onClose = vi.fn();
  render(
    <ChatPanel
      chat={makeChat([])}
      title="解説について質問する"
      placeholder="解説について質問"
      empty={{ title: "質問できるようになりました", text: "何でも聞けます。" }}
      claude={null}
      onClose={onClose}
    />,
  );
  expect(screen.getByRole("heading")).toHaveTextContent("解説について質問する");
  expect(screen.getByText("何でも聞けます。")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "質問パネルを閉じる" }));
  expect(onClose).toHaveBeenCalled();
});
