import { describe, expect, it } from "vitest";
import {
  INITIAL_ANSI_STATE,
  PLAIN_STYLE,
  applySgr,
  parseAnsi,
  type AnsiState,
  type StyledText,
} from "./ansi";

const parse = (input: string, state: AnsiState = INITIAL_ANSI_STATE) =>
  parseAnsi(state, input);

// 読んだ結果を、見比べやすい形にする。
const show = (pieces: StyledText[]) =>
  pieces.map(
    ({ style, text }) =>
      `${style.bold ? "b" : ""}${style.color ?? "-"}:${text}`,
  );

describe("コンパイラの出力の色", () => {
  // rustc 1.99 の --message-format=json-diagnostic-rendered-ansi の rendered から抜き出した行と、
  // 同じ書き方の help と追加の行（明るい色の 9x を使う）。
  const rendered =
    "\u001b[1m\u001b[91merror[E0382]\u001b[0m\u001b[1m: borrow of moved value: `name`\u001b[0m\n" +
    " \u001b[1m\u001b[94m--> \u001b[0msrc/main.rs:9:16\n" +
    "\u001b[1m\u001b[96mhelp\u001b[0m: consider cloning\n" +
    "  \u001b[92m+\u001b[0m\n";

  it("明るい色を、テーマの 6 色に寄せ、太字とともに読む", () => {
    const { pieces, state } = parse(rendered);
    expect(show(pieces)).toEqual([
      "bred:error[E0382]",
      "b-:: borrow of moved value: `name`",
      "-:\n ",
      "bblue:--> ",
      "-:src/main.rs:9:16\n",
      "bcyan:help",
      "-:: consider cloning\n  ",
      "green:+",
      "-:\n",
    ]);
    expect(state).toEqual({ style: PLAIN_STYLE, pending: "" });
  });

  it("テスト実行の cargo の行の色を読み、リンクの OSC を除く", () => {
    // CARGO_TERM_COLOR=always の cargo test（cargo 1.99.1）の標準エラー出力。
    const { pieces } = parse(
      "\u001b[1m\u001b[92m    Finished\u001b[0m \u001b]8;;https://doc.rust-lang.org/cargo/reference/profiles.html#default-profiles\u001b\\`test` profile [unoptimized + debuginfo]\u001b]8;;\u001b\\ target(s) in 0.28s\n" +
        "\u001b[1m\u001b[91merror\u001b[0m: test failed, to rerun pass `--bin ansi-probe`\n",
    );
    expect(show(pieces)).toEqual([
      "bgreen:    Finished",
      "-: `test` profile [unoptimized + debuginfo] target(s) in 0.28s\n",
      "bred:error",
      "-:: test failed, to rerun pass `--bin ansi-probe`\n",
    ]);
  });
});

describe("SGR の読み方", () => {
  it.each([
    ["31", "red"],
    ["32", "green"],
    ["33", "yellow"],
    ["34", "blue"],
    ["35", "magenta"],
    ["36", "cyan"],
    ["91", "red"],
    ["95", "magenta"],
    ["38;5;1", "red"],
    ["38;5;9", "red"],
    ["38;5;12", "blue"],
    ["38:5:14", "cyan"],
  ])("%s は %s", (params, color) => {
    expect(applySgr(PLAIN_STYLE, params)).toEqual({ color, bold: false });
  });

  it.each([
    ["30", "黒"],
    ["37", "白"],
    ["90", "明るい黒"],
    ["97", "明るい白"],
    ["38;5;0", "256 色の黒"],
    ["38;5;15", "256 色の明るい白"],
    ["38;5;196", "256 色の 16 番より後"],
    ["38;2;255;0;0", "24 ビットの色"],
    ["38:2::255:0:0", "24 ビットの色（コロン）"],
  ])("%s（%s）は既定の色にする", (params) => {
    const red = { color: "red" as const, bold: true };
    expect(applySgr(red, params)).toEqual({ color: null, bold: true });
  });

  it("拡張の色の引数を、別の指定として読まない", () => {
    // 5 や 2 や 1 を、点滅・薄字・太字として読まない。
    expect(applySgr(PLAIN_STYLE, "38;5;1")).toEqual({
      color: "red",
      bold: false,
    });
    expect(applySgr(PLAIN_STYLE, "38;2;1;1;1;32")).toEqual({
      color: "green",
      bold: false,
    });
    expect(applySgr(PLAIN_STYLE, "48;5;1;34")).toEqual({
      color: "blue",
      bold: false,
    });
    expect(applySgr(PLAIN_STYLE, "48;2;1;1;1")).toEqual(PLAIN_STYLE);
  });

  it("0・空で戻し、22 で太字を、39 で色を戻す", () => {
    const style = { color: "red" as const, bold: true };
    expect(applySgr(style, "0")).toEqual(PLAIN_STYLE);
    expect(applySgr(style, "")).toEqual(PLAIN_STYLE);
    expect(applySgr(style, "22")).toEqual({ color: "red", bold: false });
    expect(applySgr(style, "39")).toEqual({ color: null, bold: true });
    expect(applySgr(PLAIN_STYLE, "1;33")).toEqual({
      color: "yellow",
      bold: true,
    });
  });

  it("背景・下線・斜体などは無視する", () => {
    expect(applySgr(PLAIN_STYLE, "4;3;7;41;101;49")).toEqual(PLAIN_STYLE);
  });
});

