// 描画の入口（render-entry.tsx）が読み込む、レッスンの動画の入口（video/index.tsx）。
// バンドルのときに、render.ts がレッスンのファイルに向ける。
declare module "progrust-render-check-lesson" {
  const scenes: { [sceneId: string]: import("react").ComponentType };
  export default scenes;
}
