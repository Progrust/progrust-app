// 検査「描画」「表紙」（documents/spec/content-guidelines/check-tool.md）。
// 準備中でないレッスンの動画を、画面なしの Chrome で描画する（技術検証は
// documents/verification/issue-7-render-check-in-ci.md）。
//
// - レッスンごとに、入口（render-entry.tsx）を Remotion のバンドラーでバンドルする。レッスン専用の CSS の
//   変換は、アプリの Vite のプラグインと同じものを使う。
// - シーンごとに、最初のフレーム、各文の開始位置のフレーム、最後のフレームを描画する。音声は鳴らさない。
// - 描画で起きたエラー（例外と、ブラウザの console のエラー）を、レッスン・シーン・フレームとともに報告する。
// - 最初のシーンで表示している LessonTitle の数を数え、1 つでないと「表紙」のエラーにする。
//
// 台本・音声の長さ・lesson.json・index.tsx のどれかが無いか読めないレッスンは、描画しない
// （「構成」「定義」「台本」「音声」が報告する）。
import { mkdtempSync, realpathSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import type { HeadlessBrowser } from "@remotion/renderer";
import type {
  Chapter,
  LessonDefinition,
  Script,
} from "../../src/commands/content.ts";
import {
  computeTimeline,
  type CueReference,
  type Durations,
  type Timeline,
} from "../../src/video/timeline.ts";
import { isFile, readLessonJson, type LessonEntry } from "./lesson-files.ts";
import type { RenderCssLoaderOptions } from "./render-css-loader.ts";
import type { CheckOptions, Finding } from "./types.ts";
import {
  isRecord,
  readJsonFile,
  readVideoLessons,
  videoPath,
  videoRel,
} from "./video-files.ts";

export const RENDER_CHECK = "描画";
export const COVER_CHECK = "表紙";

const ROOT = fileURLToPath(new URL("../..", import.meta.url));
const ENTRY = fileURLToPath(new URL("./render-entry.tsx", import.meta.url));
const CSS_LOADER = fileURLToPath(
  new URL("./render-css-loader.ts", import.meta.url),
);
// 入口が読み込む、レッスンの video/index.tsx の名前（render-lesson.d.ts）。
const LESSON_MODULE = "progrust-render-check-lesson";
// 入口が console に出す、表示している LessonTitle の数の印（render-entry.tsx）。
const TITLE_COUNT =
  /^progrust-render-check:lesson-title frame=(\d+) count=(-?\d+)$/;

// render-entry.tsx の RenderProps と同じ形。
type RenderProps = {
  sceneId: string;
  script: Script;
  durations: Durations;
  cueReference: CueReference;
  lesson: LessonDefinition;
  chapters: Chapter[];
};

interface Target {
  entry: LessonEntry;
  props: Omit<RenderProps, "sceneId">;
  timeline: Timeline;
}

function error(
  check: string,
  entry: LessonEntry,
  file: string,
  message: string,
): Finding {
  return {
    severity: "error",
    check,
    file: videoRel(entry, file),
    lesson: entry.id,
    message,
  };
}

function firstLine(e: unknown): string {
  const text = e instanceof Error ? e.message : String(e);
  return text.trim().split("\n")[0];
}

// chapters.json を、useLesson() の材料の形（list_chapters の chapters）にする。
// lessonInfo は、章の ID・タイトルと、レッスンの ID の並びだけを使う。
function readChapters(contentDir: string): Chapter[] {
  const result = readJsonFile(join(contentDir, "chapters.json"));
  if (result.state !== "ok" || !Array.isArray(result.value)) return [];
  const counts = { c: 0, rust: 0 };
  return result.value.filter(isRecord).map((c) => {
    const language = c.language === "c" ? "c" : "rust";
    counts[language] += 1;
    return {
      id: String(c.id),
      title: String(c.title),
      description: String(c.description),
      language,
      number: counts[language],
      lessons: (Array.isArray(c.lessons) ? c.lessons : []).map((id) => ({
        id: String(id),
        kind: null,
        title: null,
        summary: null,
        estimated_minutes: null,
        in_preparation: false,
      })),
    };
  });
}

// 描画するレッスンと、描画の前に分かるエラー。
function readTargets(contentDir: string): {
  targets: Target[];
  findings: Finding[];
} {
  const chapters = readChapters(contentDir);
  const targets: Target[] = [];
  const findings: Finding[] = [];
  for (const entry of readVideoLessons(contentDir)) {
    const lesson = readLessonJson(entry);
    const script = readJsonFile(videoPath(entry, "script.json"));
    const durations = readJsonFile(videoPath(entry, "durations.json"));
    const cuePath = videoPath(entry, "cue-reference.json");
    const cues = isFile(cuePath)
      ? readJsonFile(cuePath)
      : ({ state: "ok", value: {} } as const);
    if (
      lesson.state !== "ok" ||
      script.state !== "ok" ||
      durations.state !== "ok" ||
      cues.state !== "ok" ||
      !isRecord(script.value) ||
      !isRecord(durations.value) ||
      !isRecord(cues.value) ||
      !isFile(videoPath(entry, "index.tsx"))
    ) {
      continue;
    }
    const props = {
      script: script.value as Script,
      durations: durations.value as Durations,
      cueReference: cues.value as CueReference,
      lesson: lesson.value as LessonDefinition,
      chapters,
    };
    try {
      const timeline = computeTimeline(
        props.script,
        props.durations,
        props.cueReference,
      );
      targets.push({ entry, props, timeline });
    } catch (e) {
      findings.push(
        error(
          RENDER_CHECK,
          entry,
          "script.json",
          `台本と音声の長さから、シーンと文の位置を決められないので、描画できない: ${firstLine(e)}`,
        ),
      );
    }
  }
  return { targets, findings };
}

// 描画するフレームと、その見出し（最初・文の開始・最後）。
function framesOf(scene: Timeline["scenes"][number]): Map<number, string[]> {
  const frames = new Map<number, string[]>();
  const add = (frame: number, label: string) => {
    frames.set(frame, [...(frames.get(frame) ?? []), label]);
  };
  add(0, "最初");
  for (const sentence of scene.sentences) {
    add(sentence.from, `文 ${sentence.id} の開始`);
  }
  add(scene.durationInFrames - 1, "最後");
  return new Map([...frames].sort(([a], [b]) => a - b));
}

// webpack の設定に、レッスンの入口の名前と、CSS の変換を足す。
// webpack の型を直接は読み込まないので、使う部分だけの形にする。
interface WebpackConfig {
  resolve?: {
    alias?: unknown;
    modules?: string[];
  };
  module?: { rules?: unknown[] };
}

function overrideWebpack<T extends WebpackConfig>(
  config: T,
  target: Target,
): T {
  const videoDir = realpathSync(videoPath(target.entry, ""));
  const loader = (options: RenderCssLoaderOptions) => ({
    loader: CSS_LOADER,
    options,
  });
  const alias = config.resolve?.alias;
  return {
    ...config,
    resolve: {
      ...config.resolve,
      alias: {
        ...(isRecord(alias) ? alias : {}),
        [`${LESSON_MODULE}$`]: join(videoDir, "index.tsx"),
      },
      // 教材のディレクトリがリポジトリの外にあっても、react・remotion をリポジトリから読む。
      modules: [
        ...(config.resolve?.modules ?? ["node_modules"]),
        join(ROOT, "node_modules"),
      ],
    },
    module: {
      ...config.module,
      rules: [
        {
          test: /\.css$/i,
          include: videoDir,
          enforce: "pre",
          use: [loader({ lessonId: target.entry.id })],
        },
        ...(config.module?.rules ?? []),
      ],
    },
  };
}

// フレームの一覧の表示。例: 「0（最初）、18（文 m2 の開始）」。
function frameList(frames: [number, string[]][]): string {
  return frames
    .map(([frame, labels]) => `${frame}（${labels.join("・")}）`)
    .join("、");
}

async function renderLesson(
  target: Target,
  browser: HeadlessBrowser,
): Promise<Finding[]> {
  const { bundle } = await import("@remotion/bundler");
  const { renderStill, selectComposition } = await import("@remotion/renderer");
  const { entry, props, timeline } = target;
  const findings: Finding[] = [];
  const work = mkdtempSync(join(tmpdir(), "progrust-render-check-"));
  try {
    let serveUrl: string;
    try {
      serveUrl = await bundle({
        entryPoint: ENTRY,
        outDir: join(work, "bundle"),
        rootDir: ROOT,
        publicDir: join(work, "public"),
        enableCaching: false,
        onProgress: () => {},
        webpackOverride: (config) => overrideWebpack(config, target),
      });
    } catch (e) {
      return [
        error(
          RENDER_CHECK,
          entry,
          "index.tsx",
          `動画のコードをバンドルできない: ${firstLine(e)}`,
        ),
      ];
    }

    const coverCounts: number[] = [];
    for (const [index, scene] of timeline.scenes.entries()) {
      const inputProps: RenderProps = { ...props, sceneId: scene.id };
      // 同じエラーは、フレームをまとめて 1 件にする。
      const failures = new Map<string, [number, string[]][]>();
      const fail = (message: string, frame: [number, string[]]) => {
        failures.set(message, [...(failures.get(message) ?? []), frame]);
      };
      let composition;
      try {
        composition = await selectComposition({
          serveUrl,
          id: "Scene",
          inputProps,
          puppeteerInstance: browser,
          logLevel: "error",
        });
      } catch (e) {
        findings.push(
          error(
            RENDER_CHECK,
            entry,
            "index.tsx",
            `シーン ${scene.id} を描画できない: ${firstLine(e)}`,
          ),
        );
        continue;
      }
      for (const frame of framesOf(scene)) {
        const logs: { type: string; text: string }[] = [];
        try {
          await renderStill({
            composition,
            serveUrl,
            frame: frame[0],
            output: null,
            inputProps,
            puppeteerInstance: browser,
            logLevel: "error",
            onBrowserLog: (log) =>
              logs.push({ type: log.type, text: log.text }),
          });
        } catch (e) {
          fail(firstLine(e), frame);
          continue;
        }
        for (const log of logs) {
          if (log.type === "error") {
            fail(`ブラウザの console のエラー: ${firstLine(log.text)}`, frame);
          }
        }
        if (index === 0) {
          const counts = logs.flatMap((log) => {
            const m = TITLE_COUNT.exec(log.text);
            return m !== null && Number(m[1]) === frame[0]
              ? [Number(m[2])]
              : [];
          });
          coverCounts.push(counts.at(-1) ?? 0);
        }
      }
      for (const [message, frames] of failures) {
        findings.push(
          error(
            RENDER_CHECK,
            entry,
            "index.tsx",
            `シーン ${scene.id} のフレーム ${frameList(frames)}で描画に失敗した: ${message}`,
          ),
        );
      }
    }

    // 最初のシーンの、描画できたフレームのうち、LessonTitle がいちばん多いときの数。
    if (coverCounts.length > 0) {
      const first = timeline.scenes[0].id;
      const most = Math.max(...coverCounts);
      if (most === 0) {
        findings.push(
          error(
            COVER_CHECK,
            entry,
            "index.tsx",
            `最初のシーン ${first} が LessonTitle を表示していない`,
          ),
        );
      } else if (most > 1) {
        findings.push(
          error(
            COVER_CHECK,
            entry,
            "index.tsx",
            `最初のシーン ${first} が LessonTitle を ${most} 個表示している（1 つにする）`,
          ),
        );
      }
    }
    return findings;
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

export async function checkRender({
  contentDir,
}: CheckOptions): Promise<Finding[]> {
  const { targets, findings } = readTargets(contentDir);
  if (targets.length === 0) return findings;

  const { ensureBrowser, openBrowser } = await import("@remotion/renderer");
  let browser: HeadlessBrowser;
  try {
    // 画面なしの Chrome（Chrome Headless Shell）が無ければ、ダウンロードする。
    await ensureBrowser({ logLevel: "error" });
    browser = await openBrowser("chrome", {
      chromeMode: "headless-shell",
      chromiumOptions: { gl: "swangle" },
      logLevel: "error",
    });
  } catch (e) {
    return [
      ...findings,
      ...targets.map(({ entry }) =>
        error(
          RENDER_CHECK,
          entry,
          "index.tsx",
          `画面なしの Chrome を用意できないので、描画できない: ${firstLine(e)}`,
        ),
      ),
    ];
  }
  try {
    for (const target of targets) {
      findings.push(...(await renderLesson(target, browser)));
    }
  } finally {
    await browser.close({ silent: true });
  }
  return findings;
}
