// 教材チェックツール（documents/spec/content-guidelines/check-tool.md）のテスト。
// エラーになる教材とならない教材を、一時ディレクトリに作って確かめる。
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { report, runChecks } from "./run.ts";
import type { Finding } from "./types.ts";

const root = fileURLToPath(new URL("../..", import.meta.url));
const main = join(root, "tools/content-check/main.ts");

interface Chapter {
  id: string;
  title: string;
  description: string;
  lessons: string[];
}

// 一覧（章ごとのファイルと index.md）と同じ内容の、正しい chapters.json。
const chapters: Chapter[] = [
  {
    id: "ch01",
    title: "Rust の基本",
    description: "最初の章の説明",
    lessons: ["ch01-01", "ch01-02", "ch01-review"],
  },
  {
    id: "ch02",
    title: "基本の型",
    description: "2 番目の章の説明",
    lessons: ["ch02-01", "ch02-review"],
  },
];

function chapterFile(chapter: Chapter): string {
  const rows = chapter.lessons.map((id) =>
    id.endsWith("-review")
      ? `| ${id} | 章末復習 | | |`
      : `| ${id} | タイトル | 内容 | \`x.${id}\` |`,
  );
  return [
    "---",
    "type: Specification",
    `title: "${chapter.id} ${chapter.title}"`,
    'description: "説明"',
    "---",
    "",
    chapter.description,
    "",
    "| ID | タイトル | 主に扱う内容 | 主要素 |",
    "| --- | --- | --- | --- |",
    ...rows,
    "",
  ].join("\n");
}

const dirs: string[] = [];

