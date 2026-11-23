// 「、」にあたる無音の見つけ方と切り詰め（silence.ts）と、WAV の読み書き（wav.ts）のテスト。
import { describe, expect, it } from "vitest";
import {
  findSilences,
  matchMarks,
  pauseSilences,
  removeRanges,
  sentenceMarks,
  SILENCE,
  trimCommaPauses,
  trimRanges,
} from "./silence.ts";
import { frameCount, parseWav, serializeWav, type Wav } from "./wav.ts";

const RATE = 8000;

// 音（振幅 8000 の矩形波）と無音を、秒で並べた音声を作る。
type Part = { tone: number } | { silence: number };
function makeWav(parts: Part[]): Wav {
  const values: number[] = [];
  for (const part of parts) {
    const n = Math.round(("tone" in part ? part.tone : part.silence) * RATE);
    for (let i = 0; i < n; i++) {
      values.push("tone" in part ? (i % 2 === 0 ? 8000 : -8000) : 0);
    }
  }
  return { sampleRate: RATE, channels: 1, samples: Int16Array.from(values) };
}

// 無音の長さ（秒）を、音声の先頭から順に返す。
function silenceLengths(wav: Wav): number[] {
  return findSilences(wav, SILENCE.noiseDb).map(
    (s) => (s.end - s.start) / wav.sampleRate,
  );
}

describe("sentenceMarks", () => {
  it("文の途中の「、」と「。」を順に返し、末尾の句読点は数えない", () => {
    expect(
      sentenceMarks(
        "文字列のなかみはヒープにあり、エスワンがその所有者です。つぎに、代入を見ます。",
      ),
    ).toEqual(["comma", "period", "comma"]);
  });

  it("「？」「！」は「。」と同じに扱い、続けて並んだ句読点と空白は 1 つと数える", () => {
    expect(sentenceMarks("なぜでしょうか？　理由は、あとで見ます。")).toEqual([
      "period",
      "comma",
    ]);
    expect(sentenceMarks("本当ですか？！ はい、そうです")).toEqual([
      "period",
      "comma",
    ]);
    expect(sentenceMarks("まず、。つぎに")).toEqual(["period"]);
  });

  it("句読点のない文と、先頭の句読点は数えない", () => {
    expect(sentenceMarks("三行目でエスワンをエスツーに代入します。")).toEqual(
      [],
    );
    expect(sentenceMarks("、はい")).toEqual([]);
    expect(
      sentenceMarks("ここで「ムーブ」という言葉が、出てきました。"),
    ).toEqual(["comma"]);
  });
});

describe("findSilences と pauseSilences", () => {
  it("無音とみなす音量より小さいサンプルの並びを、無音として見つける", () => {
    // -50dB は、16 ビットのサンプルで約 103.6。
    const quiet = 103;
    const loud = 104;
    const wav: Wav = {
      sampleRate: RATE,
      channels: 1,
      samples: Int16Array.from([
        loud,
        quiet,
        -quiet,
        0,
        loud,
        -loud,
        quiet,
        loud,
      ]),
    };
    expect(findSilences(wav, -50)).toEqual([
      { start: 1, end: 4 },
      { start: 6, end: 7 },
    ]);
  });

  it("2 チャンネルでは、両方のチャンネルが小さいときだけ無音とみなす", () => {
    const wav: Wav = {
      sampleRate: RATE,
      channels: 2,
      samples: Int16Array.from([0, 0, 0, 5000, 0, 0]),
    };
    expect(findSilences(wav, -50)).toEqual([
      { start: 0, end: 1 },
      { start: 2, end: 3 },
    ]);
  });

  it("先頭と末尾に接する無音と、0.2 秒より短い無音は、句読点にあたる無音にしない", () => {
    const wav = makeWav([
      { silence: 0.3 },
      { tone: 0.3 },
      { silence: 0.19 },
      { tone: 0.3 },
      { silence: 0.2 },
      { tone: 0.3 },
      { silence: 0.35 },
    ]);
    expect(pauseSilences(wav).map((s) => (s.end - s.start) / RATE)).toEqual([
      0.2,
    ]);
  });
});

