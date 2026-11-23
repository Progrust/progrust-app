// 検査「lint」（documents/spec/content-guidelines/check-tool.md）。
// 動画のコード（content/lessons/*/video/ の .tsx・.ts・.css）が、
// 禁止事項（documents/spec/content-format/video/prohibited-code.md）に違反していないことを確かめる。
// .tsx・.ts は ESLint で、.css は stylelint で検査する。
import { spawnSync } from "node:child_process";
import { readdirSync, readFileSync, realpathSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { Linter } from "eslint";
import { isDirectory } from "./lesson-files.ts";
import type { Check, CheckOptions, Finding } from "./types.ts";
import { videoEslintConfig } from "./video-eslint.ts";
import type { VideoStylelintResult } from "./video-stylelint.ts";

const CHECK = "lint";
const STYLELINT_WORKER = fileURLToPath(
  new URL("./video-stylelint.ts", import.meta.url),
);

interface VideoFile {
  // 絶対パス。
  path: string;
  // 教材のディレクトリからの相対パス（lessons/{lesson_id}/video/…）。
  rel: string;
  lesson: string;
}

function listFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true })
    .sort((a, b) => a.name.localeCompare(b.name))
    .flatMap((d) => {
      const path = join(dir, d.name);
      if (d.isDirectory()) return listFiles(path);
      return d.isFile() ? [path] : [];
    });
}

// content/lessons/*/video/ の中の、すべての .tsx・.ts・.css。
export function videoCodeFiles(contentDir: string): VideoFile[] {
  const lessonsDir = join(contentDir, "lessons");
  if (!isDirectory(lessonsDir)) return [];
  return readdirSync(lessonsDir)
    .sort()
    .flatMap((lesson) => {
      const videoDir = join(lessonsDir, lesson, "video");
      if (!isDirectory(videoDir)) return [];
      return listFiles(videoDir)
        .filter((path) => /\.(?:tsx|ts|css)$/.test(path))
        .map((path) => ({
          path,
          rel: relative(contentDir, path).split(sep).join("/"),
          lesson,
        }));
    });
}

function error(file: VideoFile, message: string): Finding {
  return {
    severity: "error",
    check: CHECK,
    file: file.rel,
    lesson: file.lesson,
    message,
  };
}

function position(line: number, column: number): string {
  return line > 0 ? `${line}:${column} ` : "";
}

function isReported(m: Linter.LintMessage): boolean {
  return !(
    m.ruleId === null &&
    !m.fatal &&
    m.message.includes("'noInlineConfig'")
  );
}

function eslintFindings(contentDir: string, files: VideoFile[]): Finding[] {
  if (files.length === 0) return [];
  // 設定の files は cwd からの相対で照合する。一時ディレクトリが symlink の下にあっても
  // 照合できるように、実際のパスにそろえる。
  const cwd = realpathSync(contentDir);
  const linter = new Linter({ cwd });
  return files.flatMap((file) => {
    const messages = linter.verify(
      readFileSync(file.path, "utf8"),
      videoEslintConfig,
      join(cwd, file.rel),
    );
    // 設定の規則はすべてエラーにしている。それ以外の知らせ（設定が当たらないファイルなど）も、
    // 見落とさないように、エラーとして表示する。ただし、lint を止める指定（eslint-disable など）が
    // 効かないという知らせは、違反ではないので表示しない（ADR 0190）。
    return messages
      .filter(isReported)
      .map((m) =>
        error(
          file,
          `${position(m.line, m.column)}${m.message}${m.ruleId ? `（${m.ruleId}）` : ""}`,
        ),
      );
  });
}

function stylelintFindings(contentDir: string, files: VideoFile[]): Finding[] {
  if (files.length === 0) return [];
  const cwd = resolve(contentDir);
  const result = spawnSync(
    process.execPath,
    [STYLELINT_WORKER, cwd, ...files.map((f) => resolve(f.path))],
    { encoding: "utf8", cwd },
  );
  let parsed: VideoStylelintResult;
  try {
    if (result.status !== 0) throw new Error();
    parsed = JSON.parse(result.stdout) as VideoStylelintResult;
  } catch {
    const detail = (result.stderr || String(result.error ?? "")).trim();
    return files.map((file) =>
      error(file, `stylelint を実行できなかった: ${detail.slice(0, 500)}`),
    );
  }
  const byPath = new Map(files.map((f) => [realpathSync(f.path), f]));
  return parsed.map((m) => {
    const file = byPath.get(realpathSync(m.file)) ?? files[0];
    return error(file, `${position(m.line, m.column)}${m.text}`);
  });
}

export function checkLint({ contentDir }: CheckOptions): Finding[] {
  const files = videoCodeFiles(contentDir);
  return [
    ...eslintFindings(
      contentDir,
      files.filter((f) => !f.path.endsWith(".css")),
    ),
    ...stylelintFindings(
      contentDir,
      files.filter((f) => f.path.endsWith(".css")),
    ),
  ];
}

export const lintCheck: Check = { name: CHECK, run: checkLint };
