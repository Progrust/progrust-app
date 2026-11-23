// ヒープと解放。左にスタック、右にヒープの図を置き、ヒープにデータを置いてスタックから指すこと、
// 使い終わったら解放すること、手で解放する言語で起きる二重解放と解放後の使用を見せる（1.16 倍に寄せる）。
import type { CSSProperties } from "react";
import { useCurrentFrame, useVideoConfig } from "remotion";
import {
  Camera,
  clamp01,
  easeInOut,
  easeOut,
  enter,
  exit,
  FRAMES,
  progress,
  sceneOpacity,
  stagger,
  useCues,
  useLesson,
} from "../../../../src/video";

const VIEW = { x: 22, y: 0, scale: 1.16 };
const CELLS = ["h", "e", "l", "l", "o"];
const cellX = (i: number) => 520 + 72 * i;

// 見出しを入れ替える。前の見出しが消え始めてから 6 フレーム後に、次の見出しが現れ始める。
function headingStyle(
  frame: number,
  inAt: number | null,
  outAt: number | null,
) {
  const shown =
    inAt === null
      ? { opacity: 1, translateY: 0 }
      : enter(frame, inAt, { rise: 14 });
  const hidden =
    outAt === null ? 0 : progress(frame, outAt, FRAMES.exit, easeOut);
  return {
    position: "absolute" as const,
    left: 72,
    top: 64,
    opacity: Math.min(shown.opacity, 1 - hidden),
    transform: `translateY(${shown.translateY - 10 * hidden}px)`,
  };
}

// 矢印（スタックの ptr からヒープのデータへ）。cls は stroke-* の色クラス。
function Arrow({ cls, drawn }: { cls: string; drawn: number }) {
  const head = clamp01((drawn * FRAMES.lineDraw - 14) / 6);
  return (
    <>
      <path
        className={cls}
        d="M272 150 L 508 150"
        strokeWidth={3}
        pathLength={1}
        strokeDasharray="1 1"
        strokeDashoffset={1 - drawn}
        opacity={drawn > 0 ? 1 : 0}
      />
      <path
        className={cls}
        d="M498 140 L 508 150 L 498 160"
        strokeWidth={3}
        opacity={head}
      />
    </>
  );
}

