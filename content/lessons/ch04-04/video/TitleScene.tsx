// 表紙。レッスン名（LessonTitle）とサブタイトルだけを置き、寄せない。
import { useCurrentFrame, useVideoConfig } from "remotion";
import {
  enter,
  LessonTitle,
  sceneOpacity,
  useLesson,
} from "../../../../src/video";

export function TitleScene() {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const { chapterNumber, chapterTitle } = useLesson();
  const subtitle = enter(frame, 18, { duration: 20, rise: 20 });
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
        代入で、値の持ち主が移る仕組み
      </p>
    </div>
  );
}
