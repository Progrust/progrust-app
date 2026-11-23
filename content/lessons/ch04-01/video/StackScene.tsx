// スタックとスタックフレーム。左にコード、右にスタックの図を置き、関数を呼ぶとフレームが積まれ、
// 戻ると消える様子を、ナレーションに合わせて見せる（コード + 図の左右の配置。1.16 倍に寄せる）。
import { useCurrentFrame, useVideoConfig } from "remotion";
import {
  Camera,
  clamp01,
  CodeBlock,
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

// on で引き始め、off で消す（off を省くと消さない）マーカーの進み具合。
function markAt(frame: number, on: number, off?: number): number {
  const drawn = progress(frame, on, FRAMES.draw, easeOut);
  const erased =
    off === undefined ? 0 : progress(frame, off, FRAMES.exit + 2, easeOut);
  return clamp01(drawn - erased);
}

export function StackScene() {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const { at } = useCues();
  const { lessonNumber, lessonTitle } = useLesson();

  // 合図（文の開始からの秒は、音声を作ったあとで言葉に合わせ直す）。
  const mainStart = at("k1", 2.62); // 「main のスタックフレームが積まれます」
  const call = at("k2", 1.55); // 「その上に add のフレームが積まれ」
  const args = at("k2", 3.81); // 「引数の a と b が置かれます」
  const ret = at("k3", 1.14); // 「add から戻ると」
  const result = at("k3", 3.2); // 「戻り値の 3 が y に入ります」
  const note = at("k4", 0.24);

  const mainIn = enter(frame, mainStart, { duration: 14, rise: 10 });
  const addIn = enter(frame, call, { duration: 14, rise: 10 });
  const addOut = exit(frame, ret, { duration: 12 });
  const argA = progress(frame, args, 12, easeOut);
  const argB = progress(frame, stagger(args, 1), 12, easeOut);
  const yIn = progress(frame, result, 12, easeOut);
  // main のフレームは、add を実行しているあいだだけ補助の色にする（色クラスの違う図形を不透明度で入れ替える）。
  const mainActive = clamp01(
    1 -
      progress(frame, call, FRAMES.dim, easeOut) +
      progress(frame, ret, FRAMES.dim, easeOut),
  );
  const callout = enter(frame, note);

  const mainLabel = (cls: string) => (
    <>
      <text className={`code ${cls}`} x={52} y={218} dominantBaseline="central">
        main
      </text>
      <g opacity={yIn}>
        <text
          className={`code ${cls}`}
          x={300}
          y={218}
          dominantBaseline="central"
        >
          y
        </text>
        <text
          className={`code ${cls}`}
          x={440}
          y={218}
          textAnchor="end"
          dominantBaseline="central"
        >
          3
        </text>
      </g>
    </>
  );

  return (
    <Camera {...VIEW}>
      <div
        className="video-stage"
        data-lesson-label={`${lessonNumber} ${lessonTitle}`}
        style={{ opacity: sceneOpacity(frame, durationInFrames) }}
      >
        <h1>関数を呼ぶと、スタックフレームが積まれる</h1>
        <div style={{ position: "absolute", left: 72, top: 149, width: 480 }}>
          <CodeBlock
            block="stack"
            marks={[
              {
                line: 5,
                text: "main",
                progress: markAt(frame, at("k1", 0.11), at("k2")),
              },
              {
                line: 6,
                text: "add(1, 2)",
                progress: markAt(frame, at("k2", 0.48), ret),
              },
              { line: 6, text: "y", progress: markAt(frame, result) },
            ]}
          />
        </div>
        <figure
          className="diagram"
          style={{ position: "absolute", left: 584, top: 149, width: 492 }}
        >
          <svg
            viewBox="0 0 492 280"
            width={492}
            height={280}
            role="img"
            aria-label="スタックの上に、main のフレームと、その上に積まれた add のフレームがある図。add から戻ると add のフレームが消え、main のフレームの y に 3 が入る"
          >
            <rect
              className="fill-muted"
              x={0}
              y={0}
              width={492}
              height={280}
              rx={20}
              stroke="none"
            />
            <text className="fill-muted" x={28} y={40} fontSize={18}>
              スタック
            </text>

            {/* main のフレーム。実行中は主役、add を実行しているあいだは補助の色 */}
            <g
              opacity={mainIn.opacity}
              transform={`translate(0 ${mainIn.translateY})`}
            >
              <g opacity={1 - mainActive}>
                <rect
                  className="fill-accent stroke-accent"
                  x={24}
                  y={180}
                  width={444}
                  height={76}
                  rx={12}
                />
                {mainLabel("fill-accent")}
              </g>
              <g opacity={mainActive}>
                <rect
                  className="fill-primary"
                  x={24}
                  y={180}
                  width={444}
                  height={76}
                  rx={12}
                  stroke="none"
                />
                {mainLabel("fill-on")}
              </g>
            </g>

            {/* add のフレーム。呼ぶと main の上に積まれ、戻ると丸ごと消える */}
            <g
              opacity={Math.min(addIn.opacity, addOut.opacity)}
              transform={`translate(0 ${addIn.translateY})`}
            >
              <rect
                className="fill-primary"
                x={24}
                y={84}
                width={444}
                height={76}
                rx={12}
                stroke="none"
              />
              <text
                className="code fill-on"
                x={52}
                y={122}
                dominantBaseline="central"
              >
                add
              </text>
              <g opacity={argA}>
                <text
                  className="code fill-on"
                  x={216}
                  y={122}
                  dominantBaseline="central"
                >
                  a
                </text>
                <text
                  className="code fill-on"
                  x={280}
                  y={122}
                  textAnchor="end"
                  dominantBaseline="central"
                >
                  1
                </text>
              </g>
              <g opacity={argB}>
                <text
                  className="code fill-on"
                  x={376}
                  y={122}
                  dominantBaseline="central"
                >
                  b
                </text>
                <text
                  className="code fill-on"
                  x={440}
                  y={122}
                  textAnchor="end"
                  dominantBaseline="central"
                >
                  2
                </text>
              </g>
            </g>
          </svg>
        </figure>
        <div
          className="callout info"
          style={{
            position: "absolute",
            left: 584,
            top: 453,
            maxWidth: 492,
            opacity: callout.opacity,
            transform: `translateY(${callout.translateY}px)`,
          }}
        >
          積んで外すだけなので、スタックの確保と解放は
          <strong className="c-primary">とても速い</strong>です。
        </div>
      </div>
    </Camera>
  );
}
