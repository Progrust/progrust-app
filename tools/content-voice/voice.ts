// 音声を作るツール（documents/spec/content-format/video/voice.md「音声を作るツール」）の本体。
// VOICEPEAK・ffmpeg・ffprobe を呼ぶところは VoiceTools に分け、テストでは置き換える。
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  readLessonEntries,
  type LessonEntry,
} from "../content-check/lesson-files.ts";
import { validateScript } from "../content-check/script.ts";
import {
  deepEqual,
  isRecord,
  readJsonFile,
  readSentences,
  readVideoLessons,
  videoPath,
  videoRel,
  type ScriptSentence,
} from "../content-check/video-files.ts";
import { trimCommaPauses } from "./silence.ts";
import { parseWav, serializeWav } from "./wav.ts";

// 声の設定（content/voice.json）。
export interface VoiceSettings {
  narrator: string;
  speed: number;
  pause: number;
  emotion: Record<string, number>;
}

// VOICEPEAK の CLI の速さの範囲。
const SPEED_MIN = 50;
const SPEED_MAX = 200;
const VOICE_KEYS = ["narrator", "speed", "pause", "emotion"];

// 声の設定の形を確かめる。違反があれば、そのメッセージを返す。
export function validateVoice(value: unknown): VoiceSettings | string[] {
  if (!isRecord(value)) return ["voice.json がオブジェクトでない"];
  const out: string[] = [];
  for (const key of Object.keys(value)) {
    if (!VOICE_KEYS.includes(key))
      out.push(`voice.json に知らない項目 ${key} がある`);
  }
  if (typeof value.narrator !== "string" || value.narrator === "") {
    out.push("voice.json の narrator が空か、文字列でない");
  }
  if (
    typeof value.speed !== "number" ||
    !(value.speed >= SPEED_MIN && value.speed <= SPEED_MAX)
  ) {
    out.push(`voice.json の speed が ${SPEED_MIN}〜${SPEED_MAX} の数でない`);
  }
  if (
    typeof value.pause !== "number" ||
    !Number.isFinite(value.pause) ||
    value.pause < 0
  ) {
    out.push("voice.json の pause が 0 以上の数でない");
  }
  if (
    !isRecord(value.emotion) ||
    !Object.values(value.emotion).every(
      (v) => typeof v === "number" && Number.isFinite(v),
    )
  ) {
    out.push("voice.json の emotion が、項目名と数の組でない");
  }
  return out.length > 0 ? out : (value as unknown as VoiceSettings);
}

// VOICEPEAK・ffmpeg・ffprobe を呼ぶ処理。失敗したときは例外を投げる。
export interface VoiceTools {
  // `say` を声の設定で読み上げた音声を、WAV で `wavPath` に書く。
  synthesize(say: string, voice: VoiceSettings, wavPath: string): void;
  // WAV を `.m4a` に変換する。
  encode(wavPath: string, m4aPath: string): void;
  // `.m4a` の長さ（秒）を測る。
  measureSeconds(m4aPath: string): number;
}

export interface VoiceOptions {
  contentDir: string;
  // 対象のレッスンの ID。空なら、動画のあるすべてのレッスン。
  lessons: string[];
  // すべての文を作り直す。
  all: boolean;
  // cue-reference.json の値を、いまの音声の長さに書き換える文の ID。対象のレッスンが 1 つのときだけ使える。
  cues: string[];
  log?: (line: string) => void;
  error?: (line: string) => void;
}

interface DurationRecord {
  seconds: number;
  say: string;
  voice: unknown;
}

interface LessonPlan {
  entry: LessonEntry;
  sentences: ScriptSentence[];
  durations: Record<string, unknown>;
  cueReference: Record<string, unknown>;
}

// 作り直す文。`durations.json` の say・voice が台本の say・声の設定と同じで、音声ファイルがある文は、作り直さない。
export function selectSentences(
  sentences: ScriptSentence[],
  durations: Record<string, unknown>,
  voice: unknown,
  hasVoiceFile: (id: string) => boolean,
  all: boolean,
): ScriptSentence[] {
  if (all) return sentences;
  return sentences.filter((s) => {
    const record = durations[s.id];
    return !(
      isRecord(record) &&
      record.say === s.say &&
      deepEqual(record.voice, voice) &&
      hasVoiceFile(s.id)
    );
  });
}

// 台本の文の順に並べ、台本にない文の記録を除く。
function ordered<T>(
  sentences: ScriptSentence[],
  record: Record<string, T>,
): Record<string, T> {
  const out: Record<string, T> = {};
  for (const s of sentences) {
    if (s.id in record) out[s.id] = record[s.id];
  }
  return out;
}

function readRecordFile(
  entry: LessonEntry,
  file: string,
  errors: string[],
): Record<string, unknown> {
  const result = readJsonFile(videoPath(entry, file));
  if (result.state === "missing") return {};
  if (result.state === "invalid") {
    errors.push(
      `${videoRel(entry, file)}: JSON として読めない（${result.message}）。直すか、消してから実行する`,
    );
    return {};
  }
  if (!isRecord(result.value)) {
    errors.push(`${videoRel(entry, file)}: オブジェクトでない`);
    return {};
  }
  return result.value;
}

