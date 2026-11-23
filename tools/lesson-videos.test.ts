// レッスンごとの動画のコードの遅延読み込みと、レッスン専用の CSS の閉じ込め
// （documents/spec/architecture/responsibilities.md、documents/spec/content-format/video/structure.md）を確かめる。
//
// テスト用のレッスンは tools/fixtures/lesson-videos/content/lessons/ にある。
// - alpha: index.tsx・parts/Panel.tsx・alpha.css・durations.json・cue-reference.json・script.json・voice/
// - beta: alpha と同じ形で、cue-reference.json が無い
// - review: 動画が無い（lesson.json だけ）
// 各ファイルには目印の文字列（`{種類}-MARKER-{レッスン ID}`）を入れてあり、ビルドの出力にどれが入ったかを調べる。
// ディレクトリが無い・空のとき、禁じた読み込みをするときは、テストごとの一時ディレクトリで確かめる。
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import react from "@vitejs/plugin-react";
import { createElement, type ComponentType } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  build,
  createServer,
  resolveConfig,
  type InlineConfig,
  type Rolldown,
  type ViteDevServer,
} from "vite";
import { afterEach, describe, expect, it } from "vitest";
import { LESSON_VIDEOS_MODULE, lessonVideos } from "./lesson-videos.ts";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const fixtureRoot = fileURLToPath(
  new URL("./fixtures/lesson-videos/", import.meta.url),
);
const fixtureLessons = join(fixtureRoot, "content", "lessons");

const temps: string[] = [];
const servers: ViteDevServer[] = [];

