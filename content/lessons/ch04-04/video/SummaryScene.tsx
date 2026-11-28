// まとめ。見出しと項目を、読み上げに合わせて 1 つずつ出す。最後のシーンなので、終わりで消さない（1.35 倍に寄せる）。
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
  { id: "z2", text: "代入すると、所有権は新しい変数に移る（ムーブ）" },
  { id: "z3", text: "ムーブでコピーされるのは、スタック上の部分だけ" },
  { id: "z4", text: "ムーブした元の変数を使うと、エラーになる" },
];

export function SummaryScene() {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const { at } = useCues();
  const { lessonNumber, lessonTitle } = useLesson();
  const heading = enter(frame, at("z1", 0.83), { rise: 14 }); // 「このレッスンで分かったことを」
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
        <h1
          style={{
            opacity: heading.opacity,
            transform: `translateY(${heading.translateY}px)`,
          }}
        >
          このレッスンのまとめ
        </h1>
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