describe("読まないエスケープ", () => {
  it.each([
    ["行を消す CSI", "a\u001b[2Kb\u001b[1Gc"],
    ["カーソルを動かす CSI", "a\u001b[?25lb\u001b[10;20Hc"],
    [
      "OSC 8 のリンク（BEL）",
      "a\u001b]8;;https://example.com\u0007b\u001b]8;;\u0007c",
    ],
    [
      "OSC 8 のリンク（ST）",
      "a\u001b]8;;file:///x\u001b\\b\u001b]8;;\u001b\\c",
    ],
    ["文字集合の指定", "a\u001b(Bb\u001b)0c"],
    ["2 文字のエスケープ", "a\u001b7b\u001b8c"],
  ])("%s を表示から除く", (_, input) => {
    expect(show(parse(input).pieces)).toEqual(["-:abc"]);
  });

  it("壊れた CSI は、そこまでを捨て、続きを文字として読む", () => {
    expect(show(parse("a\u001b[12\nb").pieces)).toEqual(["-:a\nb"]);
  });

  it("ESC が続いても、それぞれを読む", () => {
    expect(show(parse("\u001b\u001b[31mx").pieces)).toEqual(["red:x"]);
    expect(show(parse("a\u001b]0;title\u001b[31mx").pieces)).toEqual([
      "-:a",
      "red:x",
    ]);
  });
});

describe("断片の境目", () => {
  it.each([
    [
      "ESC の直後",
      "a\u001b",
      "[1;31mred\u001b[0m b",
      ["-:a", "bred:red", "-: b"],
    ],
    [
      "CSI の引数の途中",
      "a\u001b[1;3",
      "1mred\u001b[0m b",
      ["-:a", "bred:red", "-: b"],
    ],
    [
      "SGR の ; の後",
      "a\u001b[1;",
      "31mred\u001b[0m b",
      ["-:a", "bred:red", "-: b"],
    ],
    ["OSC の途中", "a\u001b]8;;http", "s://x\u0007b", ["-:a", "-:b"]],
    ["OSC の ST の途中", "a\u001b]8;;x\u001b", "\\b", ["-:a", "-:b"]],
  ])(
    "%s で切れたエスケープを、次の断片に持ち越す",
    (_, head, tail, expected) => {
      const first = parse(head);
      expect(first.state.pending).toBe(head.slice(1));
      const second = parse(tail, first.state);
      expect([...show(first.pieces), ...show(second.pieces)]).toEqual(expected);
    },
  );

  it("色を次の断片に持ち越す", () => {
    const first = parse("\u001b[1m\u001b[91merr");
    expect(first.state).toEqual({
      style: { color: "red", bold: true },
      pending: "",
    });
    const second = parse("or\u001b[0m: x", first.state);
    expect(show(second.pieces)).toEqual(["bred:or", "-:: x"]);
  });

  it("持ち越しが長すぎる OSC は、エスケープとみなさず文字として読む", () => {
    const long = "x".repeat(5000);
    const { pieces, state } = parse(`a\u001b]${long}`);
    expect(state.pending).toBe("");
    expect(show(pieces)).toEqual([`-:a]${long}`]);
  });
});

it("ESC が多い 1 MB の出力も、すぐに読み終える", () => {
  const input = "\u001b]".repeat(512 * 1024);
  const started = performance.now();
  parse(input);
  expect(performance.now() - started).toBeLessThan(2000);
});
