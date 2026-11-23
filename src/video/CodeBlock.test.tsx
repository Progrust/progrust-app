import { render } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { CodeBlock } from "./CodeBlock";
import { CodeBlocksContext, type CodeBlockData } from "./hooks";
import { tokenizeOutput, tokenizeRust } from "./highlight";

function renderBlock(blocks: CodeBlockData[], block: string) {
  return render(
    <CodeBlocksContext.Provider value={blocks}>
      <CodeBlock block={block} />
    </CodeBlocksContext.Provider>,
  );
}

const lns = (container: HTMLElement) =>
  Array.from(container.querySelectorAll<HTMLElement>(".line")).map((l) =>
    l.getAttribute("data-ln"),
  );

it("start_line が 118 のコードブロックは、行番号が 118 から表示される", () => {
  const { container } = renderBlock(
    [
      {
        id: "main",
        language: "rust",
        start_line: 118,
        code: "fn main() {\n    let x = 5;\n}",
      },
    ],
    "main",
  );
  expect(lns(container)).toEqual(["118", "119", "120"]);
});

it("start_line を省くと、行番号は 1 から表示される", () => {
  const { container } = renderBlock(
    [{ id: "a", language: "python", code: "a = 1\nb = 2" }],
    "a",
  );
  expect(lns(container)).toEqual(["1", "2"]);
});

it("diff では、削除行に行番号が表示されず、ほかの行は変更後のファイルの行番号で表示される", () => {
  const { container } = renderBlock(
    [
      {
        id: "fix",
        language: "rust",
        start_line: 12,
        diff: true,
        code: '     let s1 = String::from("hi");\n-    let s2 = s1;\n+    let s2 = s1.clone();\n     println!("{s1} {s2}");',
      },
    ],
    "fix",
  );
  expect(lns(container)).toEqual(["12", "", "13", "14"]);
  const lines = container.querySelectorAll(".line");
  expect(lines[1]).toHaveClass("is-del");
  expect(lines[2]).toHaveClass("is-add");
  expect(container.querySelector("code")).toHaveAttribute("data-diff");
  expect(lines[2].textContent).toBe("    let s2 = s1.clone();");
});

it("diff の空の行は、変わらない行として行番号が付く", () => {
  const { container } = renderBlock(
    [{ id: "d", language: "rust", diff: true, code: " a\n\n+b" }],
    "d",
  );
  expect(lns(container)).toEqual(["1", "2", "3"]);
});

it("language が text のコードブロックには、行番号が表示されない", () => {
  const { container } = renderBlock(
    [
      {
        id: "error",
        language: "text",
        code: "error[E0382]: x\n --> src/main.rs:4:16",
      },
    ],
    "error",
  );
  expect(lns(container)).toEqual(["", ""]);
  expect(container.querySelector("code")).toHaveClass("language-text");
});

it("code_blocks にない id を block に指定すると、描画時にエラーになる", () => {
  const spy = vi.spyOn(console, "error").mockImplementation(() => {});
  expect(() => renderBlock([], "none")).toThrow(/none/);
  spy.mockRestore();
});

it("label は pre の data-label になる", () => {
  const { container } = renderBlock(
    [{ id: "a", label: "src/main.rs", language: "rust", code: "x" }],
    "a",
  );
  expect(container.querySelector("pre")).toHaveAttribute(
    "data-label",
    "src/main.rs",
  );
});

it("rust だけ字句に色分けされ、ほかの言語と plaintext はされない", () => {
  const code = 'let s = "hi";';
  const rust = renderBlock([{ id: "a", language: "rust", code }], "a");
  expect(rust.container.querySelector(".tk-k")?.textContent).toBe("let");
  expect(rust.container.querySelector(".tk-s")?.textContent).toBe('"hi"');
  rust.unmount();
  for (const language of ["python", "plaintext"]) {
    const other = renderBlock([{ id: "a", language, code }], "a");
    expect(other.container.querySelector('[class^="tk-"]')).toBeNull();
    other.unmount();
  }
});

it("rust の字句を区分ごとに分ける", () => {
  const src =
    "#[derive(Debug)]\nfn f<'a>(x: &'a str) -> i32 { println!(\"{x}\"); let c = 'z'; // 注\n    String::new(); g(0x1F); true }";
  const tokens = tokenizeRust(src);
  expect(tokens.map((t) => t.text).join("")).toBe(src);
  const pick = (cls: string) =>
    tokens.filter((t) => t.cls === cls).map((t) => t.text);
  expect(pick("tk-a")).toEqual(["#[derive(Debug)]"]);
  expect(pick("tk-k")).toEqual(["fn", "let"]);
  expect(pick("tk-l")).toEqual(["'a", "'a"]);
  expect(pick("tk-m")).toEqual(["println!"]);
  expect(pick("tk-s")).toEqual(['"{x}"', "'z'"]);
  expect(pick("tk-c")).toEqual(["// 注"]);
  expect(pick("tk-t")).toEqual(["str", "i32", "String"]);
  expect(pick("tk-n")).toEqual(["0x1F", "true"]);
  expect(pick("tk-f")).toEqual(["new", "g"]);
});

it("複数行の文字列・コメントは、行ごとに色が付く", () => {
  const { container } = renderBlock(
    [{ id: "a", language: "rust", code: '/* a\n b */ "x\ny"' }],
    "a",
  );
  const lines = container.querySelectorAll(".line");
  expect(lines).toHaveLength(3);
  expect(lines[0].querySelector(".tk-c")?.textContent).toBe("/* a");
  expect(lines[1].querySelector(".tk-s")?.textContent).toBe('"x');
});

it("rustc の出力を、種類ごとに色付けする", () => {
  const tokens = tokenizeOutput([
    "error[E0382]: borrow of moved value: `s1`",
    " --> src/main.rs:4:16",
    "  |",
    '4 |     println!("{s1}");',
    "  |                ^^ value borrowed here after move",
    "  = help: consider cloning",
    "warning: unused",
  ]);
  expect(tokens[0][0]).toEqual({ cls: "out-error", text: "error[E0382]" });
  expect(tokens[0][1].cls).toBe("out-note");
  expect(tokens[1]).toEqual([
    { cls: "out-loc", text: " --> src/main.rs:4:16" },
  ]);
  expect(tokens[3].find((t) => t.cls === "out-loc")?.text).toBe("4 |");
  expect(tokens[4].some((t) => t.cls === "out-caret")).toBe(true);
  expect(tokens[5].some((t) => t.cls === "out-help")).toBe(true);
  expect(tokens[6][0].cls).toBe("out-warning");
});
