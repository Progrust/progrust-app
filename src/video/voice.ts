// 文の音声の URL（ADR 0128）。
import { convertFileSrc } from "@tauri-apps/api/core";

// get_lesson の voice_dir（音声のディレクトリの絶対パス）と文の ID から、
// アセットプロトコル（asset://）の URL を作る。
export function voiceUrl(voiceDir: string, sentenceId: string): string {
  return convertFileSrc(`${voiceDir}/${sentenceId}.m4a`);
}
