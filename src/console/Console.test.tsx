import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Console, TRUNCATED_NOTICE, type ConsoleStdin } from "./Console";
import {
  CONSOLE_TAIL_LIMIT,
  EMPTY_CONSOLE_LOG,
  appendInput,
  appendRunOutput,
} from "./consoleLog";

const outputArea = () => screen.getByTestId("console-output");

const stdin = (enabled = true): ConsoleStdin => ({
  enabled,
  onSend: vi.fn(),
  onClose: vi.fn(),
});

describe("出力", () => {
  it("出力の種類と ANSI の色を、デザインシステムのクラスで表示する", () => {
    let log = appendRunOutput(EMPTY_CONSOLE_LOG, {
      request_id: "r1",
      chunks: [
        {
          stream: "stderr",
          text: "\u001b[1m\u001b[91merror\u001b[0m\u001b[1m: x\u001b[0m\n",
        },
        { stream: "stdout", text: "名前: " },
      ],
    });
    log = appendInput(log, "ferris\n");
    log = appendRunOutput(log, {
      request_id: "r1",
      chunks: [{ stream: "system", text: "実行時間の制限を超えました" }],
    });
    render(<Console log={log} />);

    const spans = Array.from(outputArea().querySelectorAll("span"), (span) => [
      span.className,
      span.textContent,
    ]);
    expect(spans).toEqual([
      ["pg-out-stderr ansi-red ansi-bold", "error"],
      ["pg-out-stderr ansi-bold", ": x"],
      ["pg-out-stderr", "\n"],
      ["", "名前: "],
      ["pg-out-input", "ferris\n"],
      ["pg-out-system", "実行時間の制限を超えました"],
    ]);
    expect(outputArea()).toHaveClass("pg-console-out");
    expect(outputArea().textContent).not.toContain("\u001b");
    expect(screen.queryByText(TRUNCATED_NOTICE)).toBeNull();
  });

  it("1 MB を受け取ると、先頭に「古い出力を省略しました」を出し、末尾の出力だけを描く", () => {
    let log = EMPTY_CONSOLE_LOG;
    const lines: string[] = [];
    for (let n = 0; n < 32; n++) {
      const chunks = Array.from({ length: 1024 }, (_, i) => {
        const line = `${n}-${i}`.padEnd(31, ".") + "\n";
        lines.push(line);
        return {
          stream: i % 2 === 0 ? ("stdout" as const) : ("stderr" as const),
          text: line,
        };
      });
      log = appendRunOutput(log, { request_id: "r1", chunks });
    }
    const { rerender } = render(<Console log={log} />);

    const notice = outputArea().firstElementChild!;
    expect(notice).toHaveClass("pg-out-system");
    expect(notice.textContent).toBe(`${TRUNCATED_NOTICE}\n`);
    const shown = outputArea().textContent!.slice(notice.textContent!.length);
    expect(shown).toBe(lines.join("").slice(-CONSOLE_TAIL_LIMIT));
    expect(shown.endsWith("31-1023........................\n")).toBe(true);
    expect(outputArea().querySelectorAll("span").length).toBeLessThanOrEqual(
      CONSOLE_TAIL_LIMIT / 32 + 2,
    );

    // さらに出力が増えても、描く量は増えない。
    log = appendRunOutput(log, {
      request_id: "r1",
      chunks: [{ stream: "stdout", text: "last\n" }],
    });
    rerender(<Console log={log} />);
    expect(outputArea().textContent!.length).toBe(
      TRUNCATED_NOTICE.length + 1 + CONSOLE_TAIL_LIMIT,
    );
    expect(outputArea().textContent!.endsWith("last\n")).toBe(true);
  });

  it("出力が増えたら、一番下まで動かす。上に戻して読んでいる間は動かさない", () => {
    let log = appendRunOutput(EMPTY_CONSOLE_LOG, {
      request_id: "r1",
      chunks: [{ stream: "stdout", text: "a\n" }],
    });
    const { rerender } = render(<Console log={log} />);
    const out = outputArea();
    Object.defineProperty(out, "scrollHeight", {
      configurable: true,
      value: 1000,
    });
    Object.defineProperty(out, "clientHeight", {
      configurable: true,
      value: 200,
    });

    log = appendRunOutput(log, {
      request_id: "r1",
      chunks: [{ stream: "stdout", text: "b\n" }],
    });
    rerender(<Console log={log} />);
    expect(out.scrollTop).toBe(1000);

    out.scrollTop = 100;
    fireEvent.scroll(out);
    log = appendRunOutput(log, {
      request_id: "r1",
      chunks: [{ stream: "stdout", text: "c\n" }],
    });
    rerender(<Console log={log} />);
    expect(out.scrollTop).toBe(100);
  });

  it("見出しの行の右に、渡したもの（ボタンなど）を置く", () => {
    render(
      <Console
        log={EMPTY_CONSOLE_LOG}
        actions={<button type="button">実行</button>}
      />,
    );
    expect(
      screen.getByRole("region", { name: "コンソール" }),
    ).toBeInTheDocument();
    expect(screen.getByText("コンソール")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "実行" }).parentElement,
    ).toHaveClass("pg-console-actions");
  });
});