// 音声を作る前に、声の設定・対象のレッスン・台本・指定を確かめる。
// 声の設定・対象のレッスン・指定のエラー（errors）が 1 件でもあれば、どの音声も作らない。
// 台本などのエラー（lessonErrors）があるレッスンは、そのレッスンの音声だけを作らない。
function prepare(options: VoiceOptions): {
  voice?: VoiceSettings;
  voiceValue?: unknown;
  plans: LessonPlan[];
  errors: string[];
  lessonErrors: string[];
} {
  const errors: string[] = [];
  const lessonErrors: string[] = [];
  const voiceFile = readJsonFile(join(options.contentDir, "voice.json"));
  let voice: VoiceSettings | undefined;
  let voiceValue: unknown;
  if (voiceFile.state === "missing") {
    errors.push("voice.json がない");
  } else if (voiceFile.state === "invalid") {
    errors.push(`voice.json を JSON として読めない（${voiceFile.message}）`);
  } else {
    const checked = validateVoice(voiceFile.value);
    if (Array.isArray(checked)) errors.push(...checked);
    else {
      voice = checked;
      voiceValue = voiceFile.value;
    }
  }

  const videoLessons = readVideoLessons(options.contentDir);
  let entries: LessonEntry[];
  if (options.lessons.length === 0) {
    entries = videoLessons;
  } else {
    const known = new Set(
      readLessonEntries(options.contentDir).map((e) => e.id),
    );
    entries = [];
    for (const id of new Set(options.lessons)) {
      const entry = videoLessons.find((e) => e.id === id);
      if (entry !== undefined) entries.push(entry);
      else if (known.has(id)) {
        errors.push(`レッスン ${id} には動画がない（準備中か、章末復習）`);
      } else errors.push(`レッスン ${id} が chapters.json にない`);
    }
  }

  if (options.cues.length > 0 && options.lessons.length !== 1) {
    errors.push("--cue は、--lesson を 1 つだけ書いたときに使える");
  }

  const plans: LessonPlan[] = [];
  for (const entry of entries) {
    const script = readJsonFile(videoPath(entry, "script.json"));
    const rel = videoRel(entry, "script.json");
    const skip = `。レッスン ${entry.id} の音声を作らない`;
    if (script.state === "missing") {
      lessonErrors.push(`${rel}: ない${skip}`);
      continue;
    }
    if (script.state === "invalid") {
      lessonErrors.push(
        `${rel}: JSON として読めない（${script.message}）${skip}`,
      );
      continue;
    }
    const scriptErrors = validateScript(script.value);
    if (scriptErrors.length > 0) {
      lessonErrors.push(...scriptErrors.map((m) => `${rel}: ${m}${skip}`));
      continue;
    }
    const sentences = readSentences(script.value);
    const fileErrors: string[] = [];
    const durations = readRecordFile(entry, "durations.json", fileErrors);
    const cueReference = readRecordFile(
      entry,
      "cue-reference.json",
      fileErrors,
    );
    if (fileErrors.length > 0) {
      lessonErrors.push(...fileErrors.map((m) => `${m}${skip}`));
      continue;
    }
    if (options.lessons.length === 1) {
      const ids = new Set(sentences.map((s) => s.id));
      for (const id of new Set(options.cues)) {
        if (!ids.has(id)) errors.push(`--cue の文 ${id} が、${rel} にない`);
      }
    }
    plans.push({ entry, sentences, durations, cueReference });
  }
  return { voice, voiceValue, plans, errors, lessonErrors };
}

function writeJsonIfChanged(path: string, value: unknown): boolean {
  const text = `${JSON.stringify(value, null, 2)}\n`;
  if (existsSync(path) && readFileSync(path, "utf8") === text) return false;
  writeFileSync(path, text);
  return true;
}

export interface VoiceResult {
  exitCode: number;
  // 作った音声の文（`{レッスンの ID}/{文の ID}`）。
  generated: string[];
  // 消した音声ファイル（教材のディレクトリからの相対パス）。
  removed: string[];
  errors: string[];
}

