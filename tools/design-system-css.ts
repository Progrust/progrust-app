// デザインシステムの CSS の読み込み（documents/spec/screens/appearance.md「デザインシステムの CSS とアイコン」）。
// Vite のプラグインと、教材チェックツールの描画（#110）で使う。

// `design-system/*/components/bundle.css` のパスか。
export function isDesignSystemBundleCss(id: string): boolean {
  const path = id.split("?")[0].replaceAll("\\", "/");
  return /\/design-system\/[^/]+\/components\/bundle\.css$/.test(path);
}

// 書体を外部から読み込む `@import` の行（`@import url("https://…");`）を取り除く。
// 外部（http・https・`//`）から読み込む `@import` だけを取り除き、ほかの行は変えない。
export function stripFontImport(css: string): string {
  return css.replace(
    /^[ \t]*@import\s+(?:url\(\s*)?["']?(?:https?:)?\/\/[^\n]*;[ \t]*\r?\n?/gm,
    "",
  );
}

// Vite のプラグイン。開発時の起動と本番ビルドの両方で、bundle.css から外部の書体の `@import` を取り除く。
export function stripDesignSystemFontImport() {
  return {
    name: "progrust:strip-design-system-font-import",
    enforce: "pre" as const,
    transform(code: string, id: string) {
      if (!isDesignSystemBundleCss(id)) {
        return null;
      }
      return { code: stripFontImport(code), map: null };
    },
  };
}
