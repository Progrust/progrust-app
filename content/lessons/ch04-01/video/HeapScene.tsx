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

// 見出しを出す・入れ替える。前の見出しが消え始めてから 6 フレーム後に、次の見出しが現れ始める。
function headingStyle(frame: number, inAt: number, outAt: number | null) {
  const shown = enter(frame, inAt, { rise: 14 });
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

  // 合図（文の開始からの秒）。
  const headingIn = at("h1", 0.05); // 「いっぽう」
  const heapIn = at("h1", 3.38); // 「ヒープという領域に置きます」
  const dataIn = at("h3", 1.79); // 「ハローの五文字を」
  const stackIn = at("h4", 1.32); // 「スタックにある」
  const ptrIn = at("h4", 2.34); // 「ピーティーアールの箱は」
  const arrowIn = at("h4", 3.76); // 「ヒープのデータの場所を指すあたい」
  const freed = at("h5", 2.09); // 「解放して」
  const gcNote = at("h7", 0.7); // 「ラストには」
  const noGc = at("h7", 1.52); // 「ガベージコレクションがありません」
  const mistakes = at("h8", 0.05); // 「シーのように手で解放する言語では」
  const doubleFree = at("h9", 0.79); // 「同じ領域を二回解放してしまう」
  const useAfterFree = at("h10", 0.98); // 「解放したあとの領域を使ってしまう」
  const harmful = at("h11", 0.05); // 「どちらも」

  const heapArea = progress(frame, heapIn, 12, easeOut);
  const stackArea = progress(frame, stackIn, 12, easeOut);
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
        <h1 style={headingStyle(frame, headingIn, mistakes)}>
          大きさが決まらないデータは、ヒープに置く
        </h1>
        <h1 style={headingStyle(frame, mistakes + 6, null)}>
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
            {/* スタックとヒープの領域。ナレーションが触れるときに出す */}
            <g opacity={stackArea}>
              <rect
                className="fill-muted"
                x={0}
                y={0}
                width={300}
                height={300}
                rx={20}
                stroke="none"
              />
              <text className="fill-muted" x={28} y={40} fontSize={18}>
                スタック
              </text>
            </g>
            <g opacity={heapArea}>
              <rect
                className="fill-muted"
                x={460}
                y={0}
                width={540}
                height={300}
                rx={20}
                stroke="none"
              />
              <text className="fill-muted" x={488} y={40} fontSize={18}>
                ヒープ
              </text>
            </g>

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
