// レッスン専用の CSS の変換（documents/spec/content-format/video/structure.md、ADR 0093）を確かめる。
import { describe, expect, it } from "vitest";
import { lessonSelector, scopeLessonCss } from "./lesson-css.ts";

// 空白をまとめて、比べやすくする。
function normalize(css: string): string {
  return css.replace(/\s+/g, " ").trim();
}

function scoped(css: string, lessonId = "ch04-02"): string {
  return normalize(scopeLessonCss(css, lessonId));
}

describe("scopeLessonCss", () => {
  it("クラスのセレクタの前に、レッスンの属性セレクタを付ける", () => {
    expect(scoped(".box { gap: 4px; }")).toBe(
      '[data-lesson="ch04-02"] .box { gap: 4px; }',
    );
  });

  it("要素だけのセレクタと、デザインシステムの部品のクラスにも付ける", () => {
    expect(scoped("h5 { margin: 0; }\n.callout { padding: 8px; }")).toBe(
      '[data-lesson="ch04-02"] h5 { margin: 0; } [data-lesson="ch04-02"] .callout { padding: 8px; }',
    );
  });

  it("セレクタの並びでは、1 つずつに付ける", () => {
    expect(scoped(".a, .b > p,\n.c:is(.d, .e) { margin: 0; }")).toBe(
      '[data-lesson="ch04-02"] .a, [data-lesson="ch04-02"] .b > p, [data-lesson="ch04-02"] .c:is(.d, .e) { margin: 0; }',
    );
  });

  it(":root は、レッスンの属性セレクタに置き換える", () => {
    expect(scoped(":root { --gap: 4px; }")).toBe(
      '[data-lesson="ch04-02"] { --gap: 4px; }',
    );
  });

  it("先頭が :root のセレクタは、:root の部分を置き換える", () => {
    expect(scoped(":root .box, :root > p, .a { margin: 0; }")).toBe(
      '[data-lesson="ch04-02"] .box, [data-lesson="ch04-02"] > p, [data-lesson="ch04-02"] .a { margin: 0; }',
    );
  });

  it("@media・@supports・@layer の中のルールにも付ける", () => {
    expect(
      scoped(
        "@media (width >= 600px) { .a { margin: 0; } }\n" +
          "@supports (display: grid) { :root { --x: 1; } }\n" +
          "@layer lesson { h6 { margin: 0; } }",
      ),
    ).toBe(
      '@media (width >= 600px) { [data-lesson="ch04-02"] .a { margin: 0; } } ' +
        '@supports (display: grid) { [data-lesson="ch04-02"] { --x: 1; } } ' +
        '@layer lesson { [data-lesson="ch04-02"] h6 { margin: 0; } }',
    );
  });

  it("入れ子のルールは、いちばん外側のルールにだけ付ける", () => {
    expect(
      scoped(".card { margin: 0; & .title { margin: 0; } p { margin: 0; } }"),
    ).toBe(
      '[data-lesson="ch04-02"] .card { margin: 0; & .title { margin: 0; } p { margin: 0; } }',
    );
  });

  it("@keyframes のキーフレームには付けない", () => {
    expect(
      scoped("@keyframes fade { from { opacity: 0; } 50% { opacity: 1; } }"),
    ).toBe("@keyframes fade { from { opacity: 0; } 50% { opacity: 1; } }");
  });

  it("セレクタを持たない規則（@font-face・@property）は変えない", () => {
    const css =
      '@font-face { font-family: x; src: url("x.woff2"); }\n' +
      '@property --angle { syntax: "<angle>"; inherits: false; initial-value: 0deg; }';
    expect(scoped(css)).toBe(normalize(css));
  });

  it("レッスンごとに、そのレッスンの ID を付ける", () => {
    expect(scoped(".box { margin: 0; }", "ch02-01")).toBe(
      '[data-lesson="ch02-01"] .box { margin: 0; }',
    );
  });

  it("コメントは残す", () => {
    expect(scoped("/* 枠 */\n.box { margin: 0; }")).toBe(
      '/* 枠 */ [data-lesson="ch04-02"] .box { margin: 0; }',
    );
  });
});

describe("lessonSelector", () => {
  it("レッスン ID の属性セレクタを返す", () => {
    expect(lessonSelector("ch04-02")).toBe('[data-lesson="ch04-02"]');
  });

  it("引用符と逆斜線は、エスケープする", () => {
    expect(lessonSelector('a"b\\c')).toBe('[data-lesson="a\\"b\\\\c"]');
  });
});
