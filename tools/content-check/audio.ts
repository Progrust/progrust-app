// 検査「音声」（documents/spec/content-guidelines/check-tool.md）。
// durations.json の say・voice が、台本の say と content/voice.json と同じであることと、
// 台本にない文の音声ファイルと記録がないことを確かめる。
// すべての文に音声ファイルがあることは「構成」が報告する。
// 準備中のレッスンと章末復習レッスンは検査しない。
import { readdirSync } from "node:fs";
import { join } from "node:path";
import type { LessonEntry } from "./lesson-files.ts";
import type { Check, CheckOptions, Finding } from "./types.ts";
import {
  isRecord,
  readJsonFile,
  readSentences,
  readVideoLessons,
  videoPath,
  videoRel,
} from "./video-files.ts";

const NAME = "音声";

function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (Array.isArray(a) || Array.isArray(b)) {
    return (
      Array.isArray(a) &&
      Array.isArray(b) &&
      a.length === b.length &&
      a.every((v, i) => deepEqual(v, b[i]))
    );
  }
  if (isRecord(a) && isRecord(b)) {
    const keys = Object.keys(a);
    return (
      keys.length === Object.keys(b).length &&
      keys.every((k) => k in b && deepEqual(a[k], b[k]))
    );
  }
  return false;
}

function voiceFileIds(dir: string): string[] {
  try {
    return readdirSync(dir)
      .filter((name) => name.endsWith(".m4a"))
      .map((name) => name.slice(0, -".m4a".length));
  } catch {
    return [];
  }
}

export function checkAudio({ contentDir }: CheckOptions): Finding[] {
  const out: Finding[] = [];
  const voice = readJsonFile(join(contentDir, "voice.json"));
  let voiceReported = false;
  // 声の設定は、durations.json と照合するときに、1 度だけ確かめる。
  const reportVoice = () => {
    if (voiceReported) return;
    voiceReported = true;
    if (voice.state === "missing") {
      out.push({
        severity: "error",
        check: NAME,
        file: "voice.json",
        message: "voice.json がない",
      });
    } else if (voice.state === "invalid") {
      out.push({
        severity: "error",
        check: NAME,
        file: "voice.json",
        message: `voice.json を JSON として読めない（${voice.message}）`,
      });
    }
  };

  const checkLesson = (entry: LessonEntry) => {
    const error = (file: string, message: string) =>
      out.push({
        severity: "error",
        check: NAME,
        file: videoRel(entry, file),
        lesson: entry.id,
        message,
      });

    const script = readJsonFile(videoPath(entry, "script.json"));
    if (script.state !== "ok") return;
    const sentences = readSentences(script.value);
    const ids = new Set(sentences.map((s) => s.id));

    for (const id of voiceFileIds(videoPath(entry, "voice"))) {
      if (!ids.has(id)) {
        error(`voice/${id}.m4a`, `台本にない文 ${id} の音声ファイルがある`);
      }
    }

    const durations = readJsonFile(videoPath(entry, "durations.json"));
    if (durations.state === "invalid") {
      error(
        "durations.json",
        `durations.json を JSON として読めない（${durations.message}）`,
      );
    } else if (durations.state === "ok") {
      if (!isRecord(durations.value)) {
        error("durations.json", "durations.json がオブジェクトでない");
      } else {
        if (sentences.length > 0) reportVoice();
        for (const sentence of sentences) {
          const record = durations.value[sentence.id];
          if (record === undefined) {
            error(
              "durations.json",
              `文 ${sentence.id} の記録がない（音声を作り直していない）`,
            );
            continue;
          }
          if (!isRecord(record)) {
            error(
              "durations.json",
              `文 ${sentence.id} の記録がオブジェクトでない`,
            );
            continue;
          }
          if (
            typeof record.seconds !== "number" ||
            !Number.isFinite(record.seconds) ||
            record.seconds <= 0
          ) {
            error(
              "durations.json",
              `文 ${sentence.id} の seconds が正の数でない`,
            );
          }
          if (record.say !== sentence.say) {
            error(
              "durations.json",
              `文 ${sentence.id} の say が台本の say と違う（音声を作り直していない）`,
            );
          }
          if (voice.state === "ok" && !deepEqual(record.voice, voice.value)) {
            error(
              "durations.json",
              `文 ${sentence.id} の voice が voice.json と違う（音声を作り直していない）`,
            );
          }
        }
        for (const id of Object.keys(durations.value)) {
          if (!ids.has(id)) {
            error("durations.json", `台本にない文 ${id} の記録がある`);
          }
        }
      }
    }

    const reference = readJsonFile(videoPath(entry, "cue-reference.json"));
    if (reference.state === "invalid") {
      error(
        "cue-reference.json",
        `cue-reference.json を JSON として読めない（${reference.message}）`,
      );
    } else if (reference.state === "ok" && isRecord(reference.value)) {
      for (const id of Object.keys(reference.value)) {
        if (!ids.has(id)) {
          error("cue-reference.json", `台本にない文 ${id} の記録がある`);
        }
      }
    }
  };

  readVideoLessons(contentDir).forEach(checkLesson);
  return out;
}

export const audioCheck: Check = { name: NAME, run: checkAudio };
