// 検査「lint」（動画のコードの禁止事項。documents/spec/content-format/video/prohibited-code.md）のテスト。
// 違反を並べた動画のコードと違反のないコードを、テストの中で一時ディレクトリに作って確かめる
// （リポジトリの中に置くと、アプリの lint と整形の対象になるため）。
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { checkLint } from "./lint.ts";
import type { Finding } from "./types.ts";

const dirs: string[] = [];

afterAll(() => {
  for (const dir of dirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

// lessons/{レッスン}/video/ の下にファイルを作り、検査する。
function lint(lessons: Record<string, Record<string, string>>): Finding[] {
  const contentDir = mkdtempSync(join(tmpdir(), "progrust-video-lint-"));
  dirs.push(contentDir);
  for (const [lesson, files] of Object.entries(lessons)) {
    for (const [name, content] of Object.entries(files)) {
      const path = join(contentDir, "lessons", lesson, "video", name);
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, content);
    }
  }
  return checkLint({ contentDir, curriculumDir: contentDir });
}

// 禁止事項ごとの、違反するコード（1 ファイルに 1 つの違反）と、指摘に含まれる語。
const violations: [string, string, string][] = [
  // 色の直接指定（16 進表記と色の関数は、どの文字列でも）
  ["color/hex.ts", 'export const c = "#fff";', "16 進表記（#fff）"],
  ["color/hex8.ts", 'export const c = "#ff000080";', "16 進表記"],
  [
    "color/hex-template.ts",
    "export const b = `1px solid #00ff00`;",
    "16 進表記",
  ],
  ["color/hex-text.tsx", "export const A = () => <p>#123456</p>;", "16 進表記"],
  [
    "color/hex-svg.tsx",
    'export const A = () => <rect stroke="#000" />;',
    "16 進表記",
  ],
  ["color/rgb.ts", "export const c = `rgb(${1}, 2, 3)`;", "色の関数（rgb()）"],
  ["color/hsl.ts", 'export const c = "hsl(0 0% 0%)";', "色の関数（hsl()）"],
  [
    "color/oklch.ts",
    'export const c = "oklch(70% 0.1 200)";',
    "色の関数（oklch()）",
  ],
  [
    "color/gradient.ts",
    'export const s = { background: "linear-gradient(blue, white)" };',
    "色の名前（blue、white）",
  ],
  // 色の名前（style のプロパティの値、SVG の色の属性）
  [
    "color/named-style.tsx",
    'export const A = () => <div style={{ color: "red" }} />;',
    "色の名前（red）",
  ],
  [
    "color/named-style-other-key.tsx",
    'export const A = () => <div style={{ borderImage: "linear-gradient(red, blue) 1" }} />;',
    "色の名前（red、blue）",
  ],
  [
    "color/named-style-shadow.tsx",
    'export const A = () => <div style={{ boxShadow: "0 0 2px Black" }} />;',
    "色の名前（black）",
  ],
  [
    "color/named-object.ts",
    'export const s = { border: "1px solid blue" };',
    "色の名前（blue）",
  ],
  [
    "color/named-object-kebab.ts",
    'export const s = { "background-color": "white" };',
    "色の名前（white）",
  ],
  [
    "color/named-conditional.tsx",
    'export const A = ({ on }: { on: boolean }) => <div style={{ backgroundColor: on ? "green" : "var(--bg)" }} />;',
    "色の名前（green）",
  ],
  [
    "color/named-fill.tsx",
    'export const A = () => <rect fill="red" />;',
    "色の名前（red）",
  ],
  [
    "color/named-stop.tsx",
    'export const A = () => <stop stopColor="white" />;',
    "色の名前（white）",
  ],
  [
    "color/named-stroke-expr.tsx",
    'export const A = () => <path stroke={"navy"} />;',
    "色の名前（navy）",
  ],
  // Tauri の API
  [
    "tauri/import.ts",
    'import { invoke } from "@tauri-apps/api/core";\nexport { invoke };',
    "@tauri-apps/",
  ],
  ["tauri/import-plugin.ts", 'import "@tauri-apps/plugin-fs";', "@tauri-apps/"],
  ["tauri/export.ts", 'export * from "@tauri-apps/api";', "@tauri-apps/"],
  [
    "tauri/dynamic.ts",
    'export const m = import("@tauri-apps/api/event");',
    "@tauri-apps/",
  ],
  ["tauri/window.ts", "export const t = window.__TAURI__;", "__TAURI__"],
  ["tauri/internals.ts", "export const t = __TAURI_INTERNALS__;", "__TAURI__"],
  [
    "tauri/global-this.ts",
    "export const t = globalThis.__TAURI_INTERNALS__;",
    "__TAURI__",
  ],
  ["tauri/in.ts", 'export const t = "__TAURI__" in window;', "__TAURI__"],
  // ネットワーク
  ["network/fetch.ts", 'fetch("/a");', "ネットワーク"],
  ["network/xhr.ts", "new XMLHttpRequest();", "ネットワーク"],
  ["network/ws.ts", 'new WebSocket("ws://localhost");', "ネットワーク"],
  ["network/sse.ts", 'new EventSource("/e");', "ネットワーク"],
  ["network/beacon.ts", 'navigator.sendBeacon("/b");', "ネットワーク"],
  [
    "network/beacon-window.ts",
    'window.navigator.sendBeacon("/b");',
    "ネットワーク",
  ],
  ["network/window-fetch.ts", 'window.fetch("/a");', "ネットワーク"],
  [
    "network/destructure.ts",
    'const { fetch: f } = window;\nf("/a");',
    "ネットワーク",
  ],
  [
    "network/img.tsx",
    'export const A = () => <img src="https://example.com/a.png" alt="" />;',
    "外部の URL",
  ],
  [
    "network/scheme-relative.tsx",
    'export const A = () => <img src="//cdn.example.com/a.png" alt="" />;',
    "外部の URL",
  ],
  [
    "network/import.ts",
    'import x from "https://cdn.example.com/x.js";\nexport { x };',
    "外部の URL",
  ],
  [
    "network/css-url.tsx",
    'export const A = () => <div style={{ backgroundImage: "url(http://example.com/a.png)" }} />;',
    "外部の URL",
  ],
  [
    "network/template.ts",
    "export const u = `https://example.com/${1}`;",
    "外部の URL",
  ],
  // ファイル
  [
    "file/fs.ts",
    'import fs from "fs";\nexport { fs };',
    "Node.js の組み込みモジュール",
  ],
  ["file/node-path.ts", 'import "node:path";', "Node.js の組み込みモジュール"],
  [
    "file/fs-promises.ts",
    'export { readFile } from "fs/promises";',
    "Node.js の組み込みモジュール",
  ],
  ["file/require.ts", 'export const fs = require("fs");', "require()"],
  ["file/require-local.ts", 'export const a = require("./a");', "require()"],
  ["file/url.ts", 'export const f = "file:///etc/passwd";', "file: の URL"],
  // 検査をすり抜ける書き方
  ["bypass/eval.ts", 'eval("1");', "eval"],
  ["bypass/indirect-eval.ts", '(0, eval)("1");', "eval"],
  [
    "bypass/new-function.ts",
    'export const f = new Function("return 1");',
    "new Function",
  ],
  ["bypass/window-eval.ts", 'window.eval("1");', "eval"],
  [
    "bypass/destructure-function.ts",
    "const { Function: F } = globalThis;\nexport const f = new F();",
    "new Function",
  ],
  [
    "bypass/function-call.ts",
    'export const f = Function("return 1");',
    "new Function",
  ],
  [
    "bypass/dynamic-import.ts",
    'const m = "./a";\nexport const p = import(m);',
    "動的 import",
  ],
  [
    "bypass/dynamic-import-template.ts",
    "export const p = import(`./${1}`);",
    "動的 import",
  ],
  [
    "bypass/window-computed.ts",
    'export const f = window["fetch"];',
    "[...] でのアクセス",
  ],
  [
    "bypass/global-this-computed.ts",
    'const k = "x";\nexport const f = globalThis[k];',
    "[...] でのアクセス",
  ],
  [
    "bypass/self-computed.ts",
    'export const f = self["x"];',
    "[...] でのアクセス",
  ],
  [
    "bypass/navigator-computed.ts",
    'export const f = navigator["sendBeacon"];',
    "[...] でのアクセス",
  ],
  [
    "bypass/as-any.ts",
    "export const t = (window as any).foo;",
    "型アサーション",
  ],
  [
    "bypass/as-unknown.ts",
    "export const t = (navigator as unknown as { x: 1 }).x;",
    "型アサーション",
  ],
  [
    "bypass/angle.ts",
    "export const t = (<any>globalThis).foo;",
    "型アサーション",
  ],
  [
    "bypass/self-as.ts",
    "export const t = (self as any).foo;",
    "型アサーション",
  ],
  // staticFile()
  [
    "static-file.ts",
    'import { staticFile } from "remotion";\nexport const p = staticFile("a.png");',
    "staticFile()",
  ],
  [
    "static-file-namespace.ts",
    'import * as R from "remotion";\nexport const p = R.staticFile("a.png");',
    "staticFile()",
  ],
  // インラインの指定では、検査を止められない
  ["disable.ts", '// eslint-disable-next-line\nfetch("/a");', "ネットワーク"],
  [
    "disable-file.ts",
    '/* eslint-disable */\nexport const c = "#fff";',
    "16 進表記",
  ],
  // 読めないコードも、エラーにする
  ["parse-error.ts", "export const = ;", "Parsing error"],
];

const cssViolations: [string, string, string][] = [
  ["hex.css", ".a { color: #fff; }", "16 進表記（#fff）"],
  ["named.css", ".a { color: red; }", "色の名前（red）"],
  ["named-shorthand.css", ".a { border: 1px solid navy; }", "色の名前（navy）"],
  ["rgb.css", ".a { color: rgb(0 0 0); }", "色の関数（rgb）"],
  ["oklch.css", ".a { color: oklch(70% 0.1 200); }", "色の関数（oklch）"],
  [
    "color-mix.css",
    ".a { color: color-mix(in srgb, var(--a), var(--b)); }",
    "色の関数（color-mix）",
  ],
  [
    "url-https.css",
    ".a { background: url(https://example.com/a.png); }",
    "外部の URL",
  ],
  [
    "url-http.css",
    '.a { background: url("http://example.com/a.png"); }',
    "外部の URL",
  ],
  [
    "url-relative.css",
    ".a { background: url(//cdn.example.com/a.png); }",
    "外部の URL",
  ],
  ["url-file.css", ".a { background: url(file:///a.png); }", "file: の URL"],
  [
    "font.css",
    '@font-face { font-family: "X"; src: url(https://example.com/x.woff2); }',
    "外部の URL",
  ],
  ["import-local.css", '@import "./other.css";', "@import"],
  ["import-url.css", '@import url("https://example.com/x.css");', "@import"],
  ["broken.css", ".a { color: var(--fg);", "CssSyntaxError"],
  ["disable.css", "/* stylelint-disable */\n.a { color: #fff; }", "16 進表記"],
];

const clean = `
// eslint-disable-next-line -- 指定は効かないが、それだけではエラーにならない
import { useCurrentFrame } from "remotion";
import "./scene.css";
import dot from "./dot.png";
import data from "./data.json";
import { Panel } from "./parts/Panel";

// 使わない変数や any も、禁止事項でなければエラーにしない。
const unused: any = 1;
let callback: Function | undefined;

export const lazy = () => import("./parts/Panel");

// window などでないオブジェクトの fetch・eval は、ネットワークへのアクセスや eval ではない。
const api = { fetch: () => 1, eval: () => 2 };
const { fetch: get } = api;
export const results = [api.fetch(), api.eval(), get()];

const box = {
  color: "var(--fg)",
  backgroundColor: "transparent",
  borderColor: "currentColor",
  fill: "none",
  stroke: "inherit",
  outline: "1px solid var(--accent, var(--fg))",
  backgroundImage: "url(./a.png)",
};

export const Scene = () => {
  const frame = useCurrentFrame();
  const label = "赤は red、青は blue と書く";
  const width = window.innerWidth + navigator.language.length;
  return (
    <div style={{ ...box, opacity: frame / 30, fontFamily: "'Red Hat Mono'" }}>
      <p title="white paper">red と blue の説明 #1 番。{label}{width}{data.title}</p>
      <svg role="img" aria-label="図">
        <rect fill="url(#bad)" stroke="var(--accent)" />
        <circle fill="currentColor" stroke="none" />
      </svg>
      <img src={dot} alt="点" />
      <a href="./page.html">リンク</a>
      <Panel />
    </div>
  );
};
`;

const cleanCss = `
/* stylelint-disable */
.scene {
  color: var(--fg);
  background: transparent;
  border: 1px solid currentcolor;
  fill: none;
  stroke: inherit;
  background-image: url(./a.png);
}

.scene .mark {
  outline: 2px solid var(--accent);
}
`;

describe("違反を並べた動画のコード", () => {
  let findings: Finding[] = [];

  beforeAll(() => {
    findings = lint({
      ch01: Object.fromEntries(
        [...violations, ...cssViolations].map(([name, code]) => [name, code]),
      ),
    });
  });

  function messagesOf(name: string): string[] {
    return findings
      .filter((f) => f.file === `lessons/ch01/video/${name}`)
      .map((f) => f.message);
  }

  it.each(violations)(
    "%s（.ts・.tsx）は、ESLint でエラーになる",
    (name, _code, expected) => {
      const messages = messagesOf(name);
      expect(messages.length, JSON.stringify(findings)).toBeGreaterThan(0);
      expect(messages.join("\n")).toContain(expected);
    },
  );

  it.each(cssViolations)(
    "%s（.css）は、stylelint でエラーになる",
    (name, _code, expected) => {
      const messages = messagesOf(name);
      expect(messages.length, JSON.stringify(findings)).toBeGreaterThan(0);
      expect(messages.join("\n")).toContain(expected);
    },
  );

  it("指摘は、項目「lint」のエラーとして、レッスンとファイルと位置を示す", () => {
    expect(findings.every((f) => f.severity === "error")).toBe(true);
    expect(findings.every((f) => f.check === "lint")).toBe(true);
    expect(findings.every((f) => f.lesson === "ch01")).toBe(true);
    expect(messagesOf("color/hex.ts")).toEqual([
      expect.stringMatching(/^1:18 色を 16 進表記（#fff）で直接書いている/),
    ]);
    expect(messagesOf("hex.css")).toEqual([
      expect.stringMatching(/^1:13 色を 16 進表記（#fff）で直接書いている/),
    ]);
  });

  it("lint を止める指定があっても、その後ろの違反だけがエラーになる", () => {
    expect(messagesOf("disable.ts")).toEqual([
      expect.stringMatching(/^2:1 .*ネットワーク/),
    ]);
    expect(messagesOf("disable.css")).toEqual([
      expect.stringMatching(/^2:13 色を 16 進表記/),
    ]);
  });
});

describe("違反のない動画のコード", () => {
  it("エラーにならない", () => {
    expect(
      lint({
        ch01: {
          "index.tsx": clean,
          "parts/Panel.tsx":
            'export const Panel = () => <div className="callout" />;',
          "types.d.ts":
            'declare module "*.png" { const src: string; export default src; }',
          "scene.css": cleanCss,
        },
      }),
    ).toEqual([]);
  });

  it("動画のコードが無ければ、エラーにならない", () => {
    expect(lint({})).toEqual([]);
    expect(lint({ ch01: { "script.json": '{"text": "#fff"}' } })).toEqual([]);
  });
});

describe("検査の対象", () => {
  it("すべてのレッスンの video/ の、下のディレクトリのファイルも検査する", () => {
    const findings = lint({
      ch01: { "index.tsx": clean },
      ch02: { "parts/deep/Box.tsx": 'export const c = "#fff";' },
      ch03: { "styles/a.css": ".a { color: red; }" },
    });
    expect(findings).toEqual([
      expect.objectContaining({
        file: "lessons/ch02/video/parts/deep/Box.tsx",
        lesson: "ch02",
      }),
      expect.objectContaining({
        file: "lessons/ch03/video/styles/a.css",
        lesson: "ch03",
      }),
    ]);
  });
});
