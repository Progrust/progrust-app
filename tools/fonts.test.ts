import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { expect, it } from "vitest";

const here = (path: string) => new URL(path, import.meta.url);
const css = readFileSync(here("../src/fonts/fonts.css"), "utf8");

// トークン（--font-sans / --font-mono）の先頭の書体名を取り出す。
function firstFamily(tokensCss: string, name: string): string {
  const m = new RegExp(`${name}:\\s*"([^"]+)"`).exec(tokensCss);
  if (m === null) throw new Error(`${name} が tokens.css に無い`);
  return m[1];
}

const tokenFiles = [
  "../design-system/progrust/tokens.css",
  "../design-system/progrust-video/tokens.css",
];

it.each(tokenFiles)("%s の書体名に @font-face が対応する", (file) => {
  const tokens = readFileSync(here(file), "utf8");
  for (const name of ["--font-sans", "--font-mono"]) {
    const family = firstFamily(tokens, name);
    const names = [`font-family: ${family};`, `font-family: "${family}";`];
    expect(names.some((n) => css.includes(n))).toBe(true);
  }
});

it("@font-face が指すファイルがすべて存在する", () => {
  const urls = [...css.matchAll(/url\("\.\/([^"]+)"\)/g)].map((m) => m[1]);
  expect(urls).toHaveLength(2);
  for (const url of urls) {
    expect(() =>
      readFileSync(here(`../src/fonts/${decodeURI(url)}`)),
    ).not.toThrow();
  }
});

it("main.tsx が fonts.css を読み込む", () => {
  const main = readFileSync(here("../src/main.tsx"), "utf8");
  expect(main).toContain('import "./fonts/fonts.css";');
});

it.each([
  ["murecho/Murecho[wght].ttf", "150ce1d5b141bb6eace70e61ce0c3c26de3475f3"],
  ["murecho/OFL.txt", "925c52898c469117dc14c175a4a16d0de6ca88be"],
  [
    "red-hat-mono/RedHatMono-VF.ttf",
    "7b0ccc62f3dc907be3ef60599770adb3f75aefbe",
  ],
  ["red-hat-mono/OFL.txt", "9e0e0c6b6ab30607ca07516f50fc2919f276419b"],
])("%s が配布元のファイルと同じ内容（git の blob ハッシュ）", (file, blob) => {
  const data = readFileSync(here(`../src/fonts/${file}`));
  const hash = createHash("sha1")
    .update(`blob ${data.length}\0`)
    .update(data)
    .digest("hex");
  expect(hash).toBe(blob);
});
