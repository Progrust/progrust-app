// 「、」にあたる無音を見つけて、`pause` 秒まで切り詰める（documents/spec/content-format/video/voice.md「音声を作るツール」）。
import { frameCount, type Wav } from "./wav.ts";

// 無音の見つけ方の値。値と決めた理由は ADR 0198。
export const SILENCE = {
  // この音量（フルスケールに対する dB）より小さいサンプルが続くところを、無音とみなす。
  noiseDb: -50,
  // 句読点にあたる無音とみなす、いちばん短い長さ（秒）。
  minSeconds: 0.2,
} as const;

export interface SilenceParams {
  noiseDb: number;
  minSeconds: number;
}

export interface Silence {
  // 無音の最初のフレームと、無音の後の最初のフレーム（1 チャンネルあたりのサンプルの位置）。
  start: number;
  end: number;
}

// 文の途中の句読点の種類。「、」と、文を区切る「。」（「？」「！」を含む）。
export type Mark = "comma" | "period";

const COMMA = new Set(["、", "，", ","]);
const PERIOD = new Set(["。", "．", "？", "！", "?", "!"]);
// 句読点の並びの間にあっても、句読点の並びを切らない文字。
const SPACE = /\s/u;

// `say` の文の途中にある句読点を、順に返す。続けて並んだ句読点（「？！」など）は 1 つと数え、
// 「。」の類を含めば period とする。先頭と末尾の句読点は、音声の先頭と末尾の無音にあたるので数えない。
export function sentenceMarks(say: string): Mark[] {
  const marks: Mark[] = [];
  let run: Mark | undefined;
  let seenText = false;
  for (const ch of say) {
    const kind = COMMA.has(ch)
      ? "comma"
      : PERIOD.has(ch)
        ? "period"
        : undefined;
    if (kind !== undefined) {
      run = run === "period" || kind === "period" ? "period" : "comma";
    } else if (!SPACE.test(ch)) {
      if (run !== undefined && seenText) marks.push(run);
      run = undefined;
      seenText = true;
    }
  }
  return marks;
}

// 無音とみなす振幅（16 ビットのサンプルの絶対値）。
function noiseAmplitude(noiseDb: number): number {
  return 32768 * 10 ** (noiseDb / 20);
}

// すべてのチャンネルのサンプルの絶対値が、無音とみなす振幅より小さいフレームの並びを、長さによらずすべて返す。
export function findSilences(wav: Wav, noiseDb: number): Silence[] {
  const total = frameCount(wav);
  const limit = noiseAmplitude(noiseDb);
  const out: Silence[] = [];
  let start = -1;
  for (let frame = 0; frame < total; frame++) {
    let silent = true;
    for (let ch = 0; ch < wav.channels; ch++) {
      if (Math.abs(wav.samples[frame * wav.channels + ch]) >= limit) {
        silent = false;
        break;
      }
    }
    if (silent) {
      if (start < 0) start = frame;
    } else if (start >= 0) {
      out.push({ start, end: frame });
      start = -1;
    }
  }
  if (start >= 0) out.push({ start, end: total });
  return out;
}

// 句読点にあたる無音。音声の先頭と末尾に接する無音を除き、`minSeconds` 秒以上の無音を、順に返す。
export function pauseSilences(
  wav: Wav,
  params: SilenceParams = SILENCE,
): Silence[] {
  const total = frameCount(wav);
  const min = params.minSeconds * wav.sampleRate;
  return findSilences(wav, params.noiseDb).filter(
    (s) => s.start > 0 && s.end < total && s.end - s.start >= min,
  );
}

export interface MarkedSilence extends Silence {
  mark: Mark;
}

// 句読点にあたる無音と、`say` の文の途中の句読点を、前から順に対応づける。数が合わないときは undefined。
export function matchMarks(
  silences: Silence[],
  marks: Mark[],
): MarkedSilence[] | undefined {
  if (silences.length !== marks.length) return undefined;
  return silences.map((s, i) => ({ ...s, mark: marks[i] }));
}

// 取り除く範囲（フレーム）。「、」にあたる無音のうち `pause` 秒より長いものから、真ん中を取り除き、
// 前後に `pause` 秒の半分ずつを残す。
export function trimRanges(
  silences: MarkedSilence[],
  pauseSeconds: number,
  sampleRate: number,
): Silence[] {
  const keep = Math.round(pauseSeconds * sampleRate);
  const head = Math.floor(keep / 2);
  const tail = keep - head;
  return silences
    .filter((s) => s.mark === "comma" && s.end - s.start > keep)
    .map((s) => ({ start: s.start + head, end: s.end - tail }));
}

// 範囲（フレーム）を取り除いた音声を返す。範囲は前から順に並び、重ならないものとする。
export function removeRanges(wav: Wav, ranges: Silence[]): Wav {
  const removed = ranges.reduce((sum, r) => sum + (r.end - r.start), 0);
  const samples = new Int16Array((frameCount(wav) - removed) * wav.channels);
  let from = 0;
  let to = 0;
  for (const range of [...ranges, { start: frameCount(wav), end: 0 }]) {
    const part = wav.samples.subarray(
      from * wav.channels,
      range.start * wav.channels,
    );
    samples.set(part, to);
    to += part.length;
    from = range.end;
  }
  return { ...wav, samples };
}

export type TrimResult =
  | {
      ok: true;
      wav: Wav;
      silences: MarkedSilence[];
      // 取り除いた長さ（秒）。
      removedSeconds: number;
    }
  | { ok: false; message: string };

// 「、」にあたる無音を `pause` 秒まで切り詰めた音声を返す。
export function trimCommaPauses(
  wav: Wav,
  say: string,
  pauseSeconds: number,
  params: SilenceParams = SILENCE,
): TrimResult {
  const found = pauseSilences(wav, params);
  const marks = sentenceMarks(say);
  const silences = matchMarks(found, marks);
  if (silences === undefined) {
    return {
      ok: false,
      message: `文の途中の句読点（「、」「。」など）が ${marks.length} か所あるが、音声の中の ${params.minSeconds} 秒以上の無音が ${found.length} か所ある`,
    };
  }
  const ranges = trimRanges(silences, pauseSeconds, wav.sampleRate);
  const removed = ranges.reduce((sum, r) => sum + (r.end - r.start), 0);
  return {
    ok: true,
    wav: removeRanges(wav, ranges),
    silences,
    removedSeconds: removed / wav.sampleRate,
  };
}
