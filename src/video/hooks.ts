// シーンの部品が使う共通部品のフック（documents/spec/content-format/video/scenes.md「共通部品」）。
import { createContext, useContext } from "react";
import type { Script } from "../commands/content";
import type { LessonInfo } from "./lesson";
import type { Cues } from "./timeline";

export type CodeBlockData = NonNullable<Script["code_blocks"]>[number];

// 値は、動画全体の部品（LessonVideo）が入れる。
export const CuesContext = createContext<Cues | null>(null);
export const LessonContext = createContext<LessonInfo | null>(null);
export const CodeBlocksContext = createContext<readonly CodeBlockData[] | null>(
  null,
);

// 表示している LessonTitle の数を知らせる先。LessonTitle は、表示を始めたときに 1、やめたときに −1 を渡す。
// 値は、教材チェックツールの描画（検査「表紙」。tools/content-check/render-entry.tsx）だけが入れる。
// アプリでは null のままで、LessonTitle は何も知らせない。
export const LessonTitleCountContext = createContext<
  ((delta: 1 | -1) => void) | null
>(null);

// 合図の関数 at・endOf を返す。シーンの部品の中で呼ぶ。
export function useCues(): Cues {
  const cues = useContext(CuesContext);
  if (cues === null) {
    throw new Error("useCues() は、シーンの部品の中で呼ぶ");
  }
  return cues;
}

// アプリが指定した章とレッスン定義から決めた、章とレッスンの番号と名前を返す。
export function useLesson(): LessonInfo {
  const lesson = useContext(LessonContext);
  if (lesson === null) {
    throw new Error("useLesson() は、動画の中で呼ぶ");
  }
  return lesson;
}

// 台本の code_blocks を返す（CodeBlock が使う）。
export function useCodeBlocks(): readonly CodeBlockData[] {
  const blocks = useContext(CodeBlocksContext);
  if (blocks === null) {
    throw new Error("useCodeBlocks() は、動画の中で呼ぶ");
  }
  return blocks;
}
