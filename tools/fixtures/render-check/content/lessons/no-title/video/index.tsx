// テスト用のレッスン（tools/content-check/render.test.ts）。最初のシーンに LessonTitle がない動画。
import { useLesson } from "../../../../../../../src/video";

function TitleScene() {
  const { lessonTitle } = useLesson();
  return (
    <div className="video-stage" data-layout="title">
      <h1>{lessonTitle}</h1>
    </div>
  );
}

export default {
  title: TitleScene,
};