describe("trimCommaPauses", () => {
  // 「、」0.5 秒、「。」0.9 秒、子音の前の短い無音 0.06 秒を含む音声。
  const parts: Part[] = [
    { silence: 0.05 },
    { tone: 0.4 },
    { silence: 0.5 },
    { tone: 0.3 },
    { silence: 0.06 },
    { tone: 0.3 },
    { silence: 0.9 },
    { tone: 0.4 },
    { silence: 0.35 },
    { tone: 0.3 },
    { silence: 0.3 },
  ];
  const say = "このレッスンでは、所有権を学びます。つぎに、代入を見ます。";

  it("「、」にあたる無音を pause 秒まで切り詰め、「。」と短い無音と前後の無音はそのままにする", () => {
    const result = trimCommaPauses(makeWav(parts), say, 0.2);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(silenceLengths(result.wav)).toEqual([
      0.05, 0.2, 0.06, 0.9, 0.2, 0.3,
    ]);
    expect(result.silences.map((s) => s.mark)).toEqual([
      "comma",
      "period",
      "comma",
    ]);
    expect(result.removedSeconds).toBeCloseTo(0.3 + 0.15, 6);
    expect(frameCount(result.wav)).toBe(
      frameCount(makeWav(parts)) - Math.round(0.45 * RATE),
    );
  });

  it("「。」にあたる無音が「、」にあたる無音より短くても、「。」の無音はそのままにする", () => {
    const wav = makeWav([
      { tone: 0.3 },
      { silence: 0.6 },
      { tone: 0.3 },
      { silence: 0.72 },
      { tone: 0.3 },
    ]);
    const result = trimCommaPauses(wav, "はい！ では、つぎに進みます。", 0.2);
    expect(result.ok && silenceLengths(result.wav)).toEqual([0.6, 0.2]);
  });

  it("pause 秒より短い「、」の無音は、そのままにする", () => {
    const result = trimCommaPauses(makeWav(parts), say, 0.4);
    expect(result.ok && silenceLengths(result.wav)).toEqual([
      0.05, 0.4, 0.06, 0.9, 0.35, 0.3,
    ]);
  });

  it("pause が 0 なら、「、」の無音を取り除く", () => {
    const result = trimCommaPauses(makeWav(parts), say, 0);
    expect(result.ok && silenceLengths(result.wav)).toEqual([
      0.05, 0.06, 0.9, 0.3,
    ]);
  });

  it("無音の真ん中を取り除き、前後の音は変えない", () => {
    const wav = makeWav([{ tone: 0.1 }, { silence: 0.5 }, { tone: 0.1 }]);
    const ranges = trimRanges(
      [{ start: 800, end: 4800, mark: "comma" }],
      0.2,
      RATE,
    );
    expect(ranges).toEqual([{ start: 1600, end: 4000 }]);
    const out = removeRanges(wav, ranges);
    expect(Array.from(out.samples.subarray(0, 1600))).toEqual(
      Array.from(wav.samples.subarray(0, 1600)),
    );
    expect(Array.from(out.samples.subarray(1600))).toEqual(
      Array.from(wav.samples.subarray(4000)),
    );
  });

  it("文の途中の句読点の数と、句読点にあたる無音の数が合わないときは、作らない", () => {
    const result = trimCommaPauses(
      makeWav(parts),
      "このレッスンでは、所有権を学びます。",
      0.2,
    );
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.message).toContain("1 か所");
    expect(result.message).toContain("3 か所");
    expect(matchMarks([], ["comma"])).toBeUndefined();
  });
});

describe("parseWav と serializeWav", () => {
  it("書いた WAV を読むと、同じ音声になる", () => {
    const wav = makeWav([{ tone: 0.01 }, { silence: 0.01 }]);
    const back = parseWav(serializeWav(wav));
    expect(back.sampleRate).toBe(RATE);
    expect(back.channels).toBe(1);
    expect(Array.from(back.samples)).toEqual(Array.from(wav.samples));
  });

  it("fmt と data の間のほかのチャンクを読み飛ばす", () => {
    const plain = serializeWav(makeWav([{ tone: 0.001 }]));
    const list = Buffer.alloc(8 + 3 + 1);
    list.write("LIST", 0, "ascii");
    list.writeUInt32LE(3, 4);
    const withList = Buffer.concat([
      plain.subarray(0, 36),
      list,
      plain.subarray(36),
    ]);
    expect(Array.from(parseWav(withList).samples)).toEqual(
      Array.from(parseWav(plain).samples),
    );
  });

  it("16 ビットのリニア PCM でない WAV は読まない", () => {
    const buffer = serializeWav(makeWav([{ tone: 0.001 }]));
    buffer.writeUInt16LE(24, 34);
    expect(() => parseWav(buffer)).toThrow("16 ビット");
    expect(() => parseWav(Buffer.from("not a wav"))).toThrow("WAV");
  });
});
