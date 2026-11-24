// monaco-editor の Rust の定義（型の宣言が配られていない）。
declare module "monaco-editor/languages/definitions/rust/rust" {
  import type { languages } from "monaco-editor/editor/editor.api";

  export const conf: languages.LanguageConfiguration;
  export const language: languages.IMonarchLanguage;
}
