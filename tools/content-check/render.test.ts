// 教材チェックツールの検査「描画」「表紙」（documents/spec/content-guidelines/check-tool.md）のテスト。
// テスト用のレッスンは tools/fixtures/render-check/content/lessons/ にある。
// 描画には画面なしの Chrome を使う。無ければ、はじめにダウンロードする（数分かかることがある）。
import { spawnSync } from "node:child_process";
import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { checkRender } from "./render.ts";
import type { Finding } from "./types.ts";

const root = fileURLToPath(new URL("../..", import.meta.url));
const contentDir = join(root, "tools/fixtures/render-check/content");
// 描画の検査は、一覧を使わない。
const curriculumDir = join(root, "documents/spec/curriculum/chapters");

// 描画は時間がかかるので、テスト用のレッスンを 1 回だけ検査する。
const RENDER_TIMEOUT = 600_000;
let findings: Finding[] = [];

beforeAll(async () => {
  findings = await checkRender({ contentDir, curriculumDir });
}, RENDER_TIMEOUT);

function ofLesson(lesson: string, check?: string): Finding[] {
  return findings.filter(
    (f) => f.lesson === lesson && (check === undefined || f.check === check),
  );
}

describe("描画", () => {
  it("正しい動画は、レッスン専用の CSS を閉じ込め、書体の @import を読み込まずに、エラーなく描画できる", () => {
    expect(ofLesson("ok")).toEqual([]);
  });

  it("台本にない文の ID を合図で指すと、シーンとフレームとともにエラーになる", () => {
    const errors = ofLesson("broken", "描画").filter((f) =>
      f.message.startsWith("シーン unknown "),
    );
    expect(errors).toEqual([
      expect.objectContaining({
        severity: "error",
        file: "lessons/broken/video/index.tsx",
        message: expect.stringContaining("台本にない文 zz を指している"),
      }),
    ]);
    // 最初のフレーム、文の開始位置のフレーム、最後のフレームを描画する。
    // 既定の lead 0.9 秒は 27 フレーム、音声 1 秒は 30 フレーム、tail 0.7 秒は 21 フレーム（合計 78）。
    expect(errors[0].message).toContain(
      "フレーム 0（最初）、27（文 u1 の開始）、77（最後）",
    );
  });

  it("ほかのシーンの文の ID を合図で指すと、エラーになる", () => {
    const errors = ofLesson("broken", "描画").filter((f) =>
      f.message.startsWith("シーン other "),
    );
    expect(errors).toHaveLength(1);
    expect(errors[0].message).toContain(
      "ほかのシーン title の文 t1 を指している",
    );
  });

  it("範囲の一部だけが重なる scopes を指定すると、エラーになる", () => {
    const errors = ofLesson("broken", "描画").filter((f) =>
      f.message.startsWith("シーン scopes "),
    );
    expect(errors).toHaveLength(1);
    expect(errors[0].message).toContain(
      "scopes a と b の範囲が一部だけ重なっている",
    );
  });

  it("ブラウザの console に出たエラーも、エラーになる", () => {
    const errors = ofLesson("broken", "描画").filter((f) =>
      f.message.startsWith("シーン image "),
    );
    expect(errors).toHaveLength(1);
    expect(errors[0].message).toMatch(
      /^シーン image のフレーム 0（最初）、27（文 i1 の開始）、77（最後）で描画に失敗した: ブラウザの console のエラー: .*404/,
    );
  });

  it("エラーのないシーンは、報告しない", () => {
    expect(
      ofLesson("broken", "描画").filter((f) =>
        f.message.startsWith("シーン title "),
      ),
    ).toEqual([]);
    expect(ofLesson("broken", "表紙")).toEqual([]);
  });

  it("準備中のレッスンは、描画しない", () => {
    expect(ofLesson("prepared")).toEqual([]);
  });
});

