import { afterEach, describe, expect, it } from "vitest";
import tokensCss from "../../design-system/progrust/tokens.css?raw";
import {
  buildEditorTheme,
  editorThemeName,
  readMonoFont,
  readThemeValues,
  THEME_VARIABLES,
  toHexColor,
  type ThemeValues,
} from "./editorTheme";

// デザインシステムの tokens.css を、ページに読み込む。
function loadTokens(): HTMLStyleElement {
  const style = document.createElement("style");
  style.textContent = tokensCss;
  document.head.append(style);
  return style;
}

afterEach(() => {
  document.head.innerHTML = "";
  document.body.innerHTML = "";
  delete document.documentElement.dataset.theme;
});

describe("toHexColor", () => {
  it("16 進の色を #RRGGBB の大文字にする", () => {
    expect(toHexColor("#8b2fb0")).toBe("#8B2FB0");
    expect(toHexColor(" #FFFFFF ")).toBe("#FFFFFF");
    expect(toHexColor("#abc")).toBe("#AABBCC");
  });

  it("透明度のある色は #RRGGBBAA にし、不透明なら #RRGGBB にする", () => {
    expect(toHexColor("#11223380")).toBe("#11223380");
    expect(toHexColor("#112233ff")).toBe("#112233");
    expect(toHexColor("#abcd")).toBe("#AABBCCDD");
  });

  it("rgb()・rgba() を 16 進にする", () => {
    expect(toHexColor("rgb(26, 31, 43)")).toBe("#1A1F2B");
    expect(toHexColor("rgba(26, 31, 43, 0.06)")).toBe("#1A1F2B0F");
    expect(toHexColor("rgba(255, 255, 255, 1)")).toBe("#FFFFFF");
  });

  it("読めない値は null", () => {
    expect(toHexColor("")).toBeNull();
    expect(toHexColor("var(--ink)")).toBeNull();
    expect(toHexColor("red")).toBeNull();
  });
});

// tokens.css から、テーマごとの値を読む（ページに読み込んで、readThemeValues で読む）。
function tokenValues(mode: "light" | "dark"): ThemeValues {
  const style = loadTokens();
  try {
    return readThemeValues(mode);
  } finally {
    style.remove();
  }
}

describe("readThemeValues", () => {
  it("ページのテーマに関わらず、テーマごとの CSS 変数の値を読む", () => {
    document.documentElement.dataset.theme = "dark";
    const light = tokenValues("light");
    document.documentElement.dataset.theme = "light";
    const dark = tokenValues("dark");

    expect(light["--bg"]).toBe("#FFFFFF");
    expect(light["--code-keyword"]).toBe("#8B2FB0");
    expect(dark["--bg"]).toBe("#13161D");
    expect(dark["--code-keyword"]).toBe("#D7A0F5");
    for (const name of THEME_VARIABLES) {
      expect(light[name], name).not.toBe("");
      expect(dark[name], name).not.toBe("");
    }
  });

  it("値を読むために置いた要素を残さない", () => {
    tokenValues("dark");
    expect(document.body.children).toHaveLength(0);
  });
});

describe("buildEditorTheme", () => {
  it("ライトはデザインシステムのライトの値から、Monaco のライトのテーマを土台に作る", () => {
    const theme = buildEditorTheme("light", tokenValues("light"));
    expect(theme.base).toBe("vs");
    expect(theme.inherit).toBe(false);
    expect(theme.colors["editor.background"]).toBe("#FFFFFF");
    expect(theme.colors["editor.foreground"]).toBe("#1A1F2B");
    expect(theme.colors["editorLineNumber.foreground"]).toBe("#A1A9B6");
    expect(theme.rules).toEqual([
      { token: "", foreground: "1A1F2B", background: "FFFFFF" },
      { token: "keyword", foreground: "8B2FB0" },
      { token: "keyword.type", foreground: "A35A00" },
      { token: "type", foreground: "A35A00" },
      { token: "macro", foreground: "0B5CA8" },
      { token: "function", foreground: "1F4FA3" },
      { token: "string", foreground: "00795A" },
      { token: "comment", foreground: "6A7384" },
    ]);
  });

  it("ダークはデザインシステムのダークの値から、Monaco のダークのテーマを土台に作る", () => {
    const theme = buildEditorTheme("dark", tokenValues("dark"));
    expect(theme.base).toBe("vs-dark");
    expect(theme.colors["editor.background"]).toBe("#13161D");
    expect(theme.colors["editor.foreground"]).toBe("#E8EBF1");
    expect(theme.rules).toContainEqual({
      token: "keyword",
      foreground: "D7A0F5",
    });
    expect(theme.rules).toContainEqual({
      token: "comment",
      foreground: "8F98A8",
    });
  });

  it("色はすべて Monaco が読める 16 進で、字句の色は不透明の RRGGBB", () => {
    for (const mode of ["light", "dark"] as const) {
      const theme = buildEditorTheme(mode, tokenValues(mode));
      for (const value of Object.values(theme.colors)) {
        expect(value).toMatch(/^#[0-9A-F]{6}([0-9A-F]{2})?$/);
      }
      for (const rule of theme.rules) {
        expect(rule.foreground).toMatch(/^[0-9A-F]{6}$/);
      }
    }
  });

  it("読めない値の色は入れない（Monaco の既定の色のままにする）", () => {
    const values = {
      ...tokenValues("light"),
      "--bg": "",
      "--code-macro": "var(--link)",
    };
    const theme = buildEditorTheme("light", values);
    expect(theme.colors).not.toHaveProperty("editor.background");
    expect(theme.rules[0]).toEqual({ token: "", foreground: "1A1F2B" });
    expect(theme.rules.map((rule) => rule.token)).not.toContain("macro");
  });
});

describe("editorThemeName", () => {
  it("テーマごとに別の名前", () => {
    expect(editorThemeName("light")).toBe("progrust-light");
    expect(editorThemeName("dark")).toBe("progrust-dark");
  });
});

describe("readMonoFont", () => {
  it("デザインシステムの --font-mono を読む", () => {
    loadTokens();
    // 区切りの空白は、CSS の読み込み方で変わるので見ない。
    expect(readMonoFont()?.split(/,\s*/)).toEqual([
      '"Red Hat Mono"',
      '"Murecho"',
      "ui-monospace",
      "Menlo",
      "monospace",
    ]);
  });

  it("CSS が無いときは null", () => {
    expect(readMonoFont()).toBeNull();
  });
});
