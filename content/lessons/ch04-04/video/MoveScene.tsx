// ムーブの図解。左にコード、右にスタックとヒープの図を置き、代入で所有者の印が s1 から s2 へ移る様子を、
// ナレーションに合わせて見せる（コード + 図の左右の配置。1.16 倍に寄せる）。
import type { CSSProperties } from "react";
import { useCurrentFrame, useVideoConfig } from "remotion";
import {
  Camera,
  clamp01,
  CodeBlock,
  easeInOut,
  easeOut,
  enter,
  exit,
  FRAMES,
  lerp,
  progress,
  sceneOpacity,
  stagger,
  useCues,
  useLesson,
} from "../../../../src/video";

const VIEW = { x: 22, y: 0, scale: 1.16 };
const CELLS = [
  { ch: "h", x: 324 },
  { ch: "i", x: 400 },
];

// on で引き始め、off で消す（off を省くと消さない）マーカーの進み具合。
function markAt(frame: number, on: number, off?: number): number {
  const drawn = progress(frame, on, FRAMES.draw, easeOut);
  const erased =
    off === undefined ? 0 : progress(frame, off, FRAMES.exit + 2, easeOut);
  return clamp01(drawn - erased);
}

function rise(opacity: number, translateY: number): CSSProperties {
  return { opacity, transform: `translateY(${translateY}px)` };
}

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