describe("表紙", () => {
  it("最初のシーンに LessonTitle がないと、エラーになる", () => {
    expect(ofLesson("no-title")).toEqual([
      {
        severity: "error",
        check: "表紙",
        file: "lessons/no-title/video/index.tsx",
        lesson: "no-title",
        message: "最初のシーン title が LessonTitle を表示していない",
      },
    ]);
  });

  it("最初のシーンで LessonTitle を 2 つ表示すると、エラーになる", () => {
    expect(ofLesson("two-titles")).toEqual([
      expect.objectContaining({
        check: "表紙",
        message:
          "最初のシーン title が LessonTitle を 2 個表示している（1 つにする）",
      }),
    ]);
  });
});

describe("描画しないレッスン", () => {
  const dirs: string[] = [];
  afterAll(() => {
    for (const dir of dirs) rmSync(dir, { recursive: true, force: true });
  });

  it("台本・音声の長さ・index.tsx のどれかが無いレッスンは、描画しない（ほかの検査が報告する）", async () => {
    const dir = mkdtempSync(join(tmpdir(), "progrust-render-check-test-"));
    dirs.push(dir);
    cpSync(contentDir, dir, { recursive: true });
    rmSync(join(dir, "lessons/ok/video/durations.json"));
    rmSync(join(dir, "lessons/broken/video/script.json"));
    rmSync(join(dir, "lessons/no-title/video/index.tsx"));
    writeFileSync(join(dir, "lessons/two-titles/lesson.json"), "{");
    expect(await checkRender({ contentDir: dir, curriculumDir })).toEqual([]);
  });

  it("台本と音声の長さからシーンと文の位置を決められないレッスンは、描画せずにエラーにする", async () => {
    const dir = mkdtempSync(join(tmpdir(), "progrust-render-check-test-"));
    dirs.push(dir);
    cpSync(contentDir, dir, { recursive: true });
    // ok だけを残し、音声の長さから文 c1 を消す。
    for (const id of ["broken", "no-title", "two-titles"]) {
      rmSync(join(dir, "lessons", id, "video/index.tsx"));
    }
    const path = join(dir, "lessons/ok/video/durations.json");
    const durations = JSON.parse(readFileSync(path, "utf8"));
    delete durations.c1;
    writeFileSync(path, JSON.stringify(durations));
    expect(await checkRender({ contentDir: dir, curriculumDir })).toEqual([
      {
        severity: "error",
        check: "描画",
        file: "lessons/ok/video/script.json",
        lesson: "ok",
        message:
          "台本と音声の長さから、シーンと文の位置を決められないので、描画できない: 文 c1 の音声の長さが durations.json にない",
      },
    ]);
  });
});

describe("npm run content:check のコマンド", () => {
  it(
    "描画と表紙のエラーを表示し、0 以外で終わる",
    () => {
      const dir = mkdtempSync(join(tmpdir(), "progrust-render-check-cli-"));
      try {
        const lessons = ["broken", "no-title"];
        // レッスンは、src/video を相対パスで読み込むので、写さずにリンクを張る。
        mkdirSync(join(dir, "lessons"));
        for (const id of lessons) {
          symlinkSync(
            join(contentDir, "lessons", id),
            join(dir, "lessons", id),
          );
        }
        writeFileSync(
          join(dir, "chapters.json"),
          JSON.stringify([
            { id: "ch01", title: "章", description: "", lessons },
          ]),
        );
        const result = spawnSync(
          "node",
          [
            join(root, "tools/content-check/main.ts"),
            "--content",
            dir,
            "--curriculum",
            curriculumDir,
          ],
          { encoding: "utf8" },
        );
        expect(result.status).not.toBe(0);
        expect(result.stdout).toMatch(
          /^エラー \[描画\] lessons\/broken\/video\/index\.tsx \[broken\]: シーン unknown のフレーム .*台本にない文 zz を指している/m,
        );
        expect(result.stdout).toContain(
          "エラー [表紙] lessons/no-title/video/index.tsx [no-title]: 最初のシーン title が LessonTitle を表示していない",
        );
      } finally {
        rmSync(dir, { recursive: true, force: true });
      }
    },
    RENDER_TIMEOUT,
  );
});
