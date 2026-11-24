import { describe, expect, it } from "vitest";
import editorMainSource from "monaco-editor/editor/editor.main.js?raw";
import entrySource from "./monaco.ts?raw";

// import の指定子を、monaco-editor の esm/vs/ からのパス（拡張子の .js を除く）にする。
function fromEsmVs(specifier: string): string {
  const css = /monaco-editor\/esm\/vs\/(.+\.css)$/.exec(specifier);
  if (css !== null) {
    return css[1];
  }
  return specifier.replace(/^monaco-editor\//, "");
}

// monaco-editor の既定の入口（esm/vs/editor/editor.main.js）が読み込むもの。
function editorMainImports(): string[] {
  const base = ["editor"];
  return [...editorMainSource.matchAll(/^import (?:.+ from )?'([^']+)';$/gm)]
    .map((match) => match[1])
    .map((relative) => {
      const parts = [...base];
      for (const part of relative.split("/")) {
        if (part === "..") parts.pop();
        else if (part !== ".") parts.push(part);
      }
      return parts.join("/").replace(/\.js$/, "");
    });
}

// 入口（monaco.ts）の「編集機能」の一覧。
function entryEditingImports(): string[] {
  const start = entrySource.indexOf("// ---- 編集機能（");
  const end = entrySource.indexOf("// ---- 編集機能ここまで ----");
  return [
    ...entrySource.slice(start, end).matchAll(/^import "([^"]+)";$/gm),
  ].map((match) => fromEsmVs(match[1]));
}

describe("Monaco の入口", () => {
  it("既定の入口から、言語の定義・言語機能・LSP だけを除いて、同じ順に読み込む", () => {
    const expected = editorMainImports().filter(
      (path) =>
        !path.startsWith("languages/definitions/") &&
        !path.startsWith("languages/features/") &&
        !path.startsWith("external/monaco-lsp-client/"),
    );
    // 既定の入口の一覧が読めていることを確かめる（読めないと、比べる意味が無い）。
    expect(expected).toContain("editor/contrib/folding/browser/folding");
    expect(expected).toContain(
      "base/browser/ui/codicons/codicon/codicon-modifiers.css",
    );
    expect(entryEditingImports()).toEqual(expected);
  });

  it("既定の入口の export は、editor.api のものだけ", () => {
    // 既定の入口が、編集機能の一覧の外で新しい物を読み込み始めたら、ここで気づく。
    const outside = [
      ...editorMainSource.matchAll(/^export .+ from '([^']+)';$/gm),
    ].map((match) => match[1]);
    expect(outside).toEqual(["./editor.api.js"]);
  });
});
