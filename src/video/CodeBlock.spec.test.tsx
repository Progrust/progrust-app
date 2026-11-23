import { render } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { CodeBlock, type CodeBlockProps } from "./CodeBlock";
import { CodeBlocksContext, type CodeBlockData } from "./hooks";

const CODE = [
  "fn main() {", // 10
  "    let r;", // 11
  "    {", // 12
  "        let x = 5;", // 13
  "        r = &x;", // 14
  "    }", // 15
  '    println!("{r} {r}");', // 16
  "}", // 17
].join("\n");

const RUST: CodeBlockData = {
  id: "a",
  language: "rust",
  start_line: 10,
  code: CODE,
};

function renderWith(
  props: Omit<CodeBlockProps, "block">,
  data: CodeBlockData = RUST,
) {
  return render(
    <CodeBlocksContext.Provider value={[data]}>
      <CodeBlock block={data.id} {...props} />
    </CodeBlocksContext.Provider>,
  );
}

function expectThrow(
  props: Omit<CodeBlockProps, "block">,
  pattern: RegExp,
  data?: CodeBlockData,
) {
  const spy = vi.spyOn(console, "error").mockImplementation(() => {});
  expect(() => renderWith(props, data)).toThrow(pattern);
  spy.mockRestore();
}

const lns = (c: HTMLElement) =>
  Array.from(c.querySelectorAll(".line")).map((l) => l.getAttribute("data-ln"));

it("scopes は、範囲ごとに枠を差し込み、入れ子の深さを付ける", () => {
  const { container } = renderWith({
    scopes: [
      { label: "r", from: 11, to: 16 },
      { label: "x", from: 13, to: 15, progress: 0.5 },
    ],
  });
  expect(container.querySelector("code")).toHaveAttribute("data-scopes");
  const scopes = container.querySelectorAll<HTMLElement>("span.scope");
  expect(scopes).toHaveLength(2);
  expect(scopes[0]).toHaveAttribute("data-label", "r");
  expect(scopes[0]).toHaveAttribute("data-depth", "0");
  expect(scopes[0].style.getPropertyValue("--from")).toBe("2");
  expect(scopes[0].style.getPropertyValue("--to")).toBe("7");
  expect(scopes[0].style.getPropertyValue("--scope-p")).toBe("");
  expect(scopes[1]).toHaveAttribute("data-depth", "1");
  expect(scopes[1].style.getPropertyValue("--from")).toBe("4");
  expect(scopes[1].style.getPropertyValue("--scope-p")).toBe("0.5");
});

it("範囲の一部だけが重なる scopes は、描画時にエラーになる", () => {
  expectThrow(
    {
      scopes: [
        { label: "a", from: 11, to: 14 },
        { label: "b", from: 13, to: 16 },
      ],
    },
    /一部だけ重なっている/,
  );
});

it("scopes の個数・入れ子・範囲・diff の制約に違反すると、描画時にエラーになる", () => {
  expectThrow(
    {
      scopes: [
        { label: "a", from: 10, to: 11 },
        { label: "b", from: 12, to: 13 },
        { label: "c", from: 14, to: 15 },
        { label: "d", from: 16, to: 17 },
      ],
    },
    /3 つまで/,
  );
  expectThrow(
    {
      scopes: [
        { label: "a", from: 10, to: 17 },
        { label: "b", from: 11, to: 16 },
        { label: "c", from: 12, to: 15 },
      ],
    },
    /2 段まで/,
  );
  expectThrow({ scopes: [{ label: "a", from: 9, to: 12 }] }, /の外にある/);
  expectThrow({ scopes: [{ label: "a", from: 11, to: 18 }] }, /の外にある/);
  expectThrow(
    { scopes: [{ label: "a", from: 1, to: 2 }] },
    /diff のコードブロックでは scopes/,
    { id: "a", language: "rust", diff: true, code: " a\n+b" },
  );
});

it("marks は字句を span.mark で囲み、nth で何個目かを指し、進み具合を渡す", () => {
  const { container } = renderWith({
    marks: [
      { line: 16, text: "r", nth: 3 },
      { line: 14, text: "&x", progress: 0.5 },
    ],
  });
  const marks = container.querySelectorAll<HTMLElement>("span.mark");
  expect(marks).toHaveLength(2);
  expect(marks[0].textContent).toBe("&x");
  expect(marks[0].style.getPropertyValue("--mark-p")).toBe("0.5");
  expect(marks[1].textContent).toBe("r");
  expect(marks[1].style.getPropertyValue("--mark-p")).toBe("");
  // 3 個目の r は、println! の r、1 つ目の {r} の r、2 つ目の {r} の r の順で、最後のもの
  const line = marks[1].closest(".line") as HTMLElement;
  expect(line.textContent).toBe('    println!("{r} {r}");');
  const before = document.createRange();
  before.setStart(line, 0);
  before.setEndBefore(marks[1]);
  expect(before.toString()).toBe('    println!("{r} {');
});

it("進み具合が 0 の字句は囲まない", () => {
  const { container } = renderWith({
    marks: [{ line: 14, text: "&x", progress: 0 }],
    errors: [{ line: 14, text: "&x", progress: 0 }],
  });
  expect(container.querySelector(".mark")).toBeNull();
  expect(container.querySelector(".error")).toBeNull();
});

it("errors は span.error で囲み、marks と重ねるときは mark が外になる", () => {
  const { container } = renderWith({
    marks: [{ line: 14, text: "&x" }],
    errors: [{ line: 14, text: "&x", progress: 0.25 }],
  });
  const error = container.querySelector<HTMLElement>(".mark > .error");
  expect(error).not.toBeNull();
  expect(error?.textContent).toBe("&x");
  expect(error?.style.getPropertyValue("--error-p")).toBe("0.25");
});

