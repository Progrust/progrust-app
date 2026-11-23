// まとめ。項目を読み上げに合わせて 1 つずつ出す。最後のシーンなので、終わりで消さない（1.35 倍に寄せる）。
import { useCurrentFrame, useVideoConfig } from "remotion";
import {
  Camera,
  enter,
  sceneOpacity,
  useCues,
  useLesson,
} from "../../../../src/video";

const VIEW = { x: 8, y: 0, scale: 1.35 };

const ITEMS = [
  { id: "z1", text: "関数のスタックフレームは、呼ぶと積まれ、戻ると消える" },
  { id: "z2", text: "大きさが決まらないデータはヒープに置き、解放が要る" },
  { id: "z3", text: "Rust には、ガベージコレクションがない" },
  { id: "z4", text: "手での解放は、二重解放や解放後の使用を起こしうる" },
];

export function SummaryScene() {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const { at } = useCues();
  const { lessonNumber, lessonTitle } = useLesson();
  return (
    <Camera {...VIEW}>
      <div
        className="video-stage"
        data-layout="summary"
        data-lesson-label={`${lessonNumber} ${lessonTitle}`}
        style={{
          opacity: sceneOpacity(frame, durationInFrames, { last: true }),
        }}
      >
        <h1>このレッスンのまとめ</h1>
        <ul style={{ maxWidth: 820 }}>
          {ITEMS.map((item) => {
            const shown = enter(frame, at(item.id, 0.1), { duration: 18 });
            return (
              <li
                key={item.id}
                style={{
                  opacity: shown.opacity,
                  transform: `translateY(${shown.translateY}px)`,
                }}
              >
                {item.text}
              </li>
            );
          })}
        </ul>
      </div>
    </Camera>
  );
}
