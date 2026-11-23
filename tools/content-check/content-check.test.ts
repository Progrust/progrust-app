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

// 一覧のレッスンの主要素。ch01-01 と ch02-01 は、主要素が 2 つある。
// ch01-02 は動画のみのレッスン。
const VIDEO_ONLY = "ch01-02";
function primaryOf(id: string): string[] {
  if (id === VIDEO_ONLY || id.endsWith("-review")) return [];
  const base = `x.${id.replace("-", "_")}`;
  return id === "ch01-01" || id === "ch02-01" ? [base, `${base}_b`] : [base];
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

// 「カリキュラム」の検査の結果だけを返す（構成・定義は、別の describe で確かめる）。
function checkLessons(lessons: Record<string, unknown>): Finding[] {
  return runChecks(setup(chapters, elements, lessons)).filter(
    (f) => f.check === "カリキュラム",
  );
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

type Json = Record<string, any>; // eslint-disable-line @typescript-eslint/no-explicit-any

const ALL_LESSONS = chapters.flatMap((c) => c.lessons);

// 構成と定義の検査に通る lesson.json。
function fullLesson(id: string): Json {
  const chapter = id.slice(0, 4);
  const base = {
    id,
    chapter,
    title: `タイトル${id}`,
    summary: "説明",
    estimated_minutes: 10,
  };
  if (id === VIDEO_ONLY) {
    return {
      ...base,
      kind: "video_only",
      tags: ["動画"],
      primary_elements: [],
      secondary_elements: [],
    };
  }
  const exercise = {
    layout: "single",
    judge: { mode: "stdout", target: "src/main.rs" },
    dependencies: [],
    readonly_files: [],
    case_labels: {},
  };
  if (id.endsWith("-review")) {
    return {
      ...base,
      kind: "chapter_review",
      primary_elements: [`x.${chapter}_01`, `x.${chapter}_01_b`],
      secondary_elements: [],
      exercise,
    };
  }
  return {
    ...base,
    kind: "lesson",
    tags: ["タグ"],
    primary_elements: primaryOf(id),
    secondary_elements: [],
    exercise,
  };
}

// lesson.json に合わせて必要なファイルを作る。台本の文は t1 の 1 つ。
function lessonFiles(lesson: Json): Record<string, string> {
  const files: Record<string, string> = {};
  if (lesson.kind !== "chapter_review") {
    files["video/script.json"] = JSON.stringify({
      scenes: [{ id: "s", name: "表紙" }],
      sentences: [{ id: "t1", scene: "s", text: "文", say: "文" }],
    });
    files["video/index.tsx"] = "";
    files["video/durations.json"] = "{}";
    files["video/voice/t1.m4a"] = "";
  }
  if (lesson.kind !== "video_only") {
    files["exercise/problem.md"] = "";
    files["exercise/explanation.md"] = "";
    files["exercise/starter/src/main.rs"] = "";
    files["exercise/solution/src/main.rs"] = "";
    files[
      lesson.exercise?.judge?.mode === "test"
        ? "exercise/judge_tests.rs"
        : "exercise/cases.json"
    ] = "";
  }
  return files;
}

interface FullOptions {
  // レッスンごとの lesson.json を書き換える。
  lessons?: (lesson: Json, id: string) => Json | undefined;
  // レッスンごとに、ファイルを足す・消す（消すときは undefined）。
  files?: (files: Record<string, string | undefined>, id: string) => void;
  // 準備中にするレッスン（ディレクトリを作らない）。
  prepared?: string[];
  masters?: unknown;
}

function checkFull(options: FullOptions = {}): Finding[] {
  const { contentDir, curriculumDir } = setup(
    chapters,
    options.masters ?? elements,
  );
  for (const id of ALL_LESSONS) {
    if (options.prepared?.includes(id)) continue;
    const lesson = options.lessons
      ? options.lessons(fullLesson(id), id)
      : fullLesson(id);
    const dir = join(contentDir, "lessons", id);
    mkdirSync(dir, { recursive: true });
    if (lesson !== undefined) {
      writeFileSync(join(dir, "lesson.json"), JSON.stringify(lesson));
    }
    const files: Record<string, string | undefined> = lessonFiles(
      lesson ?? fullLesson(id),
    );
    options.files?.(files, id);
    for (const [path, content] of Object.entries(files)) {
      if (content === undefined) continue;
      mkdirSync(join(dir, path, ".."), { recursive: true });
      writeFileSync(join(dir, path), content);
    }
  }
  return runChecks({ contentDir, curriculumDir });
}

function only(findings: Finding[], check: string): Finding[] {
  return findings.filter((f) => f.check === check);
}

describe("構成", () => {
  it("必要なファイルが揃っていれば、エラーにならない", () => {
    expect(checkFull()).toEqual([]);
  });

  it("準備中のレッスン（ディレクトリが無いレッスン）は、エラーにならない", () => {
    expect(checkFull({ prepared: ["ch01-01", "ch01-review"] })).toEqual([]);
  });

  it("ディレクトリがあり lesson.json が無いレッスンは、エラーになる", () => {
    const findings = checkFull({
      lessons: (lesson, id) => (id === "ch01-01" ? undefined : lesson),
    });
    expect(findings).toEqual([
      {
        severity: "error",
        check: "構成",
        file: "lessons/ch01-01/lesson.json",
        lesson: "ch01-01",
        message: "lesson.json がない",
      },
    ]);
  });

  it.each([
    "video/script.json",
    "video/index.tsx",
    "video/durations.json",
    "video/voice/t1.m4a",
  ])("動画のファイル %s が無いと、エラーになる", (path) => {
    const findings = checkFull({
      files: (files, id) => {
        if (id === "ch01-01") files[path] = undefined;
      },
    });
    expect(findings).toEqual([
      expect.objectContaining({
        check: "構成",
        file: `lessons/ch01-01/${path}`,
        lesson: "ch01-01",
      }),
    ]);
  });

  it("章末復習レッスンは、video/ が無くてもエラーにならない", () => {
    expect(only(checkFull(), "構成")).toEqual([]);
  });

  it.each([
    "exercise/problem.md",
    "exercise/explanation.md",
    "exercise/starter/src/main.rs",
    "exercise/solution/src/main.rs",
    "exercise/cases.json",
  ])("演習のファイル %s が無いと、エラーになる", (path) => {
    const findings = checkFull({
      files: (files, id) => {
        if (id === "ch02-01") files[path] = undefined;
      },
    });
    expect(findings).toEqual([
      expect.objectContaining({
        check: "構成",
        file: `lessons/ch02-01/${path}`,
      }),
    ]);
  });

  it("test 方式では、cases.json ではなく judge_tests.rs が必要になる", () => {
    const testMode = (lesson: Json, id: string) => {
      if (id === "ch01-01") {
        lesson.exercise.judge = { mode: "test", target: "src/main.rs" };
      }
      return lesson;
    };
    expect(checkFull({ lessons: testMode })).toEqual([]);
    const findings = checkFull({
      lessons: testMode,
      files: (files, id) => {
        if (id === "ch01-01") files["exercise/judge_tests.rs"] = undefined;
      },
    });
    expect(findings).toHaveLength(1);
    expect(findings[0].file).toBe("lessons/ch01-01/exercise/judge_tests.rs");
  });

  it("動画のみのレッスンに exercise/ があると、エラーになる", () => {
    const findings = checkFull({
      files: (files, id) => {
        if (id === VIDEO_ONLY) files["exercise/problem.md"] = "";
      },
    });
    expect(findings).toEqual([
      expect.objectContaining({
        check: "構成",
        file: `lessons/${VIDEO_ONLY}/exercise`,
      }),
    ]);
  });
});

describe("定義", () => {
  function definition(change: (lesson: Json) => void, id = "ch01-01") {
    return only(
      checkFull({
        lessons: (lesson, lessonId) => {
          if (lessonId === id) change(lesson);
          return lesson;
        },
        // 演習のファイルは、書き換えた lesson.json に関係なく足りる状態にする。
        files: (files) => {
          files["exercise/cases.json"] ??= "";
          files["exercise/judge_tests.rs"] ??= "";
        },
      }),
      "定義",
    );
  }

  it("正しい lesson.json は、エラーにならない", () => {
    expect(definition(() => {})).toEqual([]);
  });

  it("準備中のレッスンは、検査しない", () => {
    expect(only(checkFull({ prepared: ["ch01-01"] }), "定義")).toEqual([]);
  });

  it("id と chapter が chapters.json と違うと、エラーになる", () => {
    const findings = definition((l) => {
      l.id = "ch09-01";
      l.chapter = "ch02";
    });
    expect(findings.map((f) => f.message)).toEqual([
      expect.stringContaining("id がレッスンの ID と違う"),
      expect.stringContaining("chapter が chapters.json と違う"),
    ]);
    expect(findings[0]).toMatchObject({
      file: "lessons/ch01-01/lesson.json",
      lesson: "ch01-01",
    });
  });

  it("lesson.json が JSON でないと、エラーになる", () => {
    const { contentDir, curriculumDir } = setup(chapters, elements, {
      "ch01-01": "{",
    });
    const findings = runChecks({ contentDir, curriculumDir }).filter(
      (f) => f.check === "定義",
    );
    expect(findings).toHaveLength(1);
    expect(findings[0].message).toContain("lesson.json を読めない");
  });

  it("kind が決まった値でないと、エラーになる", () => {
    const findings = definition((l) => (l.kind = "other"));
    expect(findings).toHaveLength(1);
    expect(findings[0].message).toContain("kind が");
  });

  it.each(["title", "summary"])("%s が空だと、エラーになる", (key) => {
    const findings = definition((l) => (l[key] = ""));
    expect(findings.map((f) => f.message)).toEqual([
      `${key} が空でない文字列でない`,
    ]);
  });

  it("estimated_minutes が正の数でないと、エラーになる", () => {
    expect(definition((l) => (l.estimated_minutes = 0))).toHaveLength(1);
    expect(definition((l) => (l.estimated_minutes = "10"))).toHaveLength(1);
  });

  it.each(["lesson", "video_only"])(
    "kind が %s で tags が無いと、エラーになる",
    (kind) => {
      const id = kind === "lesson" ? "ch01-01" : VIDEO_ONLY;
      const findings = definition((l) => delete l.tags, id);
      expect(findings.map((f) => f.message)).toEqual([
        "tags が 1 件以上の文字列の配列でない",
      ]);
    },
  );

  it("tags が空の配列・空の文字列・重複を持つと、エラーになる", () => {
    expect(definition((l) => (l.tags = []))).toHaveLength(1);
    expect(definition((l) => (l.tags = ["a", ""]))).toHaveLength(1);
    expect(definition((l) => (l.tags = ["a", "a"]))).toHaveLength(1);
  });

  it("章末復習レッスンが tags を持つと、エラーになる", () => {
    const findings = definition((l) => (l.tags = ["a"]), "ch01-review");
    expect(findings.map((f) => f.message)).toEqual([
      "kind が chapter_review の lesson.json は tags を持たない",
    ]);
  });

  it("video_only が exercise を持つと、エラーになる", () => {
    const findings = definition((l) => {
      l.exercise = fullLesson("ch01-01").exercise;
    }, VIDEO_ONLY);
    expect(findings.map((f) => f.message)).toEqual([
      "video_only の lesson.json は exercise を持たない",
    ]);
  });

  it("video_only の primary_elements・secondary_elements が空でないと、エラーになる", () => {
    const findings = definition((l) => {
      l.primary_elements = ["x.ch01_01"];
      l.secondary_elements = ["x.ch01_01"];
    }, VIDEO_ONLY);
    expect(findings.map((f) => f.message)).toEqual([
      expect.stringContaining("primary_elements の件数が video_only の範囲"),
      "video_only の secondary_elements は空の配列にする",
    ]);
  });

  it("lesson で exercise が無いと、エラーになる", () => {
    const findings = definition((l) => delete l.exercise);
    expect(findings.map((f) => f.message)).toEqual([
      "kind が lesson の lesson.json に exercise がない",
    ]);
  });

  it.each([
    ["lesson", "ch01-01", [], false],
    ["lesson", "ch01-01", ["x.ch01_01"], true],
    ["lesson", "ch01-01", ["x.ch01_01", "x.ch02_01", "x.ch02_01_b"], false],
    ["chapter_review", "ch01-review", ["x.ch01_01"], false],
    [
      "chapter_review",
      "ch01-review",
      ["x.ch01_01", "x.ch02_01", "x.ch02_01_b"],
      true,
    ],
    [
      "chapter_review",
      "ch01-review",
      [
        "x.ch01_01",
        "x.ch02_01",
        "x.ch02_01_b",
        "x.ch01_01",
        "x.ch02_01",
        "x.ch02_01_b",
      ],
      false,
    ],
  ])(
    "kind が %s の primary_elements の件数が範囲にあるか（%s、%j）→ %s",
    (_kind, id, primary, ok) => {
      const findings = definition((l) => (l.primary_elements = primary), id);
      const countErrors = findings.filter((f) =>
        f.message.includes("primary_elements の件数"),
      );
      expect(countErrors).toHaveLength(ok ? 0 : 1);
    },
  );

  it("要素 ID が要素マスタに無いと、エラーになる", () => {
    const findings = definition((l) => {
      l.primary_elements = ["x.ch01_01"];
      l.secondary_elements = ["x.unknown"];
    });
    expect(findings.map((f) => f.message)).toEqual([
      "secondary_elements の要素 x.unknown が要素マスタにない",
    ]);
  });

  it("exercise の値が形式に合わないと、エラーになる", () => {
    const bad: ((e: Json) => void)[] = [
      (e) => (e.layout = "other"),
      (e) => (e.judge = { mode: "other" }),
      (e) => (e.judge = { mode: "stdout", target: "src/lib.rs" }),
      (e) => (e.judge = { mode: "stdout", min_learner_tests: 1 }),
      (e) => (e.judge = { mode: "test" }),
      (e) => (e.judge = { mode: "test", target: "src/lib.rs" }),
      (e) =>
        (e.judge = {
          mode: "test",
          target: "src/main.rs",
          min_learner_tests: 0,
        }),
      (e) => (e.dependencies = "serde"),
      (e) => (e.readonly_files = ["src/lib.rs"]),
      (e) => (e.case_labels = { a: 1 }),
      (e) => (e.case_labels = { a: "名前" }),
    ];
    for (const change of bad) {
      expect(definition((l) => change(l.exercise))).toHaveLength(1);
    }
  });

  it("test 方式で layout が multi の exercise は、エラーにならない", () => {
    expect(
      definition((l) => {
        l.exercise = {
          layout: "multi",
          judge: { mode: "test", target: "src/lib.rs", min_learner_tests: 2 },
          dependencies: [],
          readonly_files: ["src/helper.rs"],
          case_labels: { empty_input: "空の入力" },
        };
      }),
    ).toEqual([]);
  });
});

describe("既習要素", () => {
  function learned(
    change: (lesson: Json) => void,
    id: string,
    prepared?: string[],
  ) {
    return only(
      checkFull({
        prepared,
        lessons: (lesson, lessonId) => {
          if (lessonId === id) change(lesson);
          return lesson;
        },
      }),
      "既習要素",
    );
  }

  it("副要素が既習の要素だけなら、エラーにならない", () => {
    expect(
      learned((l) => (l.secondary_elements = ["x.ch01_01"]), "ch02-01"),
    ).toEqual([]);
  });

  it("副要素が既習でない（後のレッスンで導入する）と、エラーになる", () => {
    const findings = learned(
      (l) => (l.secondary_elements = ["x.ch02_01"]),
      "ch01-01",
    );
    expect(findings).toHaveLength(1);
    expect(findings[0]).toMatchObject({
      severity: "error",
      check: "既習要素",
      lesson: "ch01-01",
      file: "lessons/ch01-01/lesson.json",
    });
    expect(findings[0].message).toContain("x.ch02_01");
  });

  it("そのレッスン自身の主要素を副要素にすると、エラーになる", () => {
    expect(
      learned((l) => (l.secondary_elements = ["x.ch01_01"]), "ch01-01"),
    ).toHaveLength(1);
  });

  it("一覧のどのレッスンの主要素でもない要素は、エラーになる", () => {
    expect(
      learned((l) => (l.secondary_elements = ["x.nowhere"]), "ch02-01"),
    ).toHaveLength(1);
  });

  it("準備中のレッスンが主要素とする要素も、学習順で前にあれば既習になる", () => {
    expect(
      learned((l) => (l.secondary_elements = ["x.ch01_01"]), "ch02-01", [
        "ch01-01",
      ]),
    ).toEqual([]);
  });

  it("準備中のレッスンが主要素とする要素でも、学習順で後ろにあれば既習でない", () => {
    expect(
      learned((l) => (l.secondary_elements = ["x.ch02_01"]), "ch01-01", [
        "ch02-01",
      ]),
    ).toHaveLength(1);
  });

  it("準備中のレッスンは、検査しない", () => {
    expect(
      only(
        checkFull({
          prepared: ["ch01-01"],
          lessons: (lesson, id) => {
            if (id === "ch01-02") lesson.secondary_elements = ["x.ch02_01"];
            return lesson;
          },
        }),
        "既習要素",
      ).map((f) => f.lesson),
    ).toEqual(["ch01-02"]);
  });
});

describe("要素マスタ", () => {
  const orphan = { id: "x.orphan", chapter: "ch01" };

  it("すべての要素がどれかのレッスンの主要素なら、エラーにならない", () => {
    expect(only(checkFull(), "要素マスタ")).toEqual([]);
  });

  it("どのレッスンの primary_elements にも含まれない要素があると、エラーになる", () => {
    const findings = only(
      checkFull({ masters: [...elements, orphan] }),
      "要素マスタ",
    );
    expect(findings).toHaveLength(1);
    expect(findings[0]).toMatchObject({
      severity: "error",
      file: "elements.json",
    });
    expect(findings[0].message).toContain("x.orphan");
  });

  it("deprecated の要素は、エラーにならない", () => {
    expect(
      only(
        checkFull({ masters: [...elements, { ...orphan, deprecated: true }] }),
        "要素マスタ",
      ),
    ).toEqual([]);
  });

  it("準備中のレッスンが 1 つでもあるときは、検査しない", () => {
    expect(
      only(
        checkFull({ masters: [...elements, orphan], prepared: ["ch02-01"] }),
        "要素マスタ",
      ),
    ).toEqual([]);
  });
});

describe("章末復習の主要素", () => {
  function review(primary: string[]) {
    return checkFull({
      lessons: (lesson, id) => {
        if (id === "ch01-review") lesson.primary_elements = primary;
        return lesson;
      },
    });
  }

  it("その章のレッスンの主要素なら、エラーにならない", () => {
    expect(review(["x.ch01_01", "x.ch01_01_b"])).toEqual([]);
  });

  it("ほかの章のレッスンの主要素を含むと、エラーになる", () => {
    const findings = only(
      review(["x.ch01_01", "x.ch02_01"]),
      "章末復習の主要素",
    );
    expect(findings).toHaveLength(1);
    expect(findings[0]).toMatchObject({
      severity: "error",
      lesson: "ch01-review",
      file: "lessons/ch01-review/lesson.json",
    });
    expect(findings[0].message).toContain("x.ch02_01");
  });

  it("primary_elements が 1 件だと、エラーになる", () => {
    const findings = review(["x.ch01_01"]);
    expect(findings).toHaveLength(1);
    expect(findings[0]).toMatchObject({ severity: "error", check: "定義" });
  });

  it("準備中の章末復習レッスンは、検査しない", () => {
    expect(
      only(checkFull({ prepared: ["ch01-review"] }), "章末復習の主要素"),
    ).toEqual([]);
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