afterEach(async () => {
  for (const server of servers.splice(0)) {
    await server.close();
  }
  for (const dir of temps.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

function write(path: string, content: string) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, content);
}

const marker = (kind: string, lessonId: string) => `${kind}-MARKER-${lessonId}`;

// 動画のコードとして読み込まれる目印。
const CODE_KINDS = ["LESSON", "PART", "DURATIONS", "CUE"];

// アプリの代わりの入口を置いた一時ディレクトリ。入口は、レッスン ID から動画のコードを読み込む関数だけを使う。
// アプリのビルドでは入口の export が消されるので、globalThis に置いて、関数を残す。
function makeRoot(): string {
  const root = mkdtempSync(join(tmpdir(), "progrust-lesson-videos-"));
  temps.push(root);
  write(
    join(root, "main.ts"),
    [
      `import { loadLessonVideo } from "${LESSON_VIDEOS_MODULE}";`,
      "Object.assign(globalThis, { load: (id: string) => loadLessonVideo(id) });",
      "",
    ].join("\n"),
  );
  return root;
}

function config(root: string, lessonsDir?: string): InlineConfig {
  return {
    configFile: false,
    root,
    logLevel: "silent",
    plugins: [
      lessonVideos(lessonsDir === undefined ? {} : { lessonsDir }),
      react(),
    ],
    build: {
      write: false,
      rolldownOptions: { input: join(root, "main.ts") },
    },
  };
}

type Output = (Rolldown.OutputChunk | Rolldown.OutputAsset)[];

async function buildOutput(lessonsDir?: string): Promise<Output> {
  const result = await build(config(makeRoot(), lessonsDir));
  return (Array.isArray(result) ? result : [result]).flatMap((item) =>
    "output" in item ? item.output : [],
  );
}

function chunks(output: Output): Rolldown.OutputChunk[] {
  return output.filter(
    (item): item is Rolldown.OutputChunk => item.type === "chunk",
  );
}

function source(item: Rolldown.OutputChunk | Rolldown.OutputAsset): string {
  return item.type === "chunk" ? item.code : String(item.source);
}

// 塊と、それと一緒に読み込まれる塊（静的な import）と CSS の中身。
function loadedWith(output: Output, start: Rolldown.OutputChunk): string {
  const byName = new Map(output.map((item) => [item.fileName, item]));
  const seen = new Set<string>();
  const texts: string[] = [];
  const visit = (fileName: string) => {
    const item = byName.get(fileName);
    if (seen.has(fileName) || item === undefined) {
      return;
    }
    seen.add(fileName);
    texts.push(source(item));
    if (item.type === "chunk") {
      item.imports.forEach(visit);
      item.viteMetadata?.importedCss.forEach(visit);
    }
  };
  visit(start.fileName);
  return texts.join("\n");
}

// 塊が直接読み込む CSS の中身。
function cssOf(output: Output, chunk: Rolldown.OutputChunk): string {
  return [...(chunk.viteMetadata?.importedCss ?? [])]
    .map((fileName) => output.find((item) => item.fileName === fileName))
    .map((item) => (item === undefined ? "" : source(item)))
    .join("\n");
}

function lessonModuleIds(output: Output): string[] {
  return chunks(output)
    .flatMap((c) => c.moduleIds)
    .filter((id) => id.startsWith("\0virtual:progrust-lesson-video/"));
}

function lessonChunk(output: Output, id: string): Rolldown.OutputChunk {
  const chunk = chunks(output).find((c) =>
    c.moduleIds.includes(`\0virtual:progrust-lesson-video/${id}`),
  );
  if (chunk === undefined) {
    throw new Error(`レッスン ${id} の塊が無い`);
  }
  return chunk;
}

function entryChunk(output: Output): Rolldown.OutputChunk {
  const chunk = chunks(output).find((c) => c.isEntry);
  if (chunk === undefined) {
    throw new Error("入口の塊が無い");
  }
  return chunk;
}

describe("ビルド", () => {
  it("レッスンごとに塊を分け、入口と一緒には、どのレッスンの動画のコードも読み込まない", async () => {
    const output = await buildOutput(fixtureLessons);

    const entry = entryChunk(output);
    const atStart = loadedWith(output, entry);
    for (const id of ["alpha", "beta"]) {
      for (const kind of CODE_KINDS) {
        expect(atStart).not.toContain(marker(kind, id));
      }
    }
    expect(entry.dynamicImports).toEqual(
      expect.arrayContaining([
        lessonChunk(output, "alpha").fileName,
        lessonChunk(output, "beta").fileName,
      ]),
    );
  }, 60_000);

  it("開いたレッスンの動画のコードだけが読み込まれ、ほかのレッスンのコードは読み込まれない", async () => {
    const output = await buildOutput(fixtureLessons);

    for (const [id, otherId] of [
      ["alpha", "beta"],
      ["beta", "alpha"],
    ]) {
      const loaded = loadedWith(output, lessonChunk(output, id));
      for (const kind of CODE_KINDS.filter(
        (k) => !(id === "beta" && k === "CUE"),
      )) {
        expect(loaded).toContain(marker(kind, id));
      }
      for (const kind of CODE_KINDS) {
        expect(loaded).not.toContain(marker(kind, otherId));
      }
    }
  }, 60_000);

  it("script.json と音声を、ビルドに含めない", async () => {
    const output = await buildOutput(fixtureLessons);

    const all = output.map(source).join("\n");
    expect(all).toContain(marker("LESSON", "alpha"));
    for (const id of ["alpha", "beta"]) {
      expect(all).not.toContain(marker("SCRIPT", id));
      expect(all).not.toContain(marker("VOICE", id));
    }
    expect(output.map((item) => item.fileName)).not.toContainEqual(
      expect.stringMatching(/\.m4a$|script/),
    );
  }, 60_000);

  it("レッスン専用の CSS は、そのレッスンの塊の CSS に、属性セレクタを付けて入る", async () => {
    const output = await buildOutput(fixtureLessons);

    // 圧縮で引用符が外れることがある（`[data-lesson=alpha]`）。
    const scope = (id: string) => `\\[data-lesson="?${id}"?\\]`;
    for (const id of ["alpha", "beta"]) {
      const css = cssOf(output, lessonChunk(output, id));
      expect(css).toMatch(new RegExp(`${scope(id)}\\s*\\.box\\s*\\{`));
      expect(css).toMatch(new RegExp(`${scope(id)}\\s*h5\\s*\\{`));
      expect(css).not.toContain(":root");
      // 属性セレクタの付いていないセレクタが無い。ほかのレッスンの属性セレクタも無い。
      for (const selector of css.match(/[^{}]+(?=\{)/g) ?? []) {
        expect(selector.trim()).toMatch(new RegExp(`^${scope(id)}`));
      }
    }
    expect(cssOf(output, lessonChunk(output, "alpha"))).toMatch(
      new RegExp(`${scope("alpha")}\\s*\\{\\s*--lesson-gap`),
    );
    expect(cssOf(output, entryChunk(output))).not.toContain("data-lesson");
  }, 60_000);

  it("動画の無いレッスン（video/index.tsx が無い）には、塊を作らない", async () => {
    const output = await buildOutput(fixtureLessons);
    expect(lessonModuleIds(output).sort()).toEqual([
      "\0virtual:progrust-lesson-video/alpha",
      "\0virtual:progrust-lesson-video/beta",
    ]);
  }, 60_000);

  it("動画のコードが script.json か音声を読み込むと、ビルドが失敗する", async () => {
    for (const file of ["./script.json", "./voice/t1.m4a?url"]) {
      const lessonsDir = join(makeRoot(), "lessons");
      const video = join(lessonsDir, "gamma", "video");
      write(join(video, "script.json"), "{}\n");
      write(join(video, "voice", "t1.m4a"), "voice\n");
      write(
        join(video, "index.tsx"),
        `import x from ${JSON.stringify(file)};\nexport default { title: () => String(x) };\n`,
      );
      await expect(buildOutput(lessonsDir), file).rejects.toThrow(
        "script.json と音声は、フロントエンドのビルドに含めない",
      );
    }
  }, 60_000);

  it("レッスンのディレクトリが無くても、空でも、ビルドが通る", async () => {
    // 既定の content/lessons が無い。
    const missing = await buildOutput();
    expect(lessonModuleIds(missing)).toEqual([]);

    const empty = join(makeRoot(), "lessons");
    mkdirSync(empty);
    const output = await buildOutput(empty);
    expect(lessonModuleIds(output)).toEqual([]);
  }, 60_000);
});

describe("開発時の起動", () => {
  async function startServer(
    root: string,
    lessonsDir?: string,
  ): Promise<ViteDevServer> {
    const server = await createServer({
      ...config(root, lessonsDir),
      server: { middlewareMode: true, hmr: false, ws: false },
    });
    servers.push(server);
    return server;
  }

  type LessonVideos = {
    loadLessonVideo: (id: string) => Promise<{
      scenes: Record<string, ComponentType>;
      durations: object;
      cueReference: object;
    }>;
  };

  async function lessonVideosModule(
    server: ViteDevServer,
  ): Promise<LessonVideos> {
    return (await server.ssrLoadModule(LESSON_VIDEOS_MODULE)) as LessonVideos;
  }

  it("レッスン ID から、シーンの部品と音声の長さを読み込む。cue-reference.json が無いレッスンでは空になる", async () => {
    // 既定の content/lessons を読む。
    const { loadLessonVideo } = await lessonVideosModule(
      await startServer(fixtureRoot),
    );

    const alpha = await loadLessonVideo("alpha");
    expect(Object.keys(alpha.scenes)).toEqual(["title"]);
    expect(renderToStaticMarkup(createElement(alpha.scenes.title))).toBe(
      `<div class="box" data-part="${marker("PART", "alpha")}">${marker("LESSON", "alpha")}</div>`,
    );
    expect(alpha.durations).toEqual({
      t1: expect.objectContaining({ say: marker("DURATIONS", "alpha") }),
    });
    expect(alpha.cueReference).toEqual({ [marker("CUE", "alpha")]: 1.5 });

    const beta = await loadLessonVideo("beta");
    expect(beta.durations).toEqual({
      t1: expect.objectContaining({ say: marker("DURATIONS", "beta") }),
    });
    expect(beta.cueReference).toEqual({});
  });

  it("動画のコードが無いレッスン ID では、失敗する", async () => {
    const { loadLessonVideo } = await lessonVideosModule(
      await startServer(fixtureRoot),
    );
    for (const id of ["review", "gamma", "toString"]) {
      await expect(loadLessonVideo(id)).rejects.toThrow(
        `レッスン ${id} の動画のコードが無い`,
      );
    }
  });

  it("レッスン専用の CSS に、属性セレクタを付ける", async () => {
    const server = await startServer(fixtureRoot);
    const result = await server.environments.client.transformRequest(
      "/content/lessons/alpha/video/alpha.css",
    );
    expect(result?.code).toContain('[data-lesson=\\"alpha\\"] .box');
    expect(result?.code).toContain('[data-lesson=\\"alpha\\"] h5');
    expect(result?.code).toContain('[data-lesson=\\"alpha\\"] {');
    expect(result?.code).not.toContain(":root");
  });

  it("レッスンのディレクトリが無くても、空でも、起動して、読み込む関数を渡せる", async () => {
    const missing = await lessonVideosModule(await startServer(makeRoot()));
    await expect(missing.loadLessonVideo("alpha")).rejects.toThrow(
      "レッスン alpha の動画のコードが無い",
    );

    const root = makeRoot();
    mkdirSync(join(root, "content", "lessons"), { recursive: true });
    const empty = await lessonVideosModule(await startServer(root));
    await expect(empty.loadLessonVideo("alpha")).rejects.toThrow(
      "レッスン alpha の動画のコードが無い",
    );
  });
});

describe("アプリの設定（vite.config.ts）", () => {
  it("プラグインを使う", async () => {
    const resolved = await resolveConfig(
      { configFile: join(repoRoot, "vite.config.ts"), root: repoRoot },
      "build",
    );
    expect(resolved.plugins.map((p) => p.name)).toEqual(
      expect.arrayContaining([
        "progrust:lesson-videos-guard",
        "progrust:lesson-videos",
      ]),
    );
  });

  it("開発時の起動で、読み込む関数の仮想モジュールを作れる", async () => {
    const server = await createServer({
      configFile: join(repoRoot, "vite.config.ts"),
      root: repoRoot,
      logLevel: "silent",
      server: { middlewareMode: true, hmr: false, ws: false },
    });
    servers.push(server);
    const result =
      await server.environments.client.transformRequest(LESSON_VIDEOS_MODULE);
    expect(result?.code).toContain("export function loadLessonVideo");
  });
});
