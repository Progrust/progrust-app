// エディタの配色（documents/spec/screens/appearance.md「テーマ」、ADR 0225）。
// 色の値は持たず、デザインシステムの CSS 変数の値から、Monaco のテーマを作る
// （design-system/progrust/README.md「コード」）。
import type { editor } from "monaco-editor/editor/editor.api";

export type EditorThemeMode = "light" | "dark";

// テーマを作るのに使う CSS 変数。
export const THEME_VARIABLES = [
  "--bg",
  "--panel",
  "--raised",
  "--field",
  "--line",
  "--line-faint",
  "--control-line",
  "--chart-bar",
  "--mark",
  "--ink",
  "--muted",
  "--link",
  "--code-keyword",
  "--code-string",
  "--code-macro",
  "--code-comment",
  "--code-type",
  "--code-function",
  "--code-line-number",
] as const;

export type ThemeVariable = (typeof THEME_VARIABLES)[number];

// CSS 変数の名前と値。値は CSS に書かれたままの文字列。
export type ThemeValues = Readonly<Record<ThemeVariable, string>>;

// Monaco に登録するテーマの名前。
export function editorThemeName(mode: EditorThemeMode): string {
  return `progrust-${mode}`;
}

const HEX = /^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const RGB =
  /^rgba?\(\s*(\d+(?:\.\d+)?)\s*,\s*(\d+(?:\.\d+)?)\s*,\s*(\d+(?:\.\d+)?)\s*(?:,\s*(\d*(?:\.\d+)?)\s*)?\)$/i;

const byte = (n: number) =>
  Math.round(Math.min(255, Math.max(0, n)))
    .toString(16)
    .padStart(2, "0");

// CSS の色の値を、Monaco が読める `#RRGGBB`（不透明）か `#RRGGBBAA` にする。
// 読めない値（空、var() など）は null。
export function toHexColor(value: string): string | null {
  const text = value.trim();
  const hex = HEX.exec(text);
  if (hex !== null) {
    let digits = hex[1];
    if (digits.length <= 4) {
      digits = [...digits].map((d) => d + d).join("");
    }
    if (digits.length === 8 && digits.slice(6).toLowerCase() === "ff") {
      digits = digits.slice(0, 6);
    }
    return `#${digits.toUpperCase()}`;
  }
  const rgb = RGB.exec(text);
  if (rgb !== null) {
    const [r, g, b] = [rgb[1], rgb[2], rgb[3]].map(Number);
    const alpha = rgb[4] === undefined ? 1 : Number(rgb[4]);
    const head = `#${byte(r)}${byte(g)}${byte(b)}`;
    const tail = alpha >= 1 ? "" : byte(alpha * 255);
    return (head + tail).toUpperCase();
  }
  return null;
}

// 字句の区分と、色の CSS 変数。字句の名前は src/editor/rustLanguage.ts と Monaco の Rust の定義のもの。
// 区分の無い字句（数値、演算子、名前など）は、文字の色（--ink）。
const TOKEN_RULES: readonly (readonly [string, ThemeVariable])[] = [
  ["keyword", "--code-keyword"],
  ["keyword.type", "--code-type"],
  ["type", "--code-type"],
  ["macro", "--code-macro"],
  ["function", "--code-function"],
  ["string", "--code-string"],
  ["comment", "--code-comment"],
];

