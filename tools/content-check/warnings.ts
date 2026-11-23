// 警告の項目「動画の長さ」「文の長さ」「読み」（documents/spec/content-guidelines/check-tool.md）。
// 警告は結果に表示するだけで、エラーにしない。台本や音声の記録が壊れているときは、
// 「台本」「音声」がエラーにするので、ここでは何も言わない。
import { join } from "node:path";
import { computeTimeline, FPS } from "../../src/video/timeline.ts";
import type { Durations, CueReference } from "../../src/video/timeline.ts";
import { validateScript } from "./script.ts";
import type { Check, CheckOptions, Finding } from "./types.ts";
import {
  codePointLength,
  isRecord,
  readJsonFile,
  readSentences,
  readVideoLessons,
  videoPath,
  videoRel,
} from "./video-files.ts";

// 動画の長さの範囲（秒）。
const MIN_SECONDS = 60;
const MAX_SECONDS = 300;
// text の文字数の上限。
const TEXT_MAX = 60;

function warning(
  check: string,
  file: string,
  lesson: string | undefined,
  message: string,
): Finding {
  return { severity: "warning", check, file, lesson, message };
}

// 読みの対応表（content/readings.json）。無いときは空。
function readReadings(contentDir: string, out: Finding[]): [string, string][] {
  const result = readJsonFile(join(contentDir, "readings.json"));
  if (result.state === "missing") return [];
  if (result.state === "invalid" || !isRecord(result.value)) {
    out.push(
      warning("読み", "readings.json", undefined, "readings.json を読めない"),
    );
    return [];
  }
  return Object.entries(result.value).flatMap(([word, reading]) =>
    word !== "" && typeof reading === "string" ? [[word, reading]] : [],
  );
}

export function checkWarnings({ contentDir }: CheckOptions): Finding[] {
  const out: Finding[] = [];
  const readings = readReadings(contentDir, out);

  for (const entry of readVideoLessons(contentDir)) {
    const scriptFile = videoRel(entry, "script.json");
    const script = readJsonFile(videoPath(entry, "script.json"));
    if (script.state !== "ok") continue;
    const sentences = readSentences(script.value);

    for (const s of sentences) {
      const length = codePointLength(s.text);
      if (length > TEXT_MAX) {
        out.push(
          warning(
            "文の長さ",
            scriptFile,
            entry.id,
            `文 ${s.id} の text が ${length} 字で、${TEXT_MAX} 字を超えている`,
          ),
        );
      }
      for (const [word, reading] of readings) {
        if (s.say.includes(word)) {
          out.push(
            warning(
              "読み",
              scriptFile,
              entry.id,
              `文 ${s.id} の say に、readings.json の語 ${word} がそのまま残っている（読みは ${reading}）`,
            ),
          );
        }
      }
    }

    // 動画の長さ。台本と音声の記録が正しいときだけ求める。
    if (validateScript(script.value).length > 0) continue;
    const durations = readJsonFile(videoPath(entry, "durations.json"));
    if (durations.state !== "ok" || !isRecord(durations.value)) continue;
    const records = durations.value;
    const complete = sentences.every((s) => {
      const record = records[s.id];
      return (
        isRecord(record) &&
        typeof record.seconds === "number" &&
        Number.isFinite(record.seconds) &&
        record.seconds > 0
      );
    });
    if (!complete) continue;
    const reference = readJsonFile(videoPath(entry, "cue-reference.json"));
    const cueReference =
      reference.state === "ok" && isRecord(reference.value)
        ? (reference.value as CueReference)
        : {};
    try {
      const timeline = computeTimeline(
        script.value as Parameters<typeof computeTimeline>[0],
        durations.value as Durations,
        cueReference,
      );
      const seconds = timeline.durationInFrames / FPS;
      if (seconds < MIN_SECONDS || seconds > MAX_SECONDS) {
        out.push(
          warning(
            "動画の長さ",
            scriptFile,
            entry.id,
            `動画の長さが ${seconds.toFixed(1)} 秒で、${MIN_SECONDS}〜${MAX_SECONDS} 秒（1〜5 分）の範囲にない`,
          ),
        );
      }
    } catch {
      // 台本と音声の記録の食い違いは、「台本」「音声」が報告する。
    }
  }
  return out;
}

export const warningsCheck: Check = { name: "警告", run: checkWarnings };
