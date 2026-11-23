// 音声を作るツールが使う、作者の環境の VOICEPEAK・ffmpeg・ffprobe（documents/spec/content-format/video/voice.md）。
import { spawnSync, type SpawnSyncReturns } from "node:child_process";
import { existsSync, rmSync, statSync } from "node:fs";
import type { VoiceSettings, VoiceTools } from "./voice.ts";

export const VOICEPEAK_PATH =
  "/Applications/voicepeak.app/Contents/MacOS/voicepeak";

// VOICEPEAK は、まれに失敗したり、音声を書いたあと終わらなかったりする。そのときは作り直す。
// 1 回の実行は、ふつう数秒で終わる。
const VOICEPEAK_TIMEOUT_MS = 30_000;
const VOICEPEAK_ATTEMPTS = 3;

export function voicepeakArgs(
  say: string,
  voice: VoiceSettings,
  wavPath: string,
): string[] {
  const args = [
    "-s",
    say,
    "-n",
    voice.narrator,
    "--speed",
    String(voice.speed),
  ];
  const emotion = Object.entries(voice.emotion)
    .map(([name, value]) => `${name}=${value}`)
    .join(",");
  if (emotion !== "") args.push("-e", emotion);
  args.push("-o", wavPath);
  return args;
}

// 切り詰めたあとの音声を、ffmpeg に組み込みの AAC エンコーダーで `.m4a` に変換する（ADR 0127）。
export function ffmpegArgs(wavPath: string, m4aPath: string): string[] {
  return [
    "-hide_banner",
    "-loglevel",
    "error",
    "-y",
    "-i",
    wavPath,
    "-c:a",
    "aac",
    "-b:a",
    "64k",
    "-ac",
    "1",
    m4aPath,
  ];
}

// 長さは、ffprobe の format の duration で測る（ADR 0127）。
export function ffprobeArgs(m4aPath: string): string[] {
  return [
    "-v",
    "error",
    "-show_entries",
    "format=duration",
    "-of",
    "default=noprint_wrappers=1:nokey=1",
    m4aPath,
  ];
}

export function parseDuration(stdout: string): number {
  const seconds = Number(stdout.trim());
  if (!Number.isFinite(seconds) || seconds <= 0) {
    throw new Error(`ffprobe の出力を長さとして読めない（${stdout.trim()}）`);
  }
  return seconds;
}

function describe(name: string, result: SpawnSyncReturns<string>): string {
  if (result.error) {
    const code = (result.error as NodeJS.ErrnoException).code;
    if (code === "ENOENT") return `${name} が見つからない`;
    if (code === "ETIMEDOUT") return `${name} が時間内に終わらなかった`;
    return `${name} を実行できない（${result.error.message}）`;
  }
  const stderr = result.stderr.trim().split("\n").slice(-3).join(" / ");
  return `${name} が失敗した（終了コード ${result.status ?? result.signal}${stderr ? `、${stderr}` : ""}）`;
}

export interface ExternalPaths {
  voicepeak: string;
  ffmpeg: string;
  ffprobe: string;
}

export const DEFAULT_PATHS: ExternalPaths = {
  voicepeak: VOICEPEAK_PATH,
  ffmpeg: "ffmpeg",
  ffprobe: "ffprobe",
};

// `log` には、VOICEPEAK を作り直すときの知らせを渡す。
export function createTools(
  paths: ExternalPaths = DEFAULT_PATHS,
  log: (line: string) => void = console.log,
): VoiceTools {
  return {
    synthesize(say, voice, wavPath) {
      if (!existsSync(paths.voicepeak)) {
        throw new Error(`VOICEPEAK が見つからない（${paths.voicepeak}）`);
      }
      let last = "";
      for (let attempt = 1; attempt <= VOICEPEAK_ATTEMPTS; attempt++) {
        rmSync(wavPath, { force: true });
        const result = spawnSync(
          paths.voicepeak,
          voicepeakArgs(say, voice, wavPath),
          {
            encoding: "utf8",
            stdio: ["ignore", "pipe", "pipe"],
            timeout: VOICEPEAK_TIMEOUT_MS,
            killSignal: "SIGKILL",
          },
        );
        if (
          !result.error &&
          result.status === 0 &&
          existsSync(wavPath) &&
          statSync(wavPath).size > 44
        ) {
          return;
        }
        last =
          result.error || result.status !== 0
            ? describe("VOICEPEAK", result)
            : "VOICEPEAK が音声のファイルを書かなかった";
        if (attempt < VOICEPEAK_ATTEMPTS) log(`${last}。作り直す`);
      }
      rmSync(wavPath, { force: true });
      throw new Error(`${last}（${VOICEPEAK_ATTEMPTS} 回試した）`);
    },
    encode(wavPath, m4aPath) {
      const result = spawnSync(paths.ffmpeg, ffmpegArgs(wavPath, m4aPath), {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
      });
      if (result.error || result.status !== 0) {
        throw new Error(describe("ffmpeg", result));
      }
    },
    measureSeconds(m4aPath) {
      const result = spawnSync(paths.ffprobe, ffprobeArgs(m4aPath), {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
      });
      if (result.error || result.status !== 0) {
        throw new Error(describe("ffprobe", result));
      }
      return parseDuration(result.stdout);
    },
  };
}
