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

// 一覧のレッスンの主要素。ch02-01 だけ、主要素が 2 つある。
// ch01-02 は動画のみのレッスン。
const VIDEO_ONLY = "ch01-02";
function primaryOf(id: string): string[] {
  if (id === VIDEO_ONLY || id.endsWith("-review")) return [];
  const base = `x.${id.replace("-", "_")}`;
  return id === "ch02-01" ? [base, `${base}_b`] : [base];
}

interface Element {
  id: string;
  chapter: string;
  deprecated?: boolean;
}

// 一覧と食い違わない要素マスタ。
const elements: Element[] = chapters.flatMap((c) =>
  c.lessons.flatMap((id) =>
    primaryOf(id).map((eid) => ({ id: eid, chapter: c.id })),
  ),
);

function chapterFile(chapter: Chapter): string {
  const rows = chapter.lessons.map((id) =>
    id.endsWith("-review")
      ? `| ${id} | 章末復習 | | |`
      : id === VIDEO_ONLY
        ? `| ${id} | タイトル${id} | 内容（動画のみのレッスン） | |`
        : `| ${id} | タイトル${id} | 内容 | ${primaryOf(id)
            .map((e) => `\`${e}\``)
            .join("、")} |`,
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

function setup(
  actual: unknown = chapters,
  masters: unknown = elements,
  lessons: Record<string, unknown> = {},
) {
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
  writeFileSync(join(contentDir, "elements.json"), JSON.stringify(masters));
  for (const [id, lesson] of Object.entries(lessons)) {
    mkdirSync(join(contentDir, "lessons", id), { recursive: true });
    writeFileSync(
      join(contentDir, "lessons", id, "lesson.json"),
      JSON.stringify(lesson),
    );
  }
  return { contentDir, curriculumDir };
}

function check(actual: unknown): Finding[] {
  return runChecks(setup(actual));
}

function lessonJson(id: string, change: Record<string, unknown> = {}) {
  return {
    id,
    kind: id === VIDEO_ONLY ? "video_only" : "lesson",
    title: `タイトル${id}`,
    primary_elements: primaryOf(id),
    ...change,
  };
}

function checkLessons(lessons: Record<string, unknown>): Finding[] {
  return runChecks(setup(chapters, elements, lessons));
}

function checkElements(change: (copy: Element[]) => void): Finding[] {
  const copy = structuredClone(elements);
  change(copy);
  return runChecks(setup(chapters, copy));
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

describe("lesson.json と一覧の照合", () => {
  it("一覧と一致すれば、エラーにならない", () => {
    expect(
      checkLessons({
        "ch01-01": lessonJson("ch01-01"),
        "ch01-02": lessonJson("ch01-02"),
        "ch02-01": lessonJson("ch02-01"),
      }),
    ).toEqual([]);
  });

  it("title が一覧と違うと、エラーになる", () => {
    const findings = checkLessons({
      "ch01-01": lessonJson("ch01-01", { title: "違う" }),
    });
    expect(findings).toHaveLength(1);
    expect(findings[0]).toMatchObject({
      severity: "error",
      check: "カリキュラム",
      file: "lessons/ch01-01/lesson.json",
      lesson: "ch01-01",
    });
    expect(findings[0].message).toContain("title が一覧と違う");
  });

  it("primary_elements が一覧の「主要素」と違うと、エラーになる", () => {
    const findings = checkLessons({
      "ch01-01": lessonJson("ch01-01", { primary_elements: ["x.other"] }),
    });
    expect(findings).toHaveLength(1);
    expect(findings[0].message).toContain(
      "primary_elements が一覧の「主要素」と違う",
    );
  });

  it("primary_elements の順序が違うと、エラーになる", () => {
    const findings = checkLessons({
      "ch02-01": lessonJson("ch02-01", {
        primary_elements: [...primaryOf("ch02-01")].reverse(),
      }),
    });
    expect(findings).toHaveLength(1);
  });

  it("一覧で動画のみのレッスンが video_only でないと、エラーになる", () => {
    const findings = checkLessons({
      "ch01-02": lessonJson("ch01-02", { kind: "lesson" }),
    });
    expect(findings).toHaveLength(1);
    expect(findings[0].message).toContain("video_only でない");
  });

  it("一覧で動画のみでないレッスンが video_only だと、エラーになる", () => {
    const findings = checkLessons({
      "ch01-01": lessonJson("ch01-01", {
        kind: "video_only",
        primary_elements: [],
      }),
    });
    expect(findings).toHaveLength(1);
    expect(findings[0].message).toContain("動画のみのレッスンでない");
  });

  it("章末復習レッスンは照合しない", () => {
    expect(
      checkLessons({
        "ch01-review": { kind: "chapter_review", title: "何でも" },
      }),
    ).toEqual([]);
  });

  it("準備中のレッスン（ディレクトリが無いレッスン）は照合しない", () => {
    expect(checkLessons({})).toEqual([]);
  });
});

describe("要素マスタと一覧の照合（要素と一覧）", () => {
  it("一覧と一致すれば、エラーにならない", () => {
    expect(checkElements(() => {})).toEqual([]);
  });

  it("一覧のどの「主要素」にも無い要素があると、準備中のレッスンがあってもエラーになる", () => {
    const findings = checkElements((e) =>
      e.push({ id: "x.orphan", chapter: "ch01" }),
    );
    expect(findings).toHaveLength(1);
    expect(findings[0]).toMatchObject({
      severity: "error",
      check: "要素と一覧",
    });
    expect(findings[0].message).toContain("x.orphan");
  });

  it("deprecated の要素は、一覧になくてもエラーにならない", () => {
    expect(
      checkElements((e) =>
        e.push({ id: "x.old", chapter: "ch01", deprecated: true }),
      ),
    ).toEqual([]);
  });

  it("一覧の「主要素」が要素マスタに無いと、エラーになる", () => {
    const findings = checkElements((e) => e.shift());
    expect(findings).toHaveLength(1);
    expect(findings[0].lesson).toBe("ch01-01");
    expect(findings[0].message).toContain("要素マスタにない");
  });

  it("一覧の「主要素」が deprecated だと、エラーになる", () => {
    const findings = checkElements((e) => (e[0].deprecated = true));
    expect(findings).toHaveLength(1);
    expect(findings[0].message).toContain("deprecated");
  });

  it("chapter が、最初に主要素とするレッスンの章と違うと、エラーになる", () => {
    const findings = checkElements((e) => (e[0].chapter = "ch02"));
    expect(findings).toHaveLength(1);
    expect(findings[0].message).toContain("chapter が一覧と違う");
  });

  it("要素 ID が重複すると、エラーになる", () => {
    const findings = checkElements((e) => e.push({ ...e[0] }));
    expect(findings.map((f) => f.message)).toEqual([
      `要素 ID ${elements[0].id} が重複している`,
    ]);
  });

  it("要素 ID の形式が違うと、エラーになる", () => {
    const findings = checkElements((e) => (e[0].id = "Bad-ID"));
    expect(findings.some((f) => f.message.includes("snake_case"))).toBe(true);
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
