import { describe, expect, it } from "vitest";
import type { Script } from "../commands/content";
import {
  computeTimeline,
  createCues,
  secondsToFrames,
  type Durations,
} from "./timeline";

const voice = {
  narrator: "Natsuki Karin",
  speed: 120,
  pause: 0.2,
  emotion: { hightension: 20 },
};

// テスト用の台本。title の lead は 1.3 秒、tail は省略（0.7 秒）。
// move の lead は 1.7 秒、tail は 1.6 秒。m1 の pause_after は省略（0.45 秒）。
const script: Script = {
  code_blocks: [{ id: "main", language: "rust", code: "fn main() {}" }],
  scenes: [
    { id: "title", name: "表紙", lead: 1.3 },
    { id: "move", name: "ムーブの図解", lead: 1.7, tail: 1.6 },
  ],
  sentences: [
    { id: "t1", scene: "title", text: "表紙の文", say: "ひょうしのぶん" },
    { id: "m1", scene: "move", text: "文 1", say: "ぶんいち" },
    {
      id: "m2",
      scene: "move",
      text: "文 2",
      say: "ぶんに",
      pause_after: 0.8,
    },
    { id: "m3", scene: "move", text: "文 3", say: "ぶんさん" },
  ],
};

function durationsOf(seconds: { [id: string]: number }): Durations {
  return Object.fromEntries(
    Object.entries(seconds).map(([id, s]) => [
      id,
      { seconds: s, say: id, voice },
    ]),
  );
}

const durations = durationsOf({ t1: 3.0, m1: 3.757, m2: 2.5, m3: 2.0 });

describe("secondsToFrames", () => {
  it("30fps のフレーム数に四捨五入する", () => {
    expect(secondsToFrames(1)).toBe(30);
    expect(secondsToFrames(3.757)).toBe(113); // 112.71
    expect(secondsToFrames(0.01)).toBe(0); // 0.3
  });

  it("ちょうど半分は切り上げる（浮動小数点の誤差で切り捨てない）", () => {
    // 0.45 × 30 は、浮動小数点では 13.499999999999998 になる。
    expect(secondsToFrames(0.45)).toBe(14);
    expect(secondsToFrames(0.05)).toBe(2); // 1.5
  });

  it("負の秒は、絶対値で四捨五入する", () => {
    expect(secondsToFrames(-0.2)).toBe(-6);
    expect(secondsToFrames(-0.45)).toBe(-14);
    expect(secondsToFrames(-0.01)).toBe(0);
  });
});

describe("computeTimeline", () => {
  it("シーンを台本の順に間を空けずに並べ、文を lead・音声の長さ・pause_after・tail で並べる", () => {
    const timeline = computeTimeline(script, durations, {});
    expect(timeline.scenes).toEqual([
      {
        id: "title",
        from: 0,
        // lead 39 + t1 90 + tail 21（省略時の 0.7 秒）
        durationInFrames: 150,
        sentences: [{ id: "t1", from: 39, audioEnd: 129, ratio: 1 }],
      },
      {
        id: "move",
        from: 150,
        // m3 の音声の終わり 337 + tail 48
        durationInFrames: 385,
        sentences: [
          // lead 1.7 秒 = 51、m1 は 3.757 秒 = 113
          { id: "m1", from: 51, audioEnd: 164, ratio: 1 },
          // 省略時の pause_after 0.45 秒 = 14
          { id: "m2", from: 178, audioEnd: 253, ratio: 1 },
          // pause_after 0.8 秒 = 24
          { id: "m3", from: 277, audioEnd: 337, ratio: 1 },
        ],
      },
    ]);
    // 動画の長さは、シーンの長さの合計。
    expect(timeline.durationInFrames).toBe(535);
  });

  it("秒は、足し合わせる前に、それぞれフレーム数に四捨五入する", () => {
    // m1 の音声 112.71 と間 13.5 を足してから丸めると 126 になり、m2 は 177 から始まる。
    // それぞれを丸めてから足すので、113 + 14 = 127 で、m2 は 178 から始まる。
    const move = computeTimeline(script, durations, {}).scenes[1];
    expect(move.sentences[1].from).toBe(51 + 113 + 14);
  });

  it("文の音声の長さが変わると、後ろの文とシーンの開始位置がその分だけ移る", () => {
    const before = computeTimeline(script, durations, {});

    // m1 を 3.757 秒（113 フレーム）から 4.5 秒（135 フレーム）にする: 22 フレーム後ろへ。
    const longerM1 = computeTimeline(
      script,
      { ...durations, ...durationsOf({ m1: 4.5 }) },
      {},
    );
    const [, moveBefore] = before.scenes;
    const [titleAfter, moveAfter] = longerM1.scenes;
    expect(titleAfter).toEqual(before.scenes[0]);
    expect(moveAfter.sentences[0].from).toBe(moveBefore.sentences[0].from);
    expect(moveAfter.sentences[0].audioEnd).toBe(
      moveBefore.sentences[0].audioEnd + 22,
    );
    expect(moveAfter.sentences[1].from).toBe(moveBefore.sentences[1].from + 22);
    expect(moveAfter.sentences[2].from).toBe(moveBefore.sentences[2].from + 22);
    expect(moveAfter.durationInFrames).toBe(moveBefore.durationInFrames + 22);
    expect(longerM1.durationInFrames).toBe(before.durationInFrames + 22);

    // t1 を 3.0 秒（90 フレーム）から 3.5 秒（105 フレーム）にする: 後ろのシーンが 15 フレーム後ろへ。
    const longerT1 = computeTimeline(
      script,
      { ...durations, ...durationsOf({ t1: 3.5 }) },
      {},
    );
    expect(longerT1.scenes[1].from).toBe(moveBefore.from + 15);
    // シーンの中の位置（シーンの先頭からのフレーム数）は変わらない。
    expect(longerT1.scenes[1].sentences).toEqual(moveBefore.sentences);
    expect(longerT1.durationInFrames).toBe(before.durationInFrames + 15);
  });

  it("文の音声の長さが durations.json にないと、エラーになる", () => {
    const withoutM2 = Object.fromEntries(
      Object.entries(durations).filter(([id]) => id !== "m2"),
    );
    expect(() => computeTimeline(script, withoutM2, {})).toThrow(/m2/);
  });

  it("台本の scenes にないシーンの文があると、エラーになる", () => {
    const broken: Script = {
      ...script,
      sentences: [
        ...script.sentences,
        { id: "x1", scene: "nothing", text: "x", say: "x" },
      ],
    };
    expect(() =>
      computeTimeline(broken, { ...durations, ...durationsOf({ x1: 1 }) }, {}),
    ).toThrow(/nothing/);
  });

  it("文の無いシーンがあると、エラーになる", () => {
    const broken: Script = {
      ...script,
      scenes: [...script.scenes, { id: "empty", name: "空" }],
    };
    expect(() => computeTimeline(broken, durations, {})).toThrow(/empty/);
  });
});

