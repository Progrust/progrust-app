// レッスンごとの動画のコードを遅延読み込みする Vite のプラグイン
// （documents/spec/architecture/responsibilities.md、documents/spec/content-format/video/structure.md）。
//
// `virtual:progrust-lesson-videos` は、レッスン ID から動画のコードを読み込む関数 `loadLessonVideo` を渡す。
// 動画のコード（index.tsx と、そこから読み込むファイル、durations.json、cue-reference.json）は、
// レッスンごとの塊に分かれ、`loadLessonVideo` を呼んだときに読み込まれる。
// script.json と音声（voice/）は、フロントエンドのビルドに含めない。
import { existsSync, readdirSync, realpathSync } from "node:fs";
import { join, resolve } from "node:path";
import type { Plugin } from "vite";
import { scopeLessonCss } from "./lesson-css.ts";

export const LESSON_VIDEOS_MODULE = "virtual:progrust-lesson-videos";
const LESSON_VIDEO_MODULE_PREFIX = "virtual:progrust-lesson-video/";

// 動画の入口のファイル。
const VIDEO_ENTRY = "index.tsx";
// 入口と合わせて読み込む、音声の長さのファイル。キーは、レッスンの塊の入口が export する名前。
// 開発時の起動中にレッスンを足したときや、これらのファイルを足したときは、起動し直す。
const DATA_FILES = {
  durations: "durations.json",
  cueReference: "cue-reference.json",
};

export type LessonVideosOptions = {
  // レッスンのディレクトリ（`{lessonsDir}/{lesson_id}/video/`）。既定はルートの `content/lessons`。
  lessonsDir?: string;
};

function toPosix(path: string): string {
  return path.replaceAll("\\", "/");
}

// 動画のあるレッスンの ID（`{lessonsDir}/{lesson_id}/video/index.tsx` がある ID）。ディレクトリが無ければ空。
export function findLessonVideos(lessonsDir: string): string[] {
  if (!existsSync(lessonsDir)) {
    return [];
  }
  return readdirSync(lessonsDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .filter((id) => existsSync(join(lessonsDir, id, "video", VIDEO_ENTRY)))
    .sort();
}

// パスが `{lessonsDir}/{lesson_id}/video/` の下なら、レッスン ID と video/ の中のパスを返す。
function lessonVideoPath(
  lessonsDir: string,
  id: string,
): { lessonId: string; file: string } | null {
  const path = toPosix(id.split("?")[0]);
  const base = `${toPosix(lessonsDir)}/`;
  if (!path.startsWith(base)) {
    return null;
  }
  const [lessonId, video, ...rest] = path.slice(base.length).split("/");
  if (video !== "video" || rest.length === 0) {
    return null;
  }
  return { lessonId, file: rest.join("/") };
}

// CSS として変換される読み込みか（`?raw`・`?url` などは CSS として扱われない）。
function isCssModuleRequest(id: string): boolean {
  const [path, query = ""] = id.split("?");
  if (!path.endsWith(".css")) {
    return false;
  }
  const params = new URLSearchParams(query);
  return !["raw", "url", "worker", "sharedworker"].some((key) =>
    params.has(key),
  );
}

function listModule(lessonIds: string[]): string {
  const loaders = lessonIds
    .map(
      (id) =>
        `  ${JSON.stringify(id)}: () => import(${JSON.stringify(LESSON_VIDEO_MODULE_PREFIX + id)}),`,
    )
    .join("\n");
  return [
    `const loaders = {\n${loaders}\n};`,
    "export function loadLessonVideo(lessonId) {",
    "  if (!Object.hasOwn(loaders, lessonId)) {",
    "    return Promise.reject(new Error(`レッスン ${lessonId} の動画のコードが無い`));",
    "  }",
    "  return loaders[lessonId]();",
    "}",
    "",
  ].join("\n");
}

function lessonModule(lessonsDir: string, lessonId: string): string {
  const video = join(lessonsDir, lessonId, "video");
  const lines = [
    `import scenes from ${JSON.stringify(toPosix(join(video, VIDEO_ENTRY)))};`,
  ];
  // durations.json と cue-reference.json が無いレッスン（音声を作る前など）では、空のオブジェクトにする。
  for (const [name, file] of Object.entries(DATA_FILES)) {
    const path = join(video, file);
    lines.push(
      existsSync(path)
        ? `import ${name} from ${JSON.stringify(toPosix(path))};`
        : `const ${name} = {};`,
    );
  }
  lines.push("export { scenes, durations, cueReference };", "");
  return lines.join("\n");
}

export function lessonVideos(options: LessonVideosOptions = {}): Plugin[] {
  let lessonsDir = options.lessonsDir ?? "";
  const resolvedList = `\0${LESSON_VIDEOS_MODULE}`;
  const resolvedLessonPrefix = `\0${LESSON_VIDEO_MODULE_PREFIX}`;

  // script.json と音声を読み込もうとしたら、失敗させる。
  // Vite の組み込みの読み込み（`?url` など）より先に確かめるので、enforce: "pre" にする。
  const guard: Plugin = {
    name: "progrust:lesson-videos-guard",
    enforce: "pre",

    configResolved(config) {
      const dir = resolve(config.root, options.lessonsDir ?? "content/lessons");
      // Vite はモジュールの ID を実際のパスにするので、シンボリックリンクを解いて比べる。
      lessonsDir = existsSync(dir) ? realpathSync(dir) : dir;
    },

    load(id) {
      const path = lessonVideoPath(lessonsDir, id);
      if (
        path !== null &&
        (path.file === "script.json" || path.file.startsWith("voice/"))
      ) {
        this.error(
          `script.json と音声は、フロントエンドのビルドに含めない: ${path.lessonId}/video/${path.file}`,
        );
      }
      return null;
    },
  };

  const main: Plugin = {
    name: "progrust:lesson-videos",

    resolveId(id) {
      if (id === LESSON_VIDEOS_MODULE) {
        return resolvedList;
      }
      if (id.startsWith(LESSON_VIDEO_MODULE_PREFIX)) {
        return `\0${id}`;
      }
      return null;
    },

    load(id) {
      if (id === resolvedList) {
        return listModule(findLessonVideos(lessonsDir));
      }
      if (id.startsWith(resolvedLessonPrefix)) {
        const lessonId = id.slice(resolvedLessonPrefix.length);
        if (!findLessonVideos(lessonsDir).includes(lessonId)) {
          this.error(`レッスン ${lessonId} の動画のコードが無い`);
        }
        return lessonModule(lessonsDir, lessonId);
      }
      return null;
    },

    // vite:css が CSS を変換した後（`@import` を取り込んだ後）、JS にする前に、セレクタを閉じ込める。
    transform(code, id) {
      const path = lessonVideoPath(lessonsDir, id);
      if (path === null || !isCssModuleRequest(id)) {
        return null;
      }
      return { code: scopeLessonCss(code, path.lessonId), map: null };
    },
  };

  return [guard, main];
}
