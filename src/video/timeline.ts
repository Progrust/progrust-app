// 動画のタイムラインと合図（documents/spec/content-format/video/timeline-and-cues.md）。
import type { Script } from "../commands/content";

// 動画のフレームレート（documents/spec/content-format/video/structure.md）。
export const FPS = 30;

// 台本で省略したときの間（秒。documents/spec/content-format/video/script-json.md）。
const DEFAULT_LEAD = 0.9;
const DEFAULT_TAIL = 0.7;
const DEFAULT_PAUSE_AFTER = 0.45;

// 声の設定（documents/spec/content-format/video/voice.md「声の設定」）。
export type VoiceSettings = {
  narrator: string;
  speed: number;
  pause: number;
  emotion: { [name: string]: number };
};

// 音声の長さ（durations.json）。キーは文の ID。
export type Durations = {
  [sentenceId: string]: {
    seconds: number;
    say: string;
    voice: VoiceSettings;
  };
};

// 合図を合わせたときの音声の長さ（cue-reference.json）。キーは文の ID、値は秒。
export type CueReference = { [sentenceId: string]: number };

// 文の位置。フレーム数は、シーンの先頭を 0 とする。
export type SentenceTiming = {
  id: string;
  // 文の開始位置。音声はここから鳴らす。
  from: number;
  // 文の音声が終わる位置。
  audioEnd: number;
  // `at` の正の秒に掛ける比（いまの音声の長さ ÷ 合図を合わせたときの長さ）。
  ratio: number;
};

// シーンの位置。`from` は動画の先頭を 0 としたフレーム数。
export type SceneTiming = {
  id: string;
  from: number;
  durationInFrames: number;
  sentences: SentenceTiming[];
};

export type Timeline = {
  scenes: SceneTiming[];
  // 動画の長さ（すべてのシーンの長さの合計）。
  durationInFrames: number;
};

// 秒をフレーム数に四捨五入する。
export function secondsToFrames(seconds: number): number {
  // 0.45 × 30 が 13.499999999999998 になるような浮動小数点の誤差で、
  // 13.5 が 13 に切り捨てられないよう、先に小数第 6 位で丸める。
  const frames = Number((seconds * FPS).toFixed(6));
  // 負の値も、絶対値で四捨五入する（−13.5 は −14）。`+ 0` で −0 を 0 にする。
  return frames < 0 ? -Math.round(-frames) + 0 : Math.round(frames);
}

// 台本と音声の長さから、シーンと文の位置を決める。
export function computeTimeline(
  script: Script,
  durations: Durations,
  cueReference: CueReference,
): Timeline {
  const sceneIds = new Set(script.scenes.map((scene) => scene.id));
  for (const sentence of script.sentences) {
    if (!sceneIds.has(sentence.scene)) {
      throw new Error(
        `文 ${sentence.id} のシーン ${sentence.scene} が、台本の scenes にない`,
      );
    }
  }

  let sceneFrom = 0;
  const scenes = script.scenes.map((scene): SceneTiming => {
    const sentences = script.sentences.filter((s) => s.scene === scene.id);
    if (sentences.length === 0) {
      throw new Error(`シーン ${scene.id} に文がない`);
    }

    let cursor = secondsToFrames(scene.lead ?? DEFAULT_LEAD);
    const timings = sentences.map((sentence): SentenceTiming => {
      const duration = durations[sentence.id];
      if (duration === undefined) {
        throw new Error(
          `文 ${sentence.id} の音声の長さが durations.json にない`,
        );
      }
      const from = cursor;
      const audioEnd = from + secondsToFrames(duration.seconds);
      cursor =
        audioEnd + secondsToFrames(sentence.pause_after ?? DEFAULT_PAUSE_AFTER);
      const reference = cueReference[sentence.id];
      const ratio = reference === undefined ? 1 : duration.seconds / reference;
      return { id: sentence.id, from, audioEnd, ratio };
    });

    const last = timings[timings.length - 1];
    const durationInFrames =
      last.audioEnd + secondsToFrames(scene.tail ?? DEFAULT_TAIL);
    const timing: SceneTiming = {
      id: scene.id,
      from: sceneFrom,
      durationInFrames,
      sentences: timings,
    };
    sceneFrom += durationInFrames;
    return timing;
  });

  return { scenes, durationInFrames: sceneFrom };
}

// 合図の関数（documents/spec/content-format/video/timeline-and-cues.md「合図」）。
// どちらも、シーンの先頭を 0 としたフレーム数を返す。
export type Cues = {
  // 文の開始位置から、指定した秒だけ後の位置。負の秒は開始位置より前を指す。
  at: (sentenceId: string, seconds?: number) => number;
  // 文の音声が終わる位置から、指定した秒だけ後の位置。
  endOf: (sentenceId: string, seconds?: number) => number;
};

// シーンの中で使う合図の関数を作る。指せるのは、そのシーンの文だけ。
export function createCues(timeline: Timeline, sceneId: string): Cues {
  const scene = timeline.scenes.find((s) => s.id === sceneId);
  if (scene === undefined) {
    throw new Error(`シーン ${sceneId} がタイムラインにない`);
  }

  const find = (sentenceId: string): SentenceTiming => {
    const sentence = scene.sentences.find((s) => s.id === sentenceId);
    if (sentence !== undefined) {
      return sentence;
    }
    const other = timeline.scenes.find((s) =>
      s.sentences.some((t) => t.id === sentenceId),
    );
    throw new Error(
      other === undefined
        ? `シーン ${sceneId} の合図が、台本にない文 ${sentenceId} を指している`
        : `シーン ${sceneId} の合図が、ほかのシーン ${other.id} の文 ${sentenceId} を指している`,
    );
  };

  return {
    at: (sentenceId, seconds = 0) => {
      const sentence = find(sentenceId);
      // 正の秒は、合図を合わせたときの音声での位置なので、いまの音声の長さとの比を掛ける。
      const scaled = seconds > 0 ? seconds * sentence.ratio : seconds;
      return sentence.from + secondsToFrames(scaled);
    },
    endOf: (sentenceId, seconds = 0) => {
      const sentence = find(sentenceId);
      return sentence.audioEnd + secondsToFrames(seconds);
    },
  };
}