describe("createCues", () => {
  // m1 の合図は 6.0 秒の音声で合わせ、いまの音声は 3.0 秒。
  const timeline = computeTimeline(
    script,
    { ...durations, ...durationsOf({ m1: 3.0 }) },
    { m1: 6.0 },
  );
  const m1From = 51;
  const m1AudioEnd = 51 + 90;
  const m2From = m1AudioEnd + 14;
  const cues = createCues(timeline, "move");

  it("at の正の秒に、いまの音声の長さとの比を掛ける", () => {
    // 比は 3.0 ÷ 6.0 = 0.5。2.0 秒は 1.0 秒後になる。
    expect(cues.at("m1", 2.0)).toBe(m1From + 30);
  });

  it("at の負の秒には比を掛けない", () => {
    expect(cues.at("m1", -0.2)).toBe(m1From - 6);
  });

  it("at の秒を省略すると、文の開始位置を指す", () => {
    expect(cues.at("m1")).toBe(m1From);
  });

  it("cue-reference.json にない文では、比を 1 とする", () => {
    expect(cues.at("m2", 1.0)).toBe(m2From + 30);
  });

  it("秒（at の正の秒は比を掛けた後）をフレーム数に四捨五入してから足す", () => {
    // m1 の 0.1 秒は、比 0.5 を掛けて 0.05 秒 = 1.5 フレーム。2 フレームに丸める。
    expect(cues.at("m1", 0.1)).toBe(m1From + 2);
    // 負の秒は、絶対値で四捨五入する。
    expect(cues.at("m1", -0.45)).toBe(m1From - 14);
    expect(cues.endOf("m1", -0.45)).toBe(m1AudioEnd - 14);
  });

  it("endOf は、文の音声が終わる位置から、比を掛けずに秒だけ後を指す", () => {
    expect(cues.endOf("m1")).toBe(m1AudioEnd);
    expect(cues.endOf("m1", 0.5)).toBe(m1AudioEnd + 15);
    expect(cues.endOf("m1", -0.5)).toBe(m1AudioEnd - 15);
  });

  it("シーンの先頭を 0 としたフレーム数を返す", () => {
    // move は 150 フレーム目から始まるが、合図はシーンの中の位置を返す。
    expect(timeline.scenes[1].from).toBeGreaterThan(0);
    expect(createCues(timeline, "title").at("t1")).toBe(39);
  });

  it("ほかのシーンの文の ID を指すと、エラーになる", () => {
    expect(() => cues.at("t1")).toThrow(/ほかのシーン title の文 t1/);
    expect(() => cues.endOf("t1")).toThrow(/ほかのシーン title の文 t1/);
  });

  it("台本にない文の ID を指すと、エラーになる", () => {
    expect(() => cues.at("nothing")).toThrow(/台本にない文 nothing/);
    expect(() => cues.endOf("nothing")).toThrow(/台本にない文 nothing/);
  });
});