describe("標準入力の入力欄", () => {
  const field = () => screen.getByRole("textbox", { name: "標準入力" });

  it("Enter で、入力した 1 行に改行を付けて送り、欄を空にする", () => {
    const props = stdin();
    render(<Console log={EMPTY_CONSOLE_LOG} stdin={props} />);
    fireEvent.change(field(), { target: { value: "ferris" } });
    fireEvent.keyDown(field(), { key: "Enter" });
    expect(props.onSend).toHaveBeenCalledWith("ferris\n");
    expect(field()).toHaveValue("");

    // 空の行も送る。
    fireEvent.keyDown(field(), { key: "Enter" });
    expect(props.onSend).toHaveBeenLastCalledWith("\n");
    expect(props.onClose).not.toHaveBeenCalled();
  });

  it("実行・提出のショートカット（Cmd+Enter・Cmd+Shift+Enter）では送らない", () => {
    const props = stdin();
    render(<Console log={EMPTY_CONSOLE_LOG} stdin={props} />);
    fireEvent.change(field(), { target: { value: "ferris" } });
    fireEvent.keyDown(field(), { key: "Enter", metaKey: true });
    fireEvent.keyDown(field(), { key: "Enter", metaKey: true, shiftKey: true });
    expect(props.onSend).not.toHaveBeenCalled();
    expect(field()).toHaveValue("ferris");
  });

  it("日本語の変換中の Enter は送らない", () => {
    const props = stdin();
    render(<Console log={EMPTY_CONSOLE_LOG} stdin={props} />);
    fireEvent.change(field(), { target: { value: "ふぇりす" } });
    fireEvent.keyDown(field(), { key: "Enter", isComposing: true });
    // WKWebView で、変換を確定する Enter が届くときの形。
    fireEvent.keyDown(field(), { key: "Enter", keyCode: 229 });
    expect(props.onSend).not.toHaveBeenCalled();
    expect(field()).toHaveValue("ふぇりす");
  });

  it("Ctrl+D と「入力終了」で閉じる", () => {
    const props = stdin();
    render(<Console log={EMPTY_CONSOLE_LOG} stdin={props} />);
    const notCanceled = fireEvent.keyDown(field(), { key: "d", ctrlKey: true });
    expect(notCanceled).toBe(false);
    expect(props.onClose).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "入力終了" }));
    expect(props.onClose).toHaveBeenCalledTimes(2);

    // Cmd+D と、ただの d では閉じない。
    fireEvent.keyDown(field(), { key: "d", metaKey: true });
    fireEvent.keyDown(field(), { key: "d" });
    expect(props.onClose).toHaveBeenCalledTimes(2);
    expect(props.onSend).not.toHaveBeenCalled();
  });

  it("変換中の Ctrl+D は扱わない", () => {
    const props = stdin();
    render(<Console log={EMPTY_CONSOLE_LOG} stdin={props} />);
    fireEvent.keyDown(field(), { key: "d", ctrlKey: true, isComposing: true });
    expect(props.onClose).not.toHaveBeenCalled();
  });

  it("実行中でないときは無効にし、「実行中だけ入力できます」と出す", () => {
    const props = stdin(false);
    render(<Console log={EMPTY_CONSOLE_LOG} stdin={props} />);
    expect(field()).toBeDisabled();
    expect(field()).toHaveAttribute("placeholder", "実行中だけ入力できます");
    expect(screen.getByRole("button", { name: "入力終了" })).toBeDisabled();
    fireEvent.keyDown(field(), { key: "Enter" });
    fireEvent.keyDown(field(), { key: "d", ctrlKey: true });
    expect(props.onSend).not.toHaveBeenCalled();
    expect(props.onClose).not.toHaveBeenCalled();
  });

  it("無効になったら、書きかけの文字を消す", () => {
    const props = stdin();
    const { rerender } = render(
      <Console log={EMPTY_CONSOLE_LOG} stdin={props} />,
    );
    fireEvent.change(field(), { target: { value: "書きかけ" } });
    expect(field()).not.toHaveAttribute("placeholder");
    rerender(
      <Console log={EMPTY_CONSOLE_LOG} stdin={{ ...props, enabled: false }} />,
    );
    expect(field()).toHaveValue("");
    rerender(<Console log={EMPTY_CONSOLE_LOG} stdin={props} />);
    expect(field()).toBeEnabled();
    expect(field()).toHaveValue("");
  });

  it("入力欄を表示しない指定（テスト実行）では、入力欄を出さない", () => {
    render(<Console log={EMPTY_CONSOLE_LOG} stdin={null} />);
    expect(screen.queryByRole("textbox", { name: "標準入力" })).toBeNull();
    expect(screen.queryByRole("button", { name: "入力終了" })).toBeNull();
  });
});
