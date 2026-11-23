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

export function MoveScene() {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const { at, endOf } = useCues();
  const { lessonNumber, lessonTitle } = useLesson();

  // 合図（文の開始からの秒は、音声を作ったあとで言葉に合わせ直す）。
  const s1In = at("m1", 1.12); // 「変数 s1 に」
  const dataIn = at("m1", 2.02); // 「String の値を入れます」
  const arrowIn = at("m1", 2.69);
  const ownerNote = at("m2", 4.39); // 「所有者になります」
  const sharing = at("m3", 0.22);
  const moveStart = at("m4", 3.43); // 「所有権が s2 に移ります」
  const naming = at("m4", 5.41); // 「これをムーブと呼びます」
  const noCopy = at("m5", 4.64); // 「ヒープ上の文字列は複製されません」
  const unusable = at("m6", 2.32); // 「もう使えません」
  const onlyS2 = at("m6", 3.48); // 「使えるのは s2 だけです」

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
  const copyOut = progress(frame, endOf("m5", 0.2), 14, easeOut);
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
    "--mark-p": progress(frame, naming + FRAMES.exit + 10, 14, easeOut),
  } as CSSProperties;

  const sy = ownerY + 36;

  return (
    <Camera {...VIEW}>
      <div
        className="video-stage"
        data-lesson-label={`${lessonNumber} ${lessonTitle}`}
        style={{ opacity: sceneOpacity(frame, durationInFrames) }}
      >
        <h1>代入すると、所有権が移る</h1>
        <div style={{ position: "absolute", left: 72, top: 149, width: 480 }}>
          <CodeBlock
            block="move"
            marks={[
              {
                line: 2,
                text: "s1",
                progress: markAt(frame, at("m1", 1.01), at("m4")),
              },
              { line: 3, text: "s1", progress: markAt(frame, at("m4", 0.66)) },
              { line: 4, text: "s2", progress: markAt(frame, onlyS2 + 10) },
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
            <rect
              className="fill-muted"
              x={0}
              y={0}
              width={232}
              height={280}
              rx={20}
              stroke="none"
            />
            <rect
              className="fill-muted"
              x={296}
              y={0}
              width={196}
              height={280}
              rx={20}
              stroke="none"
            />
            <text className="fill-muted" x={28} y={40} fontSize={18}>
              スタック
            </text>
            <text className="fill-muted" x={324} y={40} fontSize={18}>
              ヒープ
            </text>

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
