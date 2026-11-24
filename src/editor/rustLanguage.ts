// エディタの Rust の字句の定義（ADR 0225）。
// Monaco の Rust の定義に、デザインシステムの字句の区分（マクロ・関数・型）を分ける規則を足す。
// 区分の分け方は、動画のコードの色分け（src/video/highlight.ts）とそろえる。
import type { languages } from "monaco-editor/editor/editor.api";

// 先頭に足す規則。Monaco の定義の規則より先に試す。
const ADDED_RULES: languages.IMonarchLanguageRule[] = [
  // マクロの呼び出し（println!、vec! など）。`!=` は除く。
  [/[a-zA-Z_][a-zA-Z0-9_]*!(?!=)/, "macro"],
  // `!=` の前の名前（Monaco の定義のままだと、`a!` を 1 つの名前にする）。
  [
    /[a-z_][a-zA-Z0-9_]*(?=!=)/,
    { cases: { "@keywords": "keyword", "@default": "identifier" } },
  ],
  // 関数（名前の後に `(` か `::<` が続く）。if・while などのキーワードは除く。
  [
    /[a-z_][a-zA-Z0-9_]*(?=\s*(?:\(|::<))/,
    {
      cases: {
        "@keywords": "keyword",
        "@typeKeywords": "keyword.type",
        "@default": "function",
      },
    },
  ],
  // 大文字で始まる名前は型（String、Option、Self など）。
  [/[A-Z][a-zA-Z0-9_]*/, "type"],
];

// Monaco の Rust の定義（monaco-editor/languages/definitions/rust/rust）に、規則を足した定義を返す。
export function extendRustLanguage(
  base: languages.IMonarchLanguage,
): languages.IMonarchLanguage {
  return {
    ...base,
    tokenizer: {
      ...base.tokenizer,
      root: [...ADDED_RULES, ...base.tokenizer.root],
    },
  };
}