export function generateVoices(
  options: VoiceOptions,
  tools: VoiceTools,
): VoiceResult {
  const log = options.log ?? console.log;
  const error = options.error ?? console.error;
  const generated: string[] = [];
  const removed: string[] = [];
  const prepared = prepare(options);
  const { voice, voiceValue, plans } = prepared;
  const errors = [...prepared.errors, ...prepared.lessonErrors];
  const fail = (note?: string): VoiceResult => {
    for (const line of errors) error(`エラー: ${line}`);
    error(
      `エラー ${errors.length} 件。作った音声 ${generated.length} 件${note ?? ""}`,
    );
    return { exitCode: 1, generated, removed, errors };
  };
  if (prepared.errors.length > 0 || voice === undefined) {
    return fail("。どの音声も作らずに終わった");
  }

  const work = mkdtempSync(join(tmpdir(), "progrust-content-voice-"));
  try {
    for (const plan of plans) {
      processLesson(plan, voice);
    }
  } catch (e) {
    errors.push(`${(e as Error).message}。ここで止めた`);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
  if (errors.length > 0) return fail();
  log(`作った音声 ${generated.length} 件、消した音声 ${removed.length} 件`);
  return { exitCode: 0, generated, removed, errors };

  function processLesson(
    {
      entry,
      sentences,
      durations: oldDurations,
      cueReference: oldCue,
    }: LessonPlan,
    voice: VoiceSettings,
  ) {
    const tag = `[${entry.id}]`;
    const voiceDir = videoPath(entry, "voice");
    const ids = new Set(sentences.map((s) => s.id));
    const durations = ordered(sentences, oldDurations);
    const cueReference = ordered(sentences, oldCue);
    const durationsPath = videoPath(entry, "durations.json");
    const cuePath = videoPath(entry, "cue-reference.json");
    const save = () => {
      if (Object.keys(durations).length > 0 || existsSync(durationsPath)) {
        writeJsonIfChanged(durationsPath, ordered(sentences, durations));
      }
      if (Object.keys(cueReference).length > 0 || existsSync(cuePath)) {
        writeJsonIfChanged(cuePath, ordered(sentences, cueReference));
      }
    };

    // 台本にない文の音声ファイルと、durations.json・cue-reference.json の記録を消す。
    const files = existsSync(voiceDir) ? readdirSync(voiceDir) : [];
    for (const name of files) {
      if (name.endsWith(".m4a") && !ids.has(name.slice(0, -".m4a".length))) {
        rmSync(join(voiceDir, name));
        removed.push(videoRel(entry, `voice/${name}`));
        log(`${tag} 台本にない文の音声 voice/${name} を消した`);
      }
    }
    for (const id of Object.keys(oldDurations)) {
      if (!ids.has(id))
        log(`${tag} 台本にない文 ${id} の記録を durations.json から消した`);
    }
    for (const id of Object.keys(oldCue)) {
      if (!ids.has(id))
        log(`${tag} 台本にない文 ${id} の記録を cue-reference.json から消した`);
    }
    save();

    const targets = selectSentences(
      sentences,
      durations,
      voiceValue,
      (id) => existsSync(join(voiceDir, `${id}.m4a`)),
      options.all,
    );
    if (targets.length === 0) log(`${tag} 作り直す文はない`);
    // VOICEPEAK を同時に 2 つ以上動かさないよう、1 文ずつ順に作る。
    for (const sentence of targets) {
      const rawPath = join(work, `${sentence.id}.voicepeak.wav`);
      const trimmedPath = join(work, `${sentence.id}.wav`);
      const m4aPath = join(work, `${sentence.id}.m4a`);
      tools.synthesize(sentence.say, voice, rawPath);
      const trimmed = trimCommaPauses(
        parseWav(readFileSync(rawPath)),
        sentence.say,
        voice.pause,
      );
      if (!trimmed.ok) {
        errors.push(
          `${videoRel(entry, "script.json")}: 文 ${sentence.id} の音声を作れなかった。${trimmed.message}。say の句読点を見直す（無音の見つけ方は documents/spec/content-format/video/voice.md）`,
        );
        continue;
      }
      writeFileSync(trimmedPath, serializeWav(trimmed.wav));
      tools.encode(trimmedPath, m4aPath);
      mkdirSync(voiceDir, { recursive: true });
      const target = join(voiceDir, `${sentence.id}.m4a`);
      copyFileSync(m4aPath, target);
      const seconds = tools.measureSeconds(target);
      const record: DurationRecord = {
        seconds,
        say: sentence.say,
        voice: voiceValue,
      };
      durations[sentence.id] = record;
      if (!(sentence.id in cueReference)) cueReference[sentence.id] = seconds;
      save();
      generated.push(`${entry.id}/${sentence.id}`);
      const commas = trimmed.silences.filter((s) => s.mark === "comma").length;
      log(
        `${tag} ${sentence.id}: 作った（${seconds} 秒。「、」の無音 ${commas} か所を ${voice.pause} 秒に切り詰め、${trimmed.removedSeconds.toFixed(3)} 秒短くした）`,
      );
    }

    for (const id of new Set(options.cues)) {
      const record = durations[id];
      if (!isRecord(record) || typeof record.seconds !== "number") {
        errors.push(
          `文 ${id} の音声の長さが durations.json にないので、cue-reference.json を書き換えられない`,
        );
        continue;
      }
      cueReference[id] = record.seconds;
      log(
        `${tag} ${id}: cue-reference.json の値を ${record.seconds} 秒に書き換えた`,
      );
    }
    save();
  }
}
