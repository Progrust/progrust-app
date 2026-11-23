// 教材チェックツールの描画（検査「描画」「表紙」。documents/spec/content-guidelines/check-tool.md）で、
// Remotion のバンドラーに渡す入口。レッスンごとにバンドルする（render.ts）。
//
// シーンを 1 つずつ描く Composition「Scene」を置く。描くシーンと、台本・音声の長さ・レッスン定義・章の並びは、
// inputProps（RenderProps）で受け取る。シーンの部品は、バンドルのときに
// `progrust-render-check-lesson` を、そのレッスンの video/index.tsx に向ける（render.ts）。
//
// アプリの LessonVideo と同じ文脈（合図・レッスン・code_blocks）をシーンに与える。音声は鳴らさない。
// テーマは既定（ライト）のまま、章カラーは章の番号から決める。
import { useCallback, useLayoutEffect, useMemo, useRef } from "react";
import {
  AbsoluteFill,
  Composition,
  registerRoot,
  useCurrentFrame,
  type CalculateMetadataFunction,
} from "remotion";
import scenes from "progrust-render-check-lesson";
import "../../design-system/progrust/tokens.css";
import "../../design-system/progrust-video/tokens.css";
import "../../design-system/progrust/components/bundle.css";
import "../../design-system/progrust-video/components/bundle.css";
import { chapterColor } from "../../src/chapterNumbers";
import type {
  Chapter,
  LessonDefinition,
  Script,
} from "../../src/commands/content";
import {
  CodeBlocksContext,
  CuesContext,
  LessonContext,
  LessonTitleCountContext,
} from "../../src/video/hooks";
import { lessonInfo } from "../../src/video/lesson";
import {
  computeTimeline,
  createCues,
  FPS,
  type CueReference,
  type Durations,
} from "../../src/video/timeline";

// render.ts の RenderProps と同じ形。
type RenderProps = {
  sceneId: string;
  script: Script;
  durations: Durations;
  cueReference: CueReference;
  lesson: LessonDefinition;
  chapters: Chapter[];
};

// 表示している LessonTitle の数を、描いたフレームとともにブラウザの console に出す印。render.ts が読む。
const TITLE_COUNT_PREFIX = "progrust-render-check:lesson-title";

const WIDTH = 1280;
const HEIGHT = 720;

function SceneFrame(props: RenderProps) {
  const { sceneId, script, durations, cueReference, lesson, chapters } = props;
  const frame = useCurrentFrame();
  const timeline = useMemo(
    () => computeTimeline(script, durations, cueReference),
    [script, durations, cueReference],
  );
  const cues = useMemo(
    () => createCues(timeline, sceneId),
    [timeline, sceneId],
  );
  const info = useMemo(() => lessonInfo(chapters, lesson), [chapters, lesson]);
  const codeBlocks = useMemo(() => script.code_blocks ?? [], [script]);

  // LessonTitle が知らせる数を足し合わせる。子の useLayoutEffect は親より先に走るので、
  // 下の useLayoutEffect は、そのフレームで表示している数を出す。
  const titles = useRef(0);
  const countTitle = useCallback((delta: 1 | -1) => {
    titles.current += delta;
  }, []);
  useLayoutEffect(() => {
    console.log(`${TITLE_COUNT_PREFIX} frame=${frame} count=${titles.current}`);
  });

  const Scene = scenes[sceneId];
  if (Scene === undefined) {
    throw new Error(`index.tsx に、シーン ${sceneId} の部品がない`);
  }
  return (
    <AbsoluteFill data-chapter={chapterColor(info.chapterNumber)}>
      <AbsoluteFill data-lesson={lesson.id}>
        <LessonContext.Provider value={info}>
          <CodeBlocksContext.Provider value={codeBlocks}>
            <CuesContext.Provider value={cues}>
              <LessonTitleCountContext.Provider value={countTitle}>
                <Scene />
              </LessonTitleCountContext.Provider>
            </CuesContext.Provider>
          </CodeBlocksContext.Provider>
        </LessonContext.Provider>
      </AbsoluteFill>
    </AbsoluteFill>
  );
}

// シーンの長さを、台本と音声の長さから決める。
const calculateMetadata: CalculateMetadataFunction<RenderProps> = ({
  props,
}) => {
  const timeline = computeTimeline(
    props.script,
    props.durations,
    props.cueReference,
  );
  const scene = timeline.scenes.find((s) => s.id === props.sceneId);
  if (scene === undefined) {
    throw new Error(`シーン ${props.sceneId} がタイムラインにない`);
  }
  return { durationInFrames: scene.durationInFrames };
};

// 描くときは、いつも inputProps で渡す。既定の値は、型のためだけに置く。
const NO_PROPS: RenderProps = {
  sceneId: "",
  script: { scenes: [], sentences: [] },
  durations: {},
  cueReference: {},
  lesson: {
    id: "",
    chapter: "",
    kind: "lesson",
    title: "",
    primary_elements: [],
    secondary_elements: [],
  },
  chapters: [],
};

function Root() {
  return (
    <Composition
      id="Scene"
      component={SceneFrame}
      width={WIDTH}
      height={HEIGHT}
      fps={FPS}
      durationInFrames={1}
      defaultProps={NO_PROPS}
      calculateMetadata={calculateMetadata}
    />
  );
}

registerRoot(Root);
