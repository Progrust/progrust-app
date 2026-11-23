// テスト用のレッスン（tools/content-check/render.test.ts）。表紙のほかのシーンが、描画時にエラーになる動画。
import { useCurrentFrame } from "remotion";
import {
  CodeBlock,
  LessonTitle,
  useCues,
} from "../../../../../../../src/video";

function TitleScene() {
  return (
    <div className="video-stage" data-layout="title">
      <LessonTitle />
    </div>
  );
}

// 台本にない文 zz を指す合図。
function UnknownCueScene() {
  const frame = useCurrentFrame();
  const { at } = useCues();
  return <p>{frame >= at("zz") ? "後" : "前"}</p>;
}

// ほかのシーン（表紙）の文 t1 を指す合図。
function OtherSceneCueScene() {
  const frame = useCurrentFrame();
  const { endOf } = useCues();
  return <p>{frame >= endOf("t1") ? "後" : "前"}</p>;
}

// 範囲の一部だけが重なる scopes（2〜3 行目と 3〜4 行目）。
function OverlappingScopesScene() {
  return (
    <CodeBlock
      block="main"
      scopes={[
        { label: "a", from: 2, to: 3 },
        { label: "b", from: 3, to: 4 },
      ]}
    />
  );
}

// 読み込めない画像を素の img に渡す。例外にはならず、ブラウザの console のエラーになる。
function MissingImageScene() {
  return <img src="/missing-render-check.png" alt="読み込めない画像" />;
}

export default {
  title: TitleScene,
  unknown: UnknownCueScene,
  other: OtherSceneCueScene,
  scopes: OverlappingScopesScene,
  image: MissingImageScene,
};
