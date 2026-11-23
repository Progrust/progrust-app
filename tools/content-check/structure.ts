// 検査「構成」（documents/spec/content-guidelines/check-tool.md）。
// 準備中のレッスン（ディレクトリが無いレッスン）は検査しない。
// 演習のファイルの中身と、台本・音声の中身の検査は、ほかの項目が行う。
import { readFileSync } from "node:fs";
import { join } from "node:path";
import {
  isDirectory,
  isFile,
  isPrepared,
  readLessonEntries,
  readLessonJson,
  type LessonEntry,
} from "./lesson-files.ts";
import type { Check, CheckOptions, Finding } from "./types.ts";

const NAME = "構成";

// 台本の文の ID。台本の形の検査は「台本」の項目が行うので、読めなければ空にする。
function sentenceIds(scriptPath: string): string[] {
  try {
    const script = JSON.parse(readFileSync(scriptPath, "utf8")) as {
      sentences?: unknown;
    };
    if (!Array.isArray(script.sentences)) return [];
    return script.sentences.flatMap((s: { id?: unknown } | null) =>
      typeof s?.id === "string" ? [s.id] : [],
    );
  } catch {
    return [];
  }
}

function checkLesson(entry: LessonEntry): Finding[] {
  const out: Finding[] = [];
  const error = (file: string, message: string) =>
    out.push({
      severity: "error",
      check: NAME,
      file: `${entry.rel}/${file}`,
      lesson: entry.id,
      message,
    });
  const needFile = (file: string) => {
    if (!isFile(join(entry.dir, file))) error(file, `${file} がない`);
  };

  const lesson = readLessonJson(entry);
  if (lesson.state === "missing") error("lesson.json", "lesson.json がない");
  const kind =
    lesson.state === "ok" && typeof lesson.value.kind === "string"
      ? lesson.value.kind
      : undefined;

  const isReview =
    kind === undefined
      ? entry.id.endsWith("-review")
      : kind === "chapter_review";
  if (!isReview) {
    needFile("video/script.json");
    needFile("video/index.tsx");
    needFile("video/durations.json");
    for (const id of sentenceIds(join(entry.dir, "video/script.json"))) {
      needFile(`video/voice/${id}.m4a`);
    }
  }

  if (kind === "video_only") {
    if (existsAnything(join(entry.dir, "exercise"))) {
      error("exercise", "動画のみのレッスンに exercise/ がある");
    }
  } else if (kind !== undefined) {
    if (!isDirectory(join(entry.dir, "exercise"))) {
      error("exercise", "exercise/ がない");
    } else {
      for (const file of [
        "exercise/problem.md",
        "exercise/explanation.md",
        "exercise/starter/src/main.rs",
        "exercise/solution/src/main.rs",
      ]) {
        needFile(file);
      }
      const exercise =
        lesson.state === "ok" ? lesson.value.exercise : undefined;
      const judge =
        typeof exercise === "object" && exercise !== null
          ? (exercise as { judge?: { mode?: unknown } }).judge
          : undefined;
      const mode =
        typeof judge === "object" && judge !== null ? judge.mode : undefined;
      if (mode === "stdout") needFile("exercise/cases.json");
      if (mode === "test") needFile("exercise/judge_tests.rs");
    }
  }
  return out;
}

function existsAnything(path: string): boolean {
  return isFile(path) || isDirectory(path);
}

export function checkStructure({ contentDir }: CheckOptions): Finding[] {
  return readLessonEntries(contentDir)
    .filter((entry) => !isPrepared(entry))
    .flatMap(checkLesson);
}

export const structureCheck: Check = { name: NAME, run: checkStructure };
