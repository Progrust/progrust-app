// テスト用のレッスン（tools/lesson-videos.test.ts）。
import "./alpha.css";
import { Panel } from "./parts/Panel";

function TitleScene() {
  return <Panel text="LESSON-MARKER-alpha" />;
}

export default {
  title: TitleScene,
};
