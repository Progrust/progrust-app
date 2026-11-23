// テスト用のレッスン（tools/content-check/render.test.ts）。エラーなく描画できる動画。
import { useCurrentFrame } from "remotion";
import {
  CodeBlock,
  LessonTitle,
  useCues,
  useLesson,
} from "../../../../../../../src/video";
import "./ok.css";

function TitleScene() {
  const { chapterTitle } = useLesson();
  return (
    <div className="video-stage" data-layout="title">
      <LessonTitle />
      <p>{chapterTitle}</p>
    </div>
  );
}

// バンドルで CSS が変換されたことを、描画の中で確かめる。違っていたら、描画のエラーにする。
function CssProbe() {
  const rules = [...document.styleSheets].flatMap((sheet) => [
    ...sheet.cssRules,
  ]);
  // ok.css の 2 つの規則（`:root` と `.render-probe`）。
  const probes = rules.filter(
    (rule): rule is CSSStyleRule =>
      rule instanceof CSSStyleRule && rule.cssText.includes("render-probe"),
  );
  if (
    probes.length !== 2 ||
    probes.some((rule) => !rule.selectorText.startsWith('[data-lesson="ok"]'))
  ) {
    throw new Error(
      `レッスン専用の CSS が閉じ込められていない: ${probes.map((r) => r.selectorText).join(" / ")}`,
    );
  }
  const imports = rules.filter(
    (rule) =>
      rule instanceof CSSImportRule && /^(?:https?:)?\/\//.test(rule.href),
  );
  if (imports.length > 0) {
    throw new Error("デザインシステムの CSS の書体の @import が残っている");
  }
  return <div className="render-probe" />;
}

function CodeScene() {
  const frame = useCurrentFrame();
  const { at } = useCues();
  const marked = frame >= at("c2", 0.5) ? 1 : 0;
  return (
    <div className="video-stage" data-layout="code">
      <CodeBlock
        block="main"
        scopes={[{ label: "s", from: 2, to: 3 }]}
        marks={[{ line: 2, text: "s", progress: marked }]}
      />
      <CssProbe />
    </div>
  );
}

export default {
  title: TitleScene,
  code: CodeScene,
};
