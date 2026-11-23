// ムーブ済みの値を使ったエラー。ムーブした後の s1 を使うコードと、rustc の出力を上下に並べる。
// 出力を出すところで見出しを消し、コードと出力へ寄せる（1.2 倍）。
import type { CSSProperties } from "react";
import { useCurrentFrame, useVideoConfig } from "remotion";
import {
  Camera,
  cameraAt,
  clamp01,
  CodeBlock,
  easeOut,
  enter,
  exit,
  FRAMES,
  progress,
  sceneOpacity,
  useCues,
  useLesson,
} from "../../../../src/video";

// 見出しとコードを映す範囲と、見出しを消してコードと出力を映す範囲。
const VIEW_CODE = { x: 32, y: 0, scale: 1.2 };
const VIEW_OUTPUT = { x: 32, y: 109, scale: 1.2 };
const WIDTH = 987;

// on で引き始め、off で消す（off を省くと消さない）マーカー・波線の進み具合。
function drawAt(frame: number, on: number, off?: number): number {
  const drawn = progress(frame, on, FRAMES.draw, easeOut);
  const erased =
    off === undefined ? 0 : progress(frame, off, FRAMES.exit + 2, easeOut);
  return clamp01(drawn - erased);
}

function rise(opacity: number, translateY: number): CSSProperties {
  return { opacity, transform: `translateY(${translateY}px)` };
}

export function ErrorScene() {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const { at } = useCues();
  const { lessonNumber, lessonTitle } = useLesson();

  // 合図（文の開始からの秒は、音声を作ったあとで言葉に合わせ直す）。
  const line4 = at("e1", 0.32); // 「4 行目のように」
  const cannot = at("e1", 3.35); // 「コンパイルできません」
  const toOutput = at("e2"); // 出力を出す
  const wavy = at("e2", 1.52); // 「4 行目の s1 を」
  const reading = at("e3", 0.25); // 出力の「value moved here」の行を読む
  const moved = at("e3", 1.5); // 「3 行目の代入で」

  const view = cameraAt(frame, VIEW_CODE, VIEW_OUTPUT, toOutput);
  const heading = progress(frame, toOutput, FRAMES.sceneFade, easeOut);
  const ng = enter(frame, cannot);
  const ngOut = exit(frame, toOutput);
  const output = enter(frame, toOutput + FRAMES.exit);
  // 出力の「value moved here」の行（6・7 行目）と、それが指すコードの 3 行目だけを残して淡くする。
  const dim = progress(frame, reading, FRAMES.dim, easeOut);
  const codeDim = { 1: { dim }, 2: { dim }, 4: { dim }, 5: { dim } };
  const outputDim = Object.fromEntries(
    [1, 2, 3, 4, 5, 8, 9].map((n) => [n, { dim }]),
  );

  return (
    <Camera {...view}>
      <div
        className="video-stage"
        data-lesson-label={`${lessonNumber} ${lessonTitle}`}
        style={{ opacity: sceneOpacity(frame, durationInFrames) }}
      >
        <h1
          style={{
            opacity: 1 - heading,
            transform: `translateY(${-10 * heading}px)`,
          }}
        >
          ムーブした後の変数は、使えない
        </h1>
        <div style={{ position: "absolute", left: 72, top: 149, width: WIDTH }}>
          <CodeBlock
            block="use-after-move"
            marks={[
              { line: 4, text: "s1", progress: drawAt(frame, line4, wavy) },
              { line: 3, text: "s1", progress: drawAt(frame, moved) },
            ]}
            errors={[{ line: 4, text: "s1", progress: drawAt(frame, wavy) }]}
            lines={codeDim}
          />
        </div>
        <div
          className="callout ng"
          style={{
            position: "absolute",
            left: 72,
            top: 373,
            maxWidth: WIDTH,
            ...rise(Math.min(ng.opacity, ngOut.opacity), ng.translateY),
          }}
        >
          ムーブした後の s1 を使っているので、コンパイルできません。
        </div>
        <div
          className="fade-right"
          style={{
            position: "absolute",
            left: 72,
            top: 373,
            width: WIDTH,
            ...rise(output.opacity, output.translateY),
          }}
        >
          <CodeBlock block="error-output" lines={outputDim} />
        </div>
      </div>
    </Camera>
  );
}