it("複数の字句にまたがる指定は、1 つの囲みにまとまる", () => {
  const { container } = renderWith({
    marks: [{ line: 13, text: "let x" }],
  });
  const marks = container.querySelectorAll(".mark");
  expect(marks).toHaveLength(1);
  expect(marks[0].textContent).toBe("let x");
  expect(marks[0].querySelector(".tk-k")?.textContent).toBe("let");
});

it("marks・errors の制約に違反すると、描画時にエラーになる", () => {
  expectThrow({ marks: [{ line: 14, text: "zzz" }] }, /ない/);
  expectThrow({ marks: [{ line: 16, text: "r", nth: 4 }] }, /4 個ない/);
  expectThrow({ marks: [{ line: 9, text: "fn" }] }, /の外にある/);
  expectThrow({ errors: [{ line: 18, text: "fn" }] }, /の外にある/);
  expectThrow({ marks: [{ line: 10, text: "fn", nth: 0 }] }, /nth/);
  const text: CodeBlockData = { id: "a", language: "text", code: "error: x" };
  expectThrow({ marks: [{ line: 1, text: "error" }] }, /text/, text);
  expectThrow({ errors: [{ line: 1, text: "error" }] }, /text/, text);
});

it("diff では、変更後のファイルの行番号で marks を指す", () => {
  const { container } = renderWith(
    { marks: [{ line: 6, text: "b" }] },
    {
      id: "a",
      language: "rust",
      start_line: 5,
      diff: true,
      code: " a\n-old\n+b",
    },
  );
  expect(container.querySelector(".mark")?.textContent).toBe("b");
  expect(container.querySelector(".is-add .mark")).not.toBeNull();
});

it("lines の行番号が code_blocks の行番号の外にあると、描画時にエラーになる", () => {
  expectThrow({ lines: { 9: { dim: 1 } } }, /lines の行 9/);
  expectThrow({ lines: { 18: { open: 0 } } }, /lines の行 18/);
  const text: CodeBlockData = { id: "a", language: "text", code: "a\nb" };
  expectThrow({ lines: { 3: { dim: 1 } } }, /lines の行 3/, text);
});

it("language が text の lines は、1 から数えた行の位置で指す", () => {
  const { container } = renderWith(
    { lines: { 2: { dim: 1 } } },
    { id: "a", language: "text", start_line: 50, code: "a\nb" },
  );
  const lines = container.querySelectorAll<HTMLElement>(".line");
  expect(lines[0].style.getPropertyValue("--line-dim")).toBe("");
  expect(lines[1].style.getPropertyValue("--line-dim")).toBe("1");
});

it("行を差し込む動きの途中は、差し込んでいない行を数えずに行番号を表示する", () => {
  const { container } = renderWith({ lines: { 12: { open: 0.5 } } });
  const line = container.querySelectorAll<HTMLElement>(".line");
  expect(lns(container)).toEqual([
    "10",
    "11",
    "",
    "12",
    "13",
    "14",
    "15",
    "16",
  ]);
  expect(line[2]).toHaveClass("is-opening");
  expect(line[2].style.getPropertyValue("--line-open")).toBe("0.5");
  expect(line[3]).not.toHaveClass("is-opening");
});

it("まだ差し込みが始まっていない行も数えない", () => {
  const { container } = renderWith({
    lines: { 12: { open: 0 }, 14: { open: 0 } },
  });
  expect(lns(container)).toEqual(["10", "11", "", "12", "", "13", "14", "15"]);
});

it("差し込みが終わると、行番号は code_blocks の行番号と同じになる", () => {
  const { container } = renderWith({
    lines: { 12: { open: 1 }, 14: { open: 1 } },
  });
  expect(lns(container)).toEqual([
    "10",
    "11",
    "12",
    "13",
    "14",
    "15",
    "16",
    "17",
  ]);
  expect(container.querySelector(".is-opening")).toBeNull();
});

it("差し込みの途中は、範囲の枠の位置に、開いている分が足される", () => {
  const { container } = renderWith({
    scopes: [{ label: "r", from: 11, to: 16 }],
    lines: { 12: { open: 0.5 } },
  });
  const scope = container.querySelector<HTMLElement>("span.scope");
  expect(scope?.style.getPropertyValue("--from")).toBe("2");
  expect(scope?.style.getPropertyValue("--to")).toBe("6.5");
});

it("入力の途中は、入力した分の文字と、末尾のカーソルを出す", () => {
  const { container } = renderWith({ lines: { 11: { typed: 0.5 } } });
  const line = container.querySelectorAll(".line")[1];
  expect(line.textContent).toBe("    l");
  expect(line.querySelector(".caret")).not.toBeNull();
  const done = renderWith({ lines: { 11: { typed: 1 } } });
  expect(done.container.querySelector(".caret")).toBeNull();
});

it("入力の途中の行では、入力していない字句の指定は出ない", () => {
  const { container } = renderWith({
    lines: { 14: { typed: 0.1 } },
    marks: [{ line: 14, text: "&x" }],
  });
  expect(container.querySelector(".mark")).toBeNull();
});

it("dim は、行の --line-dim になる", () => {
  const { container } = renderWith({
    lines: { 10: { dim: 1 }, 11: { dim: 0.5 } },
  });
  const lines = container.querySelectorAll<HTMLElement>(".line");
  expect(lines[0].style.getPropertyValue("--line-dim")).toBe("1");
  expect(lines[1].style.getPropertyValue("--line-dim")).toBe("0.5");
  expect(lines[2].style.getPropertyValue("--line-dim")).toBe("");
});
