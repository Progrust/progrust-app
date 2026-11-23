// 動画プレーヤーの計算（documents/spec/screens/video-screen.md「プレーヤーの操作」）。
// 画面の部品から分けて、自動テストで確かめられるようにする。
// フレーム数は、動画の先頭を 0 とする。
import { FPS, type Timeline } from "./timeline";

// 5 秒戻る・5 秒進むの秒数。
export const SKIP_SECONDS = 5;

// 動画全体で数えた文の位置。
export type PlayerSentence = {
  id: string;
  // 文の頭（その文の開始位置）。
  from: number;
};

// タイムラインから、動画全体の文の一覧を、開始位置の順に作る。
export function sentenceStarts(timeline: Timeline): PlayerSentence[] {
  return timeline.scenes
    .flatMap((scene) =>
      scene.sentences.map((sentence) => ({
        id: sentence.id,
        from: scene.from + sentence.from,
      })),
    )
    .sort((a, b) => a.from - b.from);
}

// 動画の最後のフレーム。Player が最後に描くのは、長さ − 1 のフレームである。
export function lastFrame(timeline: Timeline): number {
  return Math.max(timeline.durationInFrames - 1, 0);
}

// 再生中の文。開始位置が再生位置と同じかそれより前の文のうち、最後の文。
// 最初の文が始まる前は、最初の文。文が無いときは undefined。
export function currentSentence(
  sentences: PlayerSentence[],
  frame: number,
): PlayerSentence | undefined {
  let current: PlayerSentence | undefined = sentences[0];
  for (const sentence of sentences) {
    if (sentence.from <= frame) {
      current = sentence;
    } else {
      break;
    }
  }
  return current;
}

// 文の頭のフレーム。
export function sentenceHeadFrame(sentence: PlayerSentence): number {
  return sentence.from;
}

// 5 秒戻る・5 秒進むの移動先。先頭と末尾を越えるときは、先頭・末尾にする。
export function skipTarget(
  timeline: Timeline,
  frame: number,
  direction: "back" | "forward",
): number {
  const delta = SKIP_SECONDS * FPS * (direction === "back" ? -1 : 1);
  return Math.min(Math.max(frame + delta, 0), lastFrame(timeline));
}

// シークバーの位置（0 から 1 の割合）。
export function seekBarRatio(timeline: Timeline, frame: number): number {
  if (timeline.durationInFrames <= 0) {
    return 0;
  }
  return Math.min(Math.max(frame / timeline.durationInFrames, 0), 1);
}

// シークバーの位置（0 から 1 の割合）を押したときの移動先。末尾を越えるときは、末尾にする。
export function frameAtRatio(timeline: Timeline, ratio: number): number {
  const clamped = Math.min(Math.max(ratio, 0), 1);
  return Math.min(
    Math.round(clamped * timeline.durationInFrames),
    lastFrame(timeline),
  );
}

// シークバーに印を出す、各シーンの開始位置（0 から 1 の割合）。
export function sceneMarkRatios(timeline: Timeline): number[] {
  return timeline.scenes.map((scene) => seekBarRatio(timeline, scene.from));
}

// 秒を「分:秒」にする（例: 42 → 0:42）。小数点以下は切り捨てる。
function formatClock(totalSeconds: number): string {
  const whole = Math.max(Math.floor(totalSeconds), 0);
  const minutes = Math.floor(whole / 60);
  const seconds = whole % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

// 時間の表示（「0:42 / 1:44」の形）。現在の位置は切り捨て、全体の長さは切り上げる
// （末尾で、現在の位置が全体の長さを超えないようにする）。
export function formatTime(frame: number, durationInFrames: number): string {
  const total = Math.ceil(durationInFrames / FPS);
  return `${formatClock(frame / FPS)} / ${formatClock(total)}`;
}

// 末尾に着いたことの判定。再生で着いたときも、シークで末尾に移したときも、
// 位置が最後のフレームに着いていれば真とする。
export function hasReachedEnd(timeline: Timeline, frame: number): boolean {
  return timeline.durationInFrames > 0 && frame >= lastFrame(timeline);
}
