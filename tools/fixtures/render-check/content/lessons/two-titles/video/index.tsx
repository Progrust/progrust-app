// テスト用のレッスン（tools/content-check/render.test.ts）。最初のシーンの途中から、LessonTitle が 2 つになる動画。
import { useCurrentFrame } from "remotion";
import { LessonTitle, useCues } from "../../../../../../../src/video";

function TitleScene() {
  const frame = useCurrentFrame();
  const { at } = useCues();
  return (
    <div className="video-stage" data-layout="title">
      <LessonTitle />
      {frame >= at("t1") ? <LessonTitle /> : null}
    </div>
  );
}

export default {
  title: TitleScene,
};
