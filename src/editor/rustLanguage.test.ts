import { beforeAll, describe, expect, it, vi } from "vitest";
import { extendRustLanguage } from "./rustLanguage";

// Monaco の字句分けは DOM を使わないので、jsdom でも動く。
// Monaco は読み込むときに matchMedia を使うが、jsdom には無いので、先に用意する。
let monaco: typeof import("monaco-editor/editor/editor.api");

// 1 行のコードを字句に分け、[字句の文字列, 字句の名前] の並びにする（空白は除く）。
function tokens(line: string): [string, string][] {
  const [lineTokens] = monaco.editor.tokenize(line, "rust-test");
  return lineTokens
    .map((token, index): [string, string] => [
      line.slice(token.offset, lineTokens[index + 1]?.offset ?? line.length),
      token.type.replace(/\.rust$/, ""),
    ])
    .filter(([text]) => text.trim() !== "");
}

// Monaco の読み込みは重く、全体を並行して動かすと既定の 10 秒を超えることがあるので、時間切れを延ばす。
beforeAll(async () => {
  vi.stubGlobal(
    "matchMedia",
    (query: string) =>
      ({
        matches: false,
        media: query,
        addEventListener: () => {},
        removeEventListener: () => {},
      }) as unknown as MediaQueryList,
  );
  monaco = await import("monaco-editor/editor/editor.api");
  const { conf, language } =
    await import("monaco-editor/languages/definitions/rust/rust");
  monaco.languages.register({ id: "rust-test" });
  monaco.languages.setLanguageConfiguration("rust-test", conf);
  monaco.languages.setMonarchTokensProvider(
    "rust-test",
    extendRustLanguage(language),
  );
}, 60_000);

describe("extendRustLanguage", () => {
  it("マクロ・関数・型・キーワードを分ける", () => {
    expect(tokens('println!("{name}");')).toEqual([
      ["println!", "macro"],
      ["(", "delimiter.parenthesis"],
      ['"', "string.quote"],
      ["{name}", "string"],
      ['"', "string.quote"],
      [")", "delimiter.parenthesis"],
      [";", "operator"],
    ]);
    expect(tokens("let mut name = String::new();")).toEqual([
      ["let", "keyword"],
      ["mut", "keyword"],
      ["name", "identifier"],
      ["=", "operator"],
      ["String", "type"],
      ["::", ""],
      ["new", "function"],
      ["()", "delimiter.parenthesis"],
      [";", "operator"],
    ]);
  });

  it("名前の後に ( か ::< が続くものを関数にし、キーワードは関数にしない", () => {
    expect(tokens("fn main() {")).toEqual([
      ["fn", "keyword"],
      ["main", "function"],
      ["()", "delimiter.parenthesis"],
      ["{", "delimiter.curly"],
    ]);
    expect(tokens("let v = x.parse::<u32>();")).toContainEqual([
      "parse",
      "function",
    ]);
    expect(tokens("while (a) {}")[0]).toEqual(["while", "keyword"]);
    expect(tokens("let x: i32 = f(1);")).toContainEqual([
      "i32",
      "keyword.type",
    ]);
  });

  it("!= の前の名前はマクロにしない", () => {
    expect(tokens("a!=b")).toEqual([
      ["a", "identifier"],
      ["!=", "operator"],
      ["b", "identifier"],
    ]);
  });

  it("Monaco の定義の字句（文字列・コメント・ライフタイム・数値）はそのまま", () => {
    expect(tokens("// コメント")).toEqual([["// コメント", "comment"]]);
    expect(tokens('let s = r#"raw"#;')).toContainEqual(['r#"', "string.quote"]);
    expect(tokens("fn f<'a>(x: &'a str) {}")).toContainEqual([
      "'a",
      "identifier",
    ]);
    expect(tokens("let n = 42u32;")).toContainEqual(["42u32", "number"]);
  });
});
