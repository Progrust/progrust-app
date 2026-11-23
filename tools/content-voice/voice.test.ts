// 音声を作るツール（documents/spec/content-format/video/voice.md「音声を作るツール」）のテスト。
// VOICEPEAK・ffmpeg・ffprobe は、決まった音声を作る偽の VoiceTools に置き換える。
import { spawnSync } from "node:child_process";
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
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { checkAudio } from "../content-check/audio.ts";
import {
  ffmpegArgs,
  ffprobeArgs,
  parseDuration,
  voicepeakArgs,
} from "./external.ts";
import { parseVoiceArgs, USAGE } from "./main.ts";
import { pauseSilences } from "./silence.ts";
import { parseWav, serializeWav, wavSeconds } from "./wav.ts";
import {
  generateVoices,
  selectSentences,
  validateVoice,
  type VoiceOptions,
  type VoiceSettings,
  type VoiceTools,
} from "./voice.ts";

const root = fileURLToPath(new URL("../..", import.meta.url));
const RATE = 8000;

// 仕様の例の声の設定。
const VOICE: VoiceSettings = {
  narrator: "Natsuki Karin",
  speed: 120,
  pause: 0.2,
  emotion: {
    hightension: 20,
    buchigire: 5,
    nageki: 0,
    sagesumi: 20,
    sasayaki: 80,
  },
};

// 偽の VOICEPEAK が作る音声。句読点で区切った語ごとに音を出し、「、」の後に 0.5 秒、
// 「。」の後に 0.9 秒の無音を置く。音の長さは、語の文字数と速さで決まる。
function fakeSpeech(say: string, voice: VoiceSettings): Buffer {
  const values: number[] = [];
  const push = (seconds: number, loud: boolean) => {
    for (let i = 0; i < Math.round(seconds * RATE); i++) {
      values.push(loud ? (i % 2 === 0 ? 8000 : -8000) : 0);
    }
  };
  push(0.05, false);
  const parts = say.split(/([、。]+)/u).filter((p) => p !== "");
  parts.forEach((part, i) => {
    if (/^[、。]+$/u.test(part)) {
      if (i < parts.length - 1) push(part.includes("。") ? 0.9 : 0.5, false);
    } else {
      push((0.1 * [...part].length * 120) / voice.speed, true);
    }
  });
  push(0.3, false);
  return serializeWav({
    sampleRate: RATE,
    channels: 1,
    samples: Int16Array.from(values),
  });
}

interface FakeTools extends VoiceTools {
  // VOICEPEAK に渡した say。
  said: string[];
}

function fakeTools(speech = fakeSpeech): FakeTools {
  const said: string[] = [];
  return {
    said,
    synthesize(say, voice, wavPath) {
      said.push(say);
      writeFileSync(wavPath, speech(say, voice));
    },
    // 偽の変換は WAV をそのまま写し、偽の ffprobe は WAV の長さを返す。
    encode(wavPath, m4aPath) {
      copyFileSync(wavPath, m4aPath);
    },
    measureSeconds(m4aPath) {
      return wavSeconds(parseWav(readFileSync(m4aPath)));
    },
  };
}

interface Sentence {
  id: string;
  scene?: string;
  text?: string;
  say: string;
}

function script(sentences: Sentence[]) {
  return {
    scenes: [{ id: "main", name: "本編" }],
    sentences: sentences.map((s) => ({
      id: s.id,
      scene: s.scene ?? "main",
      text: s.text ?? s.say,
      say: s.say,
    })),
  };
}

const SENTENCES: Sentence[] = [
  { id: "t1", say: "このレッスンでは、所有権のムーブを学びます。" },
  { id: "m1", say: "二行目で、変数エスワンにストリングのあたいを入れました。" },
  {
    id: "m2",
    say: "文字列のなかみはヒープにあり、エスワンがその所有者です。つぎに、代入を見ます。",
  },
];

const dirs: string[] = [];

