// テスト用のレッスン（tools/lesson-videos.test.ts）。cue-reference.json が無い。
import "./beta.css";
import { Panel } from "./parts/Panel";

function TitleScene() {
  return <Panel text="LESSON-MARKER-beta" />;
}

export default {
  title: TitleScene,
};