// Monaco の色の名前と、色の CSS 変数。
const EDITOR_COLORS: readonly (readonly [string, ThemeVariable])[] = [
  ["editor.background", "--bg"],
  ["editor.foreground", "--ink"],
  ["editorGutter.background", "--bg"],
  ["editorLineNumber.foreground", "--code-line-number"],
  ["editorLineNumber.activeForeground", "--muted"],
  ["editorCursor.foreground", "--ink"],
  ["editor.lineHighlightBackground", "--panel"],
  ["editor.lineHighlightBorder", "--panel"],
  ["editor.selectionBackground", "--chart-bar"],
  ["editor.inactiveSelectionBackground", "--line"],
  ["editor.selectionHighlightBackground", "--field"],
  ["editor.wordHighlightBackground", "--field"],
  ["editor.wordHighlightStrongBackground", "--field"],
  ["editor.findMatchBackground", "--mark"],
  ["editor.findMatchHighlightBackground", "--mark"],
  ["editorBracketMatch.background", "--field"],
  ["editorBracketMatch.border", "--control-line"],
  ["editorIndentGuide.background1", "--line-faint"],
  ["editorIndentGuide.activeBackground1", "--line"],
  ["editorWhitespace.foreground", "--line"],
  ["editorOverviewRuler.border", "--bg"],
  ["scrollbarSlider.background", "--line"],
  ["scrollbarSlider.hoverBackground", "--chart-bar"],
  ["scrollbarSlider.activeBackground", "--chart-bar"],
  ["editorWidget.background", "--raised"],
  ["editorWidget.foreground", "--ink"],
  ["editorWidget.border", "--line"],
  ["editorHoverWidget.background", "--raised"],
  ["editorHoverWidget.border", "--line"],
  ["input.background", "--field"],
  ["input.foreground", "--ink"],
  ["input.border", "--control-line"],
  ["focusBorder", "--link"],
  ["link.foreground", "--link"],
];

// CSS 変数の値から、Monaco のテーマを作る。読めない値の色は、Monaco の既定の色のままにする。
export function buildEditorTheme(
  mode: EditorThemeMode,
  values: ThemeValues,
): editor.IStandaloneThemeData {
  const color = (name: ThemeVariable) => toHexColor(values[name]);
  // 字句の色は、不透明の `RRGGBB` だけが使える。
  const tokenColor = (name: ThemeVariable) => color(name)?.slice(1, 7);

  const rules: editor.ITokenThemeRule[] = [];
  const base = tokenColor("--ink");
  const background = tokenColor("--bg");
  rules.push({
    token: "",
    ...(base !== undefined && { foreground: base }),
    ...(background !== undefined && { background }),
  });
  for (const [token, name] of TOKEN_RULES) {
    const foreground = tokenColor(name);
    if (foreground !== undefined) {
      rules.push({ token, foreground });
    }
  }

  const colors: editor.IColors = {};
  for (const [key, name] of EDITOR_COLORS) {
    const value = color(name);
    if (value !== null) {
      colors[key] = value;
    }
  }

  return {
    base: mode === "dark" ? "vs-dark" : "vs",
    // Monaco の既定のテーマの字句の色を引き継がない（デザインシステムに無い色を出さない）。
    inherit: false,
    rules,
    colors,
  };
}

// いま読み込まれている CSS から、テーマ（ライト・ダーク）ごとの CSS 変数の値を読む。
// デザインシステムの CSS は、data-theme 属性を持つ要素に、そのテーマの値を定める
// （design-system/progrust/tokens.css）。属性を付けた見えない要素を置いて、値を読む。
export function readThemeValues(
  mode: EditorThemeMode,
  doc: Document = document,
): ThemeValues {
  const probe = doc.createElement("div");
  probe.dataset.theme = mode;
  probe.hidden = true;
  doc.body.append(probe);
  try {
    const style = doc.defaultView?.getComputedStyle(probe);
    const entries = THEME_VARIABLES.map(
      (name) => [name, style?.getPropertyValue(name).trim() ?? ""] as const,
    );
    return Object.fromEntries(entries) as Record<ThemeVariable, string>;
  } finally {
    probe.remove();
  }
}

// エディタの書体（デザインシステムの --font-mono）。読めないときは null。
export function readMonoFont(doc: Document = document): string | null {
  const value = doc.defaultView
    ?.getComputedStyle(doc.documentElement)
    .getPropertyValue("--font-mono")
    .trim();
  return value === undefined || value === "" ? null : value;
}