afterEach(() => {
  for (const dir of dirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

// 一覧は `chapters` のとおりに作り、content/chapters.json は `actual` にする。
// `MISSING` を渡すと、chapters.json を置かない。
const MISSING = Symbol("missing");

function setup(actual: unknown = chapters) {
  const dir = mkdtempSync(join(tmpdir(), "progrust-content-check-"));
  dirs.push(dir);
  const contentDir = join(dir, "content");
  const curriculumDir = join(dir, "chapters");
  mkdirSync(contentDir);
  mkdirSync(curriculumDir);
  writeFileSync(
    join(curriculumDir, "index.md"),
    [
      "# 章とレッスンの一覧",
      "",
      ...chapters.map((c) => `* [${c.id} ${c.title}](${c.id}-x.md) - 説明`),
      "",
    ].join("\n"),
  );
  for (const c of chapters) {
    writeFileSync(join(curriculumDir, `${c.id}-x.md`), chapterFile(c));
  }
  if (actual !== MISSING) {
    writeFileSync(
      join(contentDir, "chapters.json"),
      typeof actual === "string" ? actual : JSON.stringify(actual),
    );
  }
  return { contentDir, curriculumDir };
}

function check(actual: unknown): Finding[] {
  return runChecks(setup(actual));
}

function modified(change: (copy: Chapter[]) => void): Chapter[] {
  const copy = structuredClone(chapters);
  change(copy);
  return copy;
}

describe("chapters.json と一覧の照合", () => {
  it("一覧と一致すれば、エラーにならない", () => {
    expect(check(chapters)).toEqual([]);
  });

  it("章の並びが違うと、エラーになる", () => {
    const findings = check(modified((c) => c.reverse()));
    expect(findings).toHaveLength(1);
    expect(findings[0]).toMatchObject({
      severity: "error",
      check: "カリキュラム",
      file: "chapters.json",
    });
    expect(findings[0].message).toContain("章の並び");
  });

  it("一覧にある章が無いと、エラーになる", () => {
    const findings = check(modified((c) => c.pop()));
    expect(findings.map((f) => f.message)).toEqual([
      "一覧にある章 ch02 がない",
    ]);
  });

  it("一覧にない章があると、エラーになる", () => {
    const findings = check(modified((c) => c.push({ ...c[0], id: "ch99" })));
    expect(findings.map((f) => f.message)).toEqual([
      "一覧にない章 ch99 がある",
    ]);
  });

  it("章の ID が違うと、エラーになる", () => {
    const findings = check(modified((c) => (c[0].id = "ch09")));
    expect(findings.map((f) => f.message)).toEqual([
      "一覧にある章 ch01 がない",
      "一覧にない章 ch09 がある",
    ]);
  });

  it("章のタイトルが違うと、エラーになる", () => {
    const findings = check(modified((c) => (c[1].title = "違うタイトル")));
    expect(findings).toHaveLength(1);
    expect(findings[0].message).toContain("章 ch02 の title が一覧と違う");
  });

  it("章の説明が違うと、エラーになる", () => {
    const findings = check(modified((c) => (c[0].description = "違う説明")));
    expect(findings).toHaveLength(1);
    expect(findings[0].message).toContain(
      "章 ch01 の description が一覧と違う",
    );
  });

  it("レッスンの並びが違うと、エラーになる", () => {
    const findings = check(
      modified((c) => (c[0].lessons = ["ch01-02", "ch01-01", "ch01-review"])),
    );
    expect(findings).toHaveLength(1);
    expect(findings[0].message).toContain("章 ch01 のレッスンの並び");
  });

  it("一覧にあるレッスン（章末復習を含む）が無いと、レッスンを示してエラーになる", () => {
    const findings = check(modified((c) => c[0].lessons.pop()));
    expect(findings).toHaveLength(1);
    expect(findings[0]).toMatchObject({
      severity: "error",
      file: "chapters.json",
      lesson: "ch01-review",
    });
  });

  it("一覧にないレッスンがあると、レッスンを示してエラーになる", () => {
    const findings = check(modified((c) => c[1].lessons.push("ch02-09")));
    expect(findings).toHaveLength(1);
    expect(findings[0]).toMatchObject({ lesson: "ch02-09" });
  });

  it("準備中のレッスン（ディレクトリが無いレッスン）があっても、載っていればエラーにならない", () => {
    const { contentDir, curriculumDir } = setup(chapters);
    expect(runChecks({ contentDir, curriculumDir })).toEqual([]);
  });

  it("chapters.json が無いと、エラーになる", () => {
    const findings = check(MISSING);
    expect(findings).toHaveLength(1);
    expect(findings[0].file).toBe("chapters.json");
  });

  it("chapters.json が配列でないと、エラーになる", () => {
    expect(check({}).map((f) => f.message)).toEqual([
      "chapters.json が配列でない",
    ]);
  });
});

describe("結果の表示と終了コード", () => {
  const error: Finding = {
    severity: "error",
    check: "カリキュラム",
    file: "chapters.json",
    lesson: "ch01-01",
    message: "問題",
  };
  const warning: Finding = { ...error, severity: "warning", lesson: undefined };

  it("エラーと警告を、チェック項目・ファイル・レッスンが分かる形で表示する", () => {
    const { text } = report([warning, error]);
    expect(text).toContain(
      "エラー [カリキュラム] chapters.json [ch01-01]: 問題",
    );
    expect(text).toContain("警告 [カリキュラム] chapters.json: 問題");
    expect(text).toContain("エラー 1 件、警告 1 件");
  });

  it("エラーが 1 件でもあれば、終了コードが 0 以外になる", () => {
    expect(report([error]).exitCode).not.toBe(0);
    expect(report([warning, error]).exitCode).not.toBe(0);
  });

  it("警告だけなら、終了コードは 0 になる", () => {
    expect(report([warning]).exitCode).toBe(0);
    expect(report([]).exitCode).toBe(0);
  });
});

describe("npm run content:check のコマンド", () => {
  function run(args: string[]) {
    return spawnSync("node", [main, ...args], { encoding: "utf8" });
  }

  it("教材のディレクトリと一覧のディレクトリを差し替えられ、エラーが無ければ 0 で終わる", () => {
    const { contentDir, curriculumDir } = setup(chapters);
    const result = run([
      "--content",
      contentDir,
      "--curriculum",
      curriculumDir,
    ]);
    expect(result.status, result.stdout + result.stderr).toBe(0);
  });

  it("エラーがあると、0 以外で終わる", () => {
    const { contentDir, curriculumDir } = setup(
      modified((c) => (c[0].title = "違う")),
    );
    const result = run([
      "--content",
      contentDir,
      "--curriculum",
      curriculumDir,
    ]);
    expect(result.status).not.toBe(0);
    expect(result.stdout).toContain("エラー [カリキュラム] chapters.json");
  });

  it("いまのリポジトリの教材は、エラー 0 件で終わる", () => {
    const result = run([]);
    expect(result.status, result.stdout + result.stderr).toBe(0);
    expect(result.stdout).toContain("エラー 0 件");
  });
});
