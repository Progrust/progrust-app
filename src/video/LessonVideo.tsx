// 動画全体の部品。Remotion の Player に component として渡す
// （documents/spec/content-format/video/structure.md、timeline-and-cues.md）。
// テーマと章カラーは、祖先の要素の属性で渡すので、ここでは扱わない（ADR 0176）。
import { useMemo, type ComponentType } from "react";
import { Html5Audio, Sequence } from "remotion";
import type { Chapter, LessonDefinition, Script } from "../commands/content";
import { CodeBlocksContext, CuesContext, LessonContext } from "./hooks";
import { lessonInfo } from "./lesson";
import {
  computeTimeline,
  createCues,
  type CueReference,
  type Durations,
} from "./timeline";
import { voiceUrl } from "./voice";

// レッスンの index.tsx の default export。キーはシーンの ID。
export type SceneComponents = { [sceneId: string]: ComponentType };

// Player の inputProps。
export type LessonVideoProps = {
  script: Script;
  durations: Durations;
  cueReference: CueReference;
  scenes: SceneComponents;
  // get_lesson の voice_dir。
  voiceDir: string;
  // list_chapters の chapters と、get_lesson の lesson（useLesson() の材料）。
  chapters: Chapter[];
  lesson: LessonDefinition;
};

export function LessonVideo({
  script,
  durations,
  cueReference,
  scenes,
  voiceDir,
  chapters,
  lesson,
}: LessonVideoProps) {
  const timeline = useMemo(
    () => computeTimeline(script, durations, cueReference),
    [script, durations, cueReference],
  );
  const cuesByScene = useMemo(
    () =>
      new Map(timeline.scenes.map((s) => [s.id, createCues(timeline, s.id)])),
    [timeline],
  );
  const info = useMemo(() => lessonInfo(chapters, lesson), [chapters, lesson]);
  const codeBlocks = useMemo(() => script.code_blocks ?? [], [script]);

  return (
    <LessonContext.Provider value={info}>
      <CodeBlocksContext.Provider value={codeBlocks}>
        {timeline.scenes.map((scene) => {
          const Scene = scenes[scene.id];
          if (Scene === undefined) {
            throw new Error(`index.tsx に、シーン ${scene.id} の部品がない`);
          }
          return (
            <Sequence
              key={scene.id}
              name={scene.id}
              from={scene.from}
              durationInFrames={scene.durationInFrames}
            >
              <CuesContext.Provider value={cuesByScene.get(scene.id) ?? null}>
                <Scene />
              </CuesContext.Provider>
              {scene.sentences.map((sentence) => {
                const audioFrames = sentence.audioEnd - sentence.from;
                // 1 フレームに満たない音声は鳴らさない（Sequence の長さは 1 以上）。
                if (audioFrames <= 0) {
                  return null;
                }
                return (
                  <Sequence
                    key={sentence.id}
                    name={sentence.id}
                    from={sentence.from}
                    durationInFrames={audioFrames}
                    layout="none"
                  >
                    <Html5Audio src={voiceUrl(voiceDir, sentence.id)} />
                  </Sequence>
                );
              })}
            </Sequence>
          );
        })}
      </CodeBlocksContext.Provider>
    </LessonContext.Provider>
  );
}
