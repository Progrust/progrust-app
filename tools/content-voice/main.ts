// `npm run content:voice` の入口。使い方は USAGE。
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { createTools } from "./external.ts";
import { generateVoices, type VoiceOptions } from "./voice.ts";

const root = fileURLToPath(new URL("../..", import.meta.url));

export const USAGE = `音声を作るツール（documents/spec/content-format/video/voice.md）

使い方: npm run content:voice -- [指定]

台本（video/script.json）の文の音声を VOICEPEAK で 1 文ずつ作り、video/voice/{文の ID}.m4a と
video/durations.json・video/cue-reference.json を書く。作り直すのは、durations.json の say・voice が
台本の say・content/voice.json と違う文と、音声ファイルがない文だけ。台本にない文の音声と記録は消す。

指定:
  --lesson <レッスンの ID>  対象のレッスン。何度も書ける。書かなければ、動画のあるすべてのレッスン
  --all                     対象のレッスンのすべての文の音声を作り直す
  --cue <文の ID>           合図を合わせ直した文の cue-reference.json の値を、いまの音声の長さに
                            書き換える。何度も書ける。--lesson を 1 つだけ書いたときに使える
  --content <ディレクトリ>  教材のディレクトリ（書かなければ、リポジトリの content/）
  -h, --help                この使い方を表示する

例:
  npm run content:voice -- --lesson ch01-01
  npm run content:voice -- --lesson ch01-01 --all
  npm run content:voice -- --lesson ch01-01 --cue m5 --cue m6

作者の環境の VOICEPEAK（/Applications/voicepeak.app）と ffmpeg・ffprobe を使う。`;

export type ParsedArgs =
  | { kind: "help" }
  | { kind: "error"; message: string }
  | { kind: "run"; options: Omit<VoiceOptions, "log" | "error"> };

export function parseVoiceArgs(args: string[]): ParsedArgs {
  let values;
  try {
    ({ values } = parseArgs({
      args,
      options: {
        lesson: { type: "string", multiple: true },
        all: { type: "boolean" },
        cue: { type: "string", multiple: true },
        content: { type: "string" },
        help: { type: "boolean", short: "h" },
      },
      strict: true,
      allowPositionals: false,
    }));
  } catch (e) {
    return { kind: "error", message: (e as Error).message };
  }
  if (values.help) return { kind: "help" };
  return {
    kind: "run",
    options: {
      contentDir: values.content ?? `${root}content`,
      lessons: values.lesson ?? [],
      all: values.all ?? false,
      cues: values.cue ?? [],
    },
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const parsed = parseVoiceArgs(process.argv.slice(2));
  if (parsed.kind === "help") {
    console.log(USAGE);
  } else if (parsed.kind === "error") {
    console.error(`エラー: ${parsed.message}\n\n${USAGE}`);
    process.exitCode = 1;
  } else {
    process.exitCode = generateVoices(parsed.options, createTools()).exitCode;
  }
}