export function MoveScene() {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const { at, endOf } = useCues();
  const { lessonNumber, lessonTitle } = useLesson();

  // 合図（文の開始からの秒）。
  const codeIn = at("m1", 0.05); // 「左のコードは」
  const line2 = at("m2", 2.85); // 「変数エスワンに」
  const dataIn = at("m3", 2.04); // 「エイチとアイは」
  const heapIn = at("m3", 2.91); // 「ヒープに置かれ」
  const stackIn = at("m3", 3.98); // 「スタックにある」
  const s1In = at("m3", 4.99); // 「エスワンの箱が」
  const arrowIn = at("m3", 6.13); // 「その場所を指します」
  const ownerHeading = at("m4", 0.05); // 「ラストの変数は」
  const ownerNote = at("m4", 4.85); // 「所有者になります」
  const sharing = at("m5", 0.05); // 「パイソンやジャバスクリプトの代入では」
  const line3 = at("m6", 2.18); // 「三行目で」
  const line3S1 = at("m6", 3.12); // 「エスワンをエスツーに代入すると」
  const moveHeading = at("m6", 5.22); // 「所有権が」
  const moveStart = at("m6", 6.19); // 「エスツーに移ります」
  const naming = at("m7", 0.05); // 「このように」
  const namingMark = at("m7", 2.57); // 「ムーブと呼びます」
  const noCopy = at("m8", 4.8); // 「ヒープ上の文字列は複製されません」
  const unusable = at("m10", 2.16); // 「もう使えません」
  const line4 = at("m10", 4.15); // 「四行目のように」
  const onlyS2 = at("m10", 5.41); // 「使えるのは」

  const code = enter(frame, codeIn);
  const heapArea = progress(frame, heapIn, 12, easeOut);
  const stackArea = progress(frame, stackIn, 12, easeOut);
  const s1Box = enter(frame, s1In, { duration: 14, rise: 10 });
  const drawn = progress(frame, arrowIn, FRAMES.lineDraw, easeInOut);
  const head = progress(frame, arrowIn + 14, 6, easeOut);
  const note = progress(frame, ownerNote, 12, easeOut);
  // 所有者の印（塗り・「所有者」・矢印の根元）が s1 の箱から s2 の箱へ移る。
  const moveP = progress(frame, moveStart, FRAMES.move, easeInOut);
  const ownerY = lerp(64, 160, moveP);
  const arrowEnd = lerp(140, 156, moveP);
  const leftBehind = progress(frame, moveStart + 10, 14, easeOut);
  const s2Label = progress(frame, moveStart + 18, 12, easeOut);
  const copyIn = progress(frame, noCopy, 14, easeOut);
  const copyOut = progress(frame, endOf("m9", 0.2), 14, easeOut);
  const copyShown = Math.min(copyIn, 1 - copyOut);
  const unusableP = progress(frame, unusable, 12, easeOut);
  const usable = enter(frame, onlyS2);

  const calloutA = enter(frame, sharing);
  const calloutAOut = exit(frame, naming);
  const calloutB = enter(frame, naming + FRAMES.exit);
  const calloutBox: CSSProperties = {
    position: "absolute",
    left: 584,
    top: 453,
    maxWidth: 492,
  };
  const markStyle = {
    "--mark-p": progress(frame, namingMark, 14, easeOut),
  } as CSSProperties;

  const sy = ownerY + 36;

  return (
    <Camera {...VIEW}>
      <div
        className="video-stage"
        data-lesson-label={`${lessonNumber} ${lessonTitle}`}
        style={{ opacity: sceneOpacity(frame, durationInFrames) }}
      >
        <h1 style={headingStyle(frame, ownerHeading, moveHeading)}>
          変数は、値の所有者になる
        </h1>
        <h1 style={headingStyle(frame, moveHeading + 6, null)}>
          代入すると、所有権が移る
        </h1>
        <div
          style={{
            position: "absolute",
            left: 72,
            top: 149,
            width: 480,
            ...rise(code.opacity, code.translateY),
          }}
        >
          <CodeBlock
            block="move"
            marks={[
              {
                line: 2,
                text: "s1",
                progress: markAt(frame, line2, line3),
              },
              { line: 3, text: "s1", progress: markAt(frame, line3S1) },
              { line: 4, text: "s2", progress: markAt(frame, line4) },
            ]}
          />
        </div>
        <p
          style={{
            position: "absolute",
            left: 72,
            top: 404,
            width: 480,
            ...rise(usable.opacity, usable.translateY),
          }}
        >
          ムーブの後、s2 は<strong className="c-ok">使えます</strong>。
          <br />
          s1 は<strong className="c-ng">使えません</strong>。
        </p>
        <figure
          className="diagram"
          style={{ position: "absolute", left: 584, top: 149, width: 492 }}
        >
          <svg
            viewBox="0 0 492 280"
            width={492}
            height={280}
            role="img"
            aria-label="s1 が持っていた文字列の所有権が s2 に移り、s1 は使えなくなる図。ヒープ上の文字列は複製されない"
          >
            {/* スタックとヒープの領域。ナレーションが触れるときに出す */}
            <g opacity={stackArea}>
              <rect
                className="fill-muted"
                x={0}
                y={0}
                width={232}
                height={280}
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
                x={296}
                y={0}
                width={196}
                height={280}
                rx={20}
                stroke="none"
              />
              <text className="fill-muted" x={324} y={40} fontSize={18}>
                ヒープ
              </text>
            </g>

            {/* ヒープ上の文字列 */}
            {CELLS.map((cell, i) => (
              <g
                key={i}
                opacity={progress(frame, stagger(dataIn, i), 12, easeOut)}
              >
                <rect
                  className="fill-accent stroke-accent"
                  x={cell.x}
                  y={116}
                  width={64}
                  height={64}
                  rx={8}
                />
                <text
                  className="code fill-accent"
                  x={cell.x + 32}
                  y={148}
                  textAnchor="middle"
                  dominantBaseline="central"
                >
                  {cell.ch}
                </text>
              </g>
            ))}

            {/* 起こらないこと: ヒープ上の文字列の複製（破線のセルで一時的に見せて消す） */}
            <g opacity={copyShown}>
              {CELLS.map((cell, i) => (
                <g key={i}>
                  <rect
                    className="fill-surface stroke-muted"
                    x={cell.x}
                    y={188}
                    width={64}
                    height={56}
                    rx={8}
                    strokeDasharray="8 6"
                  />
                  <text
                    className="code fill-muted"
                    x={cell.x + 32}
                    y={216}
                    textAnchor="middle"
                    dominantBaseline="central"
                  >
                    {cell.ch}
                  </text>
                </g>
              ))}
              <path
                className="stroke-ng"
                d="M324 259 l14 14 M338 259 l-14 14"
                strokeWidth={3}
              />
              <text
                className="fill-ng"
                x={348}
                y={266}
                fontSize={18}
                fontWeight={500}
                dominantBaseline="central"
              >
                複製されない
              </text>
            </g>

            {/* ムーブした後の s1: 破線の無効の箱と、矢印の跡 */}
            <g opacity={leftBehind}>
              <rect
                className="fill-surface stroke-muted"
                x={24}
                y={64}
                width={184}
                height={72}
                rx={12}
                strokeDasharray="8 6"
              />
              <path
                className="stroke-muted"
                d="M208 100 C 256 100, 264 140, 310 140"
                strokeDasharray="8 6"
              />
              <text
                className="code fill-muted"
                x={52}
                y={100}
                dominantBaseline="central"
              >
                s1
              </text>
            </g>
            <text
              className="fill-muted"
              x={188}
              y={100}
              textAnchor="end"
              dominantBaseline="central"
              fontSize={18}
              fontWeight={500}
              opacity={unusableP}
            >
              使えない
            </text>

            {/* 所有者の印。塗り・「所有者」・矢印の根元が一緒に移る */}
            <g
              opacity={s1Box.opacity}
              transform={`translate(0 ${s1Box.translateY})`}
            >
              <rect
                className="fill-primary"
                x={24}
                y={ownerY}
                width={184}
                height={72}
                rx={12}
                stroke="none"
              />
              <text
                className="fill-on"
                x={188}
                y={sy}
                textAnchor="end"
                dominantBaseline="central"
                fontSize={18}
                fontWeight={500}
                opacity={note}
              >
                所有者
              </text>
              <text
                className="code fill-on"
                x={52}
                y={100}
                dominantBaseline="central"
                opacity={1 - progress(frame, moveStart, 8, easeOut)}
              >
                s1
              </text>
              <text
                className="code fill-on"
                x={52}
                y={196}
                dominantBaseline="central"
                opacity={s2Label}
              >
                s2
              </text>
            </g>
            <path
              className="stroke-primary"
              d={`M208 ${sy} C 256 ${sy}, 264 ${arrowEnd}, 310 ${arrowEnd}`}
              strokeWidth={3}
              pathLength={1}
              strokeDasharray="1 1"
              strokeDashoffset={1 - drawn}
              opacity={drawn > 0 ? 1 : 0}
            />
            <path
              className="stroke-primary"
              d={`M302 ${arrowEnd - 8} L 312 ${arrowEnd} L 302 ${arrowEnd + 8}`}
              strokeWidth={3}
              opacity={head}
            />
          </svg>
        </figure>
        <div
          className="callout info"
          style={{
            ...calloutBox,
            ...rise(
              Math.min(calloutA.opacity, calloutAOut.opacity),
              calloutA.translateY,
            ),
          }}
        >
          Python や JavaScript の代入は、同じオブジェクトを共有します。
        </div>
        <div
          className="callout info"
          style={{
            ...calloutBox,
            ...rise(calloutB.opacity, calloutB.translateY),
          }}
        >
          所有権が移ることを、
          <span className="mark" style={markStyle}>
            ムーブ
          </span>
          と呼びます。
        </div>
      </div>
    </Camera>
  );
}
