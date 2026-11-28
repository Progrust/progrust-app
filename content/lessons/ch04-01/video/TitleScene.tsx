// 表紙。レッスン名（LessonTitle）とサブタイトルだけを置き、寄せない。
// レッスン名はシーンの始まりとともに出し、サブタイトルはナレーションが触れるときに出す。
import { useCurrentFrame, useVideoConfig } from "remotion";
import {
  enter,
  LessonTitle,
  sceneOpacity,
  useCues,
  useLesson,
} from "../../../../src/video";

export function TitleScene() {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const { at } = useCues();
  const { chapterNumber, chapterTitle } = useLesson();
  // 合図（文の開始からの秒）。
  const subtitleIn = at("t2", 1.36); // 「スタックとヒープという」
  const subtitle = enter(frame, subtitleIn, { duration: 20, rise: 20 });
  return (
    <div
      className="video-stage"
      data-layout="title"
      data-chapter-label={`第${chapterNumber}章 ${chapterTitle}`}
      style={{ opacity: sceneOpacity(frame, durationInFrames) }}
    >
      <LessonTitle />
      <p
        style={{
          opacity: subtitle.opacity,
          transform: `translateY(${subtitle.translateY}px)`,
        }}
      >
        スタックとヒープの違いと、解放で起きる問題
      </p>
    </div>
  );
}