export function HeapScene() {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const { at } = useCues();
  const { lessonNumber, lessonTitle } = useLesson();

  // 合図（文の開始からの秒は、音声を作ったあとで言葉に合わせ直す）。
  const dataIn = at("h1", 3.39); // 「ヒープという領域に置きます」
  const ptrIn = at("h2", 0.31); // 「スタックには」
  const arrowIn = at("h2", 2.51); // 「どこにあるかを指す値」
  const freed = at("h3", 2.32); // 「解放して」
  const gcNote = at("h4", 0.2);
  const noGc = at("h4", 3.66); // 「ガベージコレクションがありません」
  const doubleFree = at("h5", 4.84); // 「二重解放が起こりえます」
  const useAfterFree = at("h6", 2.79); // 「解放後の使用も起こりえます」
  const harmful = at("h6", 5.3); // 「どちらも、プログラムを壊す深刻な不具合です」

  const ptr = enter(frame, ptrIn, { duration: 14, rise: 10 });
  const drawn = progress(frame, arrowIn, FRAMES.lineDraw, easeInOut);
  const freedP = progress(frame, freed, 14, easeOut);
  const doubleP = progress(frame, doubleFree, 14, easeOut);
  const uafP = progress(frame, useAfterFree, 14, easeOut);
  const info = enter(frame, gcNote);
  const infoOut = exit(frame, harmful);
  const ng = enter(frame, harmful + FRAMES.exit);
  const markStyle = {
    "--mark-p": progress(frame, noGc, 14, easeOut),
  } as CSSProperties;

  const callout = (opacity: number, translateY: number) => ({
    position: "absolute" as const,
    left: 72,
    top: 473,
    maxWidth: 1000,
    opacity,
    transform: `translateY(${translateY}px)`,
  });

  return (
    <Camera {...VIEW}>
      <div
        className="video-stage"
        data-lesson-label={`${lessonNumber} ${lessonTitle}`}
        style={{ opacity: sceneOpacity(frame, durationInFrames) }}
      >
        <h1 style={headingStyle(frame, null, at("h5"))}>
          大きさが決まらないデータは、ヒープに置く
        </h1>
        <h1 style={headingStyle(frame, at("h5") + 6, null)}>
          手で解放すると、誤りが起こりうる
        </h1>
        <figure
          className="diagram"
          style={{ position: "absolute", left: 72, top: 149, width: 1000 }}
        >
          <svg
            viewBox="0 0 1000 300"
            width={1000}
            height={300}
            role="img"
            aria-label="スタックの ptr が、ヒープ上のデータの場所を指す図。解放したあとのデータをもう一度解放する二重解放と、解放したあとのデータを指して使う解放後の使用を、× で示す"
          >
            <rect
              className="fill-muted"
              x={0}
              y={0}
              width={300}
              height={300}
              rx={20}
              stroke="none"
            />
            <rect
              className="fill-muted"
              x={460}
              y={0}
              width={540}
              height={300}
              rx={20}
              stroke="none"
            />
            <text className="fill-muted" x={28} y={40} fontSize={18}>
              スタック
            </text>
            <text className="fill-muted" x={488} y={40} fontSize={18}>
              ヒープ
            </text>

            {/* ヒープ上のデータ。解放すると破線のセルに入れ替わる */}
            {CELLS.map((ch, i) => {
              const shown = progress(frame, stagger(dataIn, i), 12, easeOut);
              return (
                <g key={i} opacity={shown}>
                  <g opacity={1 - freedP}>
                    <rect
                      className="fill-accent stroke-accent"
                      x={cellX(i)}
                      y={120}
                      width={60}
                      height={60}
                      rx={8}
                    />
                    <text
                      className="code fill-accent"
                      x={cellX(i) + 30}
                      y={150}
                      textAnchor="middle"
                      dominantBaseline="central"
                    >
                      {ch}
                    </text>
                  </g>
                  <g opacity={freedP}>
                    <rect
                      className="fill-surface stroke-muted"
                      x={cellX(i)}
                      y={120}
                      width={60}
                      height={60}
                      rx={8}
                      strokeDasharray="8 6"
                    />
                    <text
                      className="code fill-muted"
                      x={cellX(i) + 30}
                      y={150}
                      textAnchor="middle"
                      dominantBaseline="central"
                    >
                      {ch}
                    </text>
                  </g>
                </g>
              );
            })}
            <text
              className="fill-muted"
              x={520}
              y={214}
              fontSize={18}
              fontWeight={500}
              dominantBaseline="central"
              opacity={freedP}
            >
              解放済み
            </text>

            {/* 二重解放 */}
            <g opacity={doubleP}>
              <path
                className="stroke-ng"
                d="M520 244 l14 14 M534 244 l-14 14"
                strokeWidth={3}
              />
              <text
                className="fill-ng"
                x={546}
                y={251}
                fontSize={18}
                fontWeight={500}
                dominantBaseline="central"
              >
                2 回目の解放
              </text>
            </g>

            {/* スタックの ptr と、ヒープを指す矢印。解放後の使用では誤りの赤に入れ替わる */}
            <g
              opacity={ptr.opacity}
              transform={`translate(0 ${ptr.translateY})`}
            >
              <rect
                className="fill-primary"
                x={28}
                y={112}
                width={244}
                height={76}
                rx={12}
                stroke="none"
              />
              <text
                className="code fill-on"
                x={56}
                y={150}
                dominantBaseline="central"
              >
                ptr
              </text>
            </g>
            <g opacity={1 - uafP}>
              <Arrow cls="stroke-primary" drawn={drawn} />
            </g>
            <g opacity={uafP}>
              <Arrow cls="stroke-ng" drawn={1} />
              <path
                className="stroke-ng"
                d="M318 100 l14 14 M332 100 l-14 14"
                strokeWidth={3}
              />
              <text
                className="fill-ng"
                x={342}
                y={107}
                fontSize={18}
                fontWeight={500}
                dominantBaseline="central"
              >
                解放後の使用
              </text>
            </g>
          </svg>
        </figure>
        <div
          className="callout info"
          style={callout(
            Math.min(info.opacity, infoOut.opacity),
            info.translateY,
          )}
        >
          Rust には、
          <span className="mark" style={markStyle}>
            ガベージコレクションがありません
          </span>
          。
        </div>
        <div className="callout ng" style={callout(ng.opacity, ng.translateY)}>
          二重解放も解放後の使用も、プログラムを壊す深刻な不具合です。
        </div>
      </div>
    </Camera>
  );
}
