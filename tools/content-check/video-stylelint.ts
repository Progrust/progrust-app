// 動画のコード（`.css`）の禁止事項を stylelint で検査する
// （documents/spec/content-format/video/prohibited-code.md）。
// stylelint は非同期でしか呼べないので、教材チェックツールは、このファイルを別のプロセスで実行する。
// 引数: <教材のディレクトリ> <検査する .css のパス>...
// 出力: 指摘の配列（VideoStylelintResult）の JSON。
import stylelint from "stylelint";

export interface VideoStylelintMessage {
  // 検査したファイルの絶対パス。
  file: string;
  line: number;
  column: number;
  text: string;
}

export type VideoStylelintResult = VideoStylelintMessage[];

const TOKEN = "動画のデザインシステムのトークン（CSS 変数）を使う";

// 色の関数。
const COLOR_FUNCTIONS = [
  "rgb",
  "rgba",
  "hsl",
  "hsla",
  "hwb",
  "lab",
  "lch",
  "oklab",
  "oklch",
  "color",
  "color-mix",
  "device-cmyk",
  "light-dark",
];

// 禁止事項だけを検査する。インラインの指定（stylelint-disable など）は効かない。
export const videoStylelintConfig: stylelint.Config = {
  rules: {
    "color-no-hex": [
      true,
      { message: `色を 16 進表記（%s）で直接書いている。${TOKEN}` },
    ],
    "color-named": [
      "never",
      { message: `色を色の名前（%s）で直接書いている。${TOKEN}` },
    ],
    "function-disallowed-list": [
      COLOR_FUNCTIONS,
      { message: `色を色の関数（%s）で直接書いている。${TOKEN}` },
    ],
    "function-url-scheme-disallowed-list": [
      ["http", "https", "file"],
      { message: "外部の URL・file: の URL（%s:）を参照している" },
    ],
    "function-url-no-scheme-relative": [
      true,
      { message: "外部の URL（// で始まるもの）を参照している" },
    ],
    "at-rule-disallowed-list": [
      ["import"],
      { message: "CSS の @import は使えない。CSS は TSX から import する" },
    ],
  },
};

async function main(contentDir: string, files: string[]): Promise<void> {
  const { results } = await stylelint.lint({
    files,
    config: videoStylelintConfig,
    cwd: contentDir,
    ignoreDisables: true,
  });
  const out: VideoStylelintResult = [];
  for (const result of results) {
    const file = result.source ?? "";
    for (const w of result.warnings) {
      out.push({ file, line: w.line, column: w.column, text: w.text });
    }
    for (const w of result.invalidOptionWarnings) {
      out.push({ file, line: 0, column: 0, text: w.text });
    }
  }
  process.stdout.write(JSON.stringify(out));
}

if (import.meta.main) {
  const [contentDir, ...files] = process.argv.slice(2);
  await main(contentDir, files);
}