afterEach(() => {
  for (const dir of dirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

function setup(
  lessons: Record<string, Sentence[]> = { "ch01-01": SENTENCES },
  voice: unknown = VOICE,
) {
  const dir = mkdtempSync(join(tmpdir(), "progrust-content-voice-test-"));
  dirs.push(dir);
  const contentDir = join(dir, "content");
  mkdirSync(contentDir);
  writeFileSync(
    join(contentDir, "chapters.json"),
    JSON.stringify([
      {
        id: "ch01",
        title: "Rust の基本",
        description: "説明",
        lessons: [...Object.keys(lessons), "ch01-09", "ch01-review"],
      },
    ]),
  );
  writeFileSync(join(contentDir, "voice.json"), JSON.stringify(voice));
  mkdirSync(join(contentDir, "lessons", "ch01-review"), { recursive: true });
  for (const [id, sentences] of Object.entries(lessons)) {
    writeScript(contentDir, id, sentences);
  }
  return contentDir;
}

function videoDir(contentDir: string, lesson = "ch01-01") {
  return join(contentDir, "lessons", lesson, "video");
}

function writeScript(
  contentDir: string,
  lesson: string,
  sentences: Sentence[],
) {
  const dir = videoDir(contentDir, lesson);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "script.json"), JSON.stringify(script(sentences)));
}

function writeVoice(contentDir: string, voice: unknown) {
  writeFileSync(join(contentDir, "voice.json"), JSON.stringify(voice));
}

function readJson(path: string) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function durations(contentDir: string, lesson = "ch01-01") {
  return readJson(join(videoDir(contentDir, lesson), "durations.json"));
}

function cueReference(contentDir: string, lesson = "ch01-01") {
  return readJson(join(videoDir(contentDir, lesson), "cue-reference.json"));
}

function voiceFiles(contentDir: string, lesson = "ch01-01") {
  const dir = join(videoDir(contentDir, lesson), "voice");
  return existsSync(dir) ? readdirSync(dir).sort() : [];
}

function run(
  contentDir: string,
  tools: VoiceTools,
  options: Partial<VoiceOptions> = {},
) {
  const logs: string[] = [];
  const errors: string[] = [];
  const result = generateVoices(
    {
      contentDir,
      lessons: [],
      all: false,
      cues: [],
      log: (line) => logs.push(line),
      error: (line) => errors.push(line),
      ...options,
    },
    tools,
  );
  return { ...result, logs, errorLines: errors };
}

describe("音声を作る", () => {
  it("台本のすべての文の音声と、durations.json・cue-reference.json を作り、教材チェックツールの「音声」が通る", () => {
    const contentDir = setup();
    const tools = fakeTools();
    const result = run(contentDir, tools);
    expect(result.exitCode).toBe(0);
    expect(tools.said).toEqual(SENTENCES.map((s) => s.say));
    expect(voiceFiles(contentDir)).toEqual(["m1.m4a", "m2.m4a", "t1.m4a"]);

    const records = durations(contentDir);
    expect(Object.keys(records)).toEqual(["t1", "m1", "m2"]);
    for (const s of SENTENCES) {
      expect(records[s.id].say).toBe(s.say);
      expect(records[s.id].voice).toEqual(VOICE);
      const file = join(videoDir(contentDir), "voice", `${s.id}.m4a`);
      expect(records[s.id].seconds).toBe(
        wavSeconds(parseWav(readFileSync(file))),
      );
    }
    expect(cueReference(contentDir)).toEqual({
      t1: records.t1.seconds,
      m1: records.m1.seconds,
      m2: records.m2.seconds,
    });

    expect(
      checkAudio({ contentDir, curriculumDir: join(contentDir, "none") }),
    ).toEqual([]);
  });

  it("「、」の無音を pause 秒まで切り詰め、文の途中の「。」の無音は残す", () => {
    const contentDir = setup({
      "ch01-01": [{ id: "m2", say: SENTENCES[2].say }],
    });
    run(contentDir, fakeTools());
    // 偽の音声の「、」の無音 0.5 秒 2 か所が、それぞれ 0.2 秒になる。「。」の無音 0.9 秒は残る。
    const raw = wavSeconds(parseWav(fakeSpeech(SENTENCES[2].say, VOICE)));
    expect(durations(contentDir).m2.seconds).toBeCloseTo(raw - 0.3 - 0.3, 6);
    const audio = parseWav(
      readFileSync(join(videoDir(contentDir), "voice", "m2.m4a")),
    );
    expect(pauseSilences(audio).map((s) => (s.end - s.start) / RATE)).toEqual([
      0.2, 0.9, 0.2,
    ]);
  });

  it("2 回目は、変えていない文の音声を作り直さない", () => {
    const contentDir = setup();
    run(contentDir, fakeTools());
    const before = readFileSync(
      join(videoDir(contentDir), "durations.json"),
      "utf8",
    );
    const tools = fakeTools();
    const result = run(contentDir, tools);
    expect(result.exitCode).toBe(0);
    expect(tools.said).toEqual([]);
    expect(result.logs).toContain("[ch01-01] 作り直す文はない");
    expect(
      readFileSync(join(videoDir(contentDir), "durations.json"), "utf8"),
    ).toBe(before);
  });

  it("1 つの文の say を変えると、その文の音声だけを作り直し、durations.json のその文の記録を更新する", () => {
    const contentDir = setup();
    run(contentDir, fakeTools());
    const before = durations(contentDir);
    const t1Audio = readFileSync(join(videoDir(contentDir), "voice", "t1.m4a"));

    const changed = "二行目で、変数エスワンに、ストリングのあたいを入れます。";
    writeScript(contentDir, "ch01-01", [
      SENTENCES[0],
      { id: "m1", say: changed },
      SENTENCES[2],
    ]);
    const tools = fakeTools();
    expect(run(contentDir, tools).exitCode).toBe(0);
    expect(tools.said).toEqual([changed]);

    const after = durations(contentDir);
    expect(after.t1).toEqual(before.t1);
    expect(after.m2).toEqual(before.m2);
    expect(after.m1.say).toBe(changed);
    expect(after.m1.seconds).not.toBe(before.m1.seconds);
    expect(after.m1.seconds).toBe(
      wavSeconds(
        parseWav(readFileSync(join(videoDir(contentDir), "voice", "m1.m4a"))),
      ),
    );
    expect(readFileSync(join(videoDir(contentDir), "voice", "t1.m4a"))).toEqual(
      t1Audio,
    );
  });

  it("voice.json の speed を変えると、すべての文の音声を作り直す", () => {
    const contentDir = setup({
      "ch01-01": SENTENCES,
      "ch01-02": [{ id: "a1", say: "はい。それでは、つぎに進みましょう。" }],
    });
    run(contentDir, fakeTools());
    const before = durations(contentDir);

    const faster = { ...VOICE, speed: 150 };
    writeVoice(contentDir, faster);
    const tools = fakeTools();
    expect(run(contentDir, tools).exitCode).toBe(0);
    expect(tools.said).toEqual([
      ...SENTENCES.map((s) => s.say),
      "はい。それでは、つぎに進みましょう。",
    ]);
    const after = durations(contentDir);
    for (const s of SENTENCES) {
      expect(after[s.id].voice).toEqual(faster);
      expect(after[s.id].seconds).toBeLessThan(before[s.id].seconds);
    }
    expect(durations(contentDir, "ch01-02").a1.voice).toEqual(faster);
  });

  it("声の設定のほかの項目を変えても、すべての文の音声を作り直す", () => {
    const contentDir = setup();
    run(contentDir, fakeTools());
    writeVoice(contentDir, {
      ...VOICE,
      emotion: { ...VOICE.emotion, sasayaki: 70 },
    });
    const tools = fakeTools();
    run(contentDir, tools);
    expect(tools.said).toHaveLength(SENTENCES.length);
  });

  it("すべての文を作り直す指定（all）では、変えていない文も作り直す", () => {
    const contentDir = setup();
    run(contentDir, fakeTools());
    const tools = fakeTools();
    expect(run(contentDir, tools, { all: true }).exitCode).toBe(0);
    expect(tools.said).toEqual(SENTENCES.map((s) => s.say));
  });

  it("音声ファイルがない文は、作り直す", () => {
    const contentDir = setup();
    run(contentDir, fakeTools());
    rmSync(join(videoDir(contentDir), "voice", "m1.m4a"));
    const tools = fakeTools();
    run(contentDir, tools);
    expect(tools.said).toEqual([SENTENCES[1].say]);
  });

  it("対象のレッスンを指定すると、そのレッスンだけを作る", () => {
    const contentDir = setup({
      "ch01-01": SENTENCES,
      "ch01-02": [{ id: "a1", say: "はい、そうです。" }],
    });
    const tools = fakeTools();
    expect(run(contentDir, tools, { lessons: ["ch01-02"] }).exitCode).toBe(0);
    expect(tools.said).toEqual(["はい、そうです。"]);
    expect(voiceFiles(contentDir)).toEqual([]);
  });

  it("台本にない文の音声ファイルと、durations.json・cue-reference.json の記録を消す", () => {
    const contentDir = setup();
    run(contentDir, fakeTools());
    writeScript(contentDir, "ch01-01", [SENTENCES[0], SENTENCES[2]]);
    const tools = fakeTools();
    const result = run(contentDir, tools);
    expect(result.exitCode).toBe(0);
    expect(tools.said).toEqual([]);
    expect(voiceFiles(contentDir)).toEqual(["m2.m4a", "t1.m4a"]);
    expect(Object.keys(durations(contentDir))).toEqual(["t1", "m2"]);
    expect(Object.keys(cueReference(contentDir))).toEqual(["t1", "m2"]);
    expect(result.removed).toEqual(["lessons/ch01-01/video/voice/m1.m4a"]);
  });
});

describe("say が 140 字を超える文", () => {
  const say = (n: number) => `${"あ".repeat(n - 1)}。`;

  it("say が 141 字の文がある台本は、音声を作らずにエラーにする", () => {
    const contentDir = setup({
      "ch01-01": [SENTENCES[0], { id: "long", say: say(141) }],
    });
    const tools = fakeTools();
    const result = run(contentDir, tools);
    expect(result.exitCode).toBe(1);
    expect(tools.said).toEqual([]);
    expect(result.errorLines.join("\n")).toContain(
      "文 longの say が 141 字で、140 字を超えている",
    );
    expect(voiceFiles(contentDir)).toEqual([]);
    expect(existsSync(join(videoDir(contentDir), "durations.json"))).toBe(
      false,
    );
  });

  it("ほかのレッスンの台本は、141 字の文がなければ音声を作る", () => {
    const contentDir = setup({
      "ch01-01": SENTENCES,
      "ch01-02": [{ id: "long", say: say(141) }],
    });
    const tools = fakeTools();
    const result = run(contentDir, tools);
    expect(result.exitCode).toBe(1);
    expect(result.errorLines.join("\n")).toContain(
      "レッスン ch01-02 の音声を作らない",
    );
    expect(tools.said).toEqual(SENTENCES.map((s) => s.say));
    expect(voiceFiles(contentDir, "ch01-02")).toEqual([]);
  });

  it("台本がないか、台本の形式に合わないレッスンは、そのレッスンの音声を作らない", () => {
    const contentDir = setup({
      "ch01-01": SENTENCES,
      "ch01-02": [
        { id: "a1", say: "はい。" },
        { id: "a1", say: "いいえ。" },
      ],
      "ch01-03": [],
    });
    rmSync(join(videoDir(contentDir, "ch01-03"), "script.json"));
    const tools = fakeTools();
    const result = run(contentDir, tools);
    expect(result.exitCode).toBe(1);
    const text = result.errorLines.join("\n");
    expect(text).toContain("sentencesの id a1 が重複している");
    expect(text).toContain("lessons/ch01-03/video/script.json: ない");
    expect(tools.said).toEqual(SENTENCES.map((s) => s.say));
  });

  it("say が 140 字なら、音声を作る", () => {
    const contentDir = setup({ "ch01-01": [{ id: "long", say: say(140) }] });
    const tools = fakeTools();
    expect(run(contentDir, tools).exitCode).toBe(0);
    expect(tools.said).toEqual([say(140)]);
  });
});

describe("cue-reference.json", () => {
  it("すでにある文の値は、音声を作り直しても書き換えない", () => {
    const contentDir = setup();
    run(contentDir, fakeTools());
    const before = cueReference(contentDir);
    writeScript(contentDir, "ch01-01", [
      SENTENCES[0],
      { id: "m1", say: "二行目で、エスワンを作りました。" },
      SENTENCES[2],
    ]);
    run(contentDir, fakeTools());
    expect(cueReference(contentDir)).toEqual(before);
    expect(durations(contentDir).m1.seconds).not.toBe(before.m1);
  });

  it("指定した文の値を、いまの音声の長さに書き換える", () => {
    const contentDir = setup();
    run(contentDir, fakeTools());
    writeScript(contentDir, "ch01-01", [
      SENTENCES[0],
      { id: "m1", say: "二行目で、エスワンを作りました。" },
      { id: "m2", say: "つぎに、代入を見ます。" },
    ]);
    const before = cueReference(contentDir);
    const result = run(contentDir, fakeTools(), {
      lessons: ["ch01-01"],
      cues: ["m1"],
    });
    expect(result.exitCode).toBe(0);
    const now = durations(contentDir);
    expect(cueReference(contentDir)).toEqual({
      t1: before.t1,
      m1: now.m1.seconds,
      m2: before.m2,
    });
  });

  it("音声を作り直さないときも、指定した文の値を書き換える", () => {
    const contentDir = setup();
    run(contentDir, fakeTools());
    const path = join(videoDir(contentDir), "cue-reference.json");
    writeFileSync(path, JSON.stringify({ t1: 9.9, m1: 9.9, m2: 9.9 }));
    const tools = fakeTools();
    run(contentDir, tools, { lessons: ["ch01-01"], cues: ["t1", "m2"] });
    expect(tools.said).toEqual([]);
    expect(cueReference(contentDir)).toEqual({
      t1: durations(contentDir).t1.seconds,
      m1: 9.9,
      m2: durations(contentDir).m2.seconds,
    });
  });

  it("台本にない文と、レッスンを 1 つに決めない指定はエラーにし、音声を作らない", () => {
    const contentDir = setup();
    const unknown = run(contentDir, fakeTools(), {
      lessons: ["ch01-01"],
      cues: ["zz"],
    });
    expect(unknown.exitCode).toBe(1);
    expect(unknown.errorLines.join("\n")).toContain("--cue の文 zz");
    expect(voiceFiles(contentDir)).toEqual([]);

    const noLesson = run(contentDir, fakeTools(), { cues: ["m1"] });
    expect(noLesson.exitCode).toBe(1);
    expect(noLesson.errorLines.join("\n")).toContain(
      "--lesson を 1 つだけ書いたとき",
    );
  });
});

describe("エラー", () => {
  it("文の途中の句読点と、音声の中の無音の数が合わない文は、その文の音声を作らずにエラーにする", () => {
    const contentDir = setup();
    // m1 だけ、「、」の無音が無い音声を返す。
    const tools = fakeTools((say, voice) =>
      fakeSpeech(say === SENTENCES[1].say ? say.replace("、", "") : say, voice),
    );
    const result = run(contentDir, tools);
    expect(result.exitCode).toBe(1);
    expect(result.errorLines.join("\n")).toContain(
      "文 m1 の音声を作れなかった",
    );
    expect(voiceFiles(contentDir)).toEqual(["m2.m4a", "t1.m4a"]);
    expect(Object.keys(durations(contentDir))).toEqual(["t1", "m2"]);
  });

  it("VOICEPEAK などが失敗したら、そこで止める", () => {
    const contentDir = setup();
    const tools = fakeTools();
    let count = 0;
    const failing: VoiceTools = {
      ...tools,
      synthesize(say, voice, path) {
        count += 1;
        if (count === 2) throw new Error("VOICEPEAK が失敗した");
        tools.synthesize(say, voice, path);
      },
    };
    const result = run(contentDir, failing);
    expect(result.exitCode).toBe(1);
    expect(result.errorLines.join("\n")).toContain("VOICEPEAK が失敗した");
    expect(count).toBe(2);
    expect(voiceFiles(contentDir)).toEqual(["t1.m4a"]);
    expect(Object.keys(durations(contentDir))).toEqual(["t1"]);
  });

  it("動画のないレッスンと、chapters.json にないレッスンの指定はエラーにする", () => {
    const contentDir = setup();
    const result = run(contentDir, fakeTools(), {
      lessons: ["ch01-review", "ch01-09", "ch09-01"],
    });
    expect(result.exitCode).toBe(1);
    const text = result.errorLines.join("\n");
    expect(text).toContain("レッスン ch01-review には動画がない");
    expect(text).toContain("レッスン ch01-09 には動画がない");
    expect(text).toContain("レッスン ch09-01 が chapters.json にない");
  });

  it("voice.json がないか、形が違うときは、音声を作らない", () => {
    const missing = setup();
    rmSync(join(missing, "voice.json"));
    const tools = fakeTools();
    expect(run(missing, tools).errorLines.join("\n")).toContain(
      "voice.json がない",
    );
    const bad = setup(undefined, { ...VOICE, speed: 300, pitch: 1 });
    const result = run(bad, tools);
    expect(result.exitCode).toBe(1);
    expect(result.errorLines.join("\n")).toContain("speed");
    expect(result.errorLines.join("\n")).toContain("pitch");
    expect(tools.said).toEqual([]);
  });

  it("durations.json か cue-reference.json を JSON として読めないレッスンは、音声を作らない", () => {
    const contentDir = setup();
    writeFileSync(join(videoDir(contentDir), "durations.json"), "{");
    const tools = fakeTools();
    expect(run(contentDir, tools).exitCode).toBe(1);
    expect(tools.said).toEqual([]);
    expect(
      readFileSync(join(videoDir(contentDir), "durations.json"), "utf8"),
    ).toBe("{");
    rmSync(join(videoDir(contentDir), "durations.json"));
    writeFileSync(join(videoDir(contentDir), "cue-reference.json"), "[]");
    expect(run(contentDir, tools).exitCode).toBe(1);
    expect(tools.said).toEqual([]);
  });
});

describe("selectSentences", () => {
  const sentences = [
    { id: "a", text: "a", say: "あ。" },
    { id: "b", text: "b", say: "い。" },
  ];
  const record = (say: string) => ({ seconds: 1, say, voice: VOICE });

  it("say・voice が同じで音声ファイルがある文だけを除く", () => {
    const select = (durations: Record<string, unknown>, has = true) =>
      selectSentences(sentences, durations, VOICE, () => has, false).map(
        (s) => s.id,
      );
    expect(select({ a: record("あ。"), b: record("い。") })).toEqual([]);
    expect(select({ a: record("あ。"), b: record("う。") })).toEqual(["b"]);
    expect(select({ a: record("あ。") })).toEqual(["b"]);
    expect(select({ a: record("あ。"), b: record("い。") }, false)).toEqual([
      "a",
      "b",
    ]);
    expect(
      select({
        a: record("あ。"),
        b: { ...record("い。"), voice: { ...VOICE, pause: 0.3 } },
      }),
    ).toEqual(["b"]);
    // 声の設定のキーの順序は問わない。
    const reordered = { ...record("い。"), voice: { ...VOICE } };
    reordered.voice = {
      emotion: VOICE.emotion,
      pause: VOICE.pause,
      speed: VOICE.speed,
      narrator: VOICE.narrator,
    };
    expect(select({ a: record("あ。"), b: reordered })).toEqual([]);
  });

  it("all のときは、すべての文を返す", () => {
    expect(
      selectSentences(
        sentences,
        { a: record("あ。"), b: record("い。") },
        VOICE,
        () => true,
        true,
      ),
    ).toEqual(sentences);
  });
});

describe("validateVoice", () => {
  it("リポジトリの content/voice.json は、仕様の例の値で、形が正しい", () => {
    const value = readJson(join(root, "content", "voice.json"));
    expect(value).toEqual(VOICE);
    expect(validateVoice(value)).toEqual(VOICE);
  });

  it("形の違反を返す", () => {
    expect(validateVoice({ ...VOICE, pause: -1 })).toEqual([
      "voice.json の pause が 0 以上の数でない",
    ]);
    expect(validateVoice({ ...VOICE, emotion: { a: "1" } })).toEqual([
      "voice.json の emotion が、項目名と数の組でない",
    ]);
    expect(validateVoice({ ...VOICE, narrator: "" })).toEqual([
      "voice.json の narrator が空か、文字列でない",
    ]);
    expect(validateVoice([])).toEqual(["voice.json がオブジェクトでない"]);
  });
});

describe("VOICEPEAK・ffmpeg・ffprobe の引数", () => {
  it("VOICEPEAK に、say・ナレーター・速さ・感情・出力先を渡す", () => {
    expect(voicepeakArgs("はい、そうです。", VOICE, "/tmp/x.wav")).toEqual([
      "-s",
      "はい、そうです。",
      "-n",
      "Natsuki Karin",
      "--speed",
      "120",
      "-e",
      "hightension=20,buchigire=5,nageki=0,sagesumi=20,sasayaki=80",
      "-o",
      "/tmp/x.wav",
    ]);
    expect(voicepeakArgs("あ", { ...VOICE, emotion: {} }, "x.wav")).toEqual([
      "-s",
      "あ",
      "-n",
      "Natsuki Karin",
      "--speed",
      "120",
      "-o",
      "x.wav",
    ]);
  });

  it("ffmpeg の aac で 64kbps・モノラルの .m4a に変換し、ffprobe の format の duration で測る", () => {
    const args = ffmpegArgs("in.wav", "out.m4a");
    expect(args.join(" ")).toContain(
      "-i in.wav -c:a aac -b:a 64k -ac 1 out.m4a",
    );
    expect(ffprobeArgs("out.m4a")).toContain("format=duration");
    expect(parseDuration("3.757000\n")).toBe(3.757);
    expect(() => parseDuration("N/A")).toThrow("ffprobe");
  });
});

describe("コマンドの指定", () => {
  it("レッスン・すべての文の作り直し・合図の書き換え・教材のディレクトリを読む", () => {
    expect(
      parseVoiceArgs([
        "--lesson",
        "ch01-01",
        "--all",
        "--cue",
        "m5",
        "--cue",
        "m6",
        "--content",
        "/tmp/content",
      ]),
    ).toEqual({
      kind: "run",
      options: {
        contentDir: "/tmp/content",
        lessons: ["ch01-01"],
        all: true,
        cues: ["m5", "m6"],
      },
    });
    expect(parseVoiceArgs([])).toEqual({
      kind: "run",
      options: {
        contentDir: `${root}content`,
        lessons: [],
        all: false,
        cues: [],
      },
    });
  });

  it("知らない指定はエラーにし、--help では使い方を表示する", () => {
    expect(parseVoiceArgs(["--speed", "100"]).kind).toBe("error");
    expect(parseVoiceArgs(["ch01-01"]).kind).toBe("error");
    expect(parseVoiceArgs(["-h"])).toEqual({ kind: "help" });
    for (const option of [
      "--lesson",
      "--all",
      "--cue",
      "--content",
      "--help",
    ]) {
      expect(USAGE).toContain(option);
    }
  });

  it("npm run content:voice は、使い方を表示できる", () => {
    const scripts = readJson(join(root, "package.json")).scripts;
    expect(scripts["content:voice"]).toBe("node tools/content-voice/main.ts");
    const help = spawnSync("node", ["tools/content-voice/main.ts", "--help"], {
      cwd: root,
      encoding: "utf8",
    });
    expect(help.status).toBe(0);
    expect(help.stdout.trim()).toBe(USAGE);
    const bad = spawnSync("node", ["tools/content-voice/main.ts", "--x"], {
      cwd: root,
      encoding: "utf8",
    });
    expect(bad.status).toBe(1);
    expect(bad.stderr).toContain(USAGE);
  });
});
