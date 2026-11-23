// 章とレッスンの一覧（documents/spec/curriculum/chapters/）を読む。
// 読み方は documents/spec/curriculum/chapters/how-to-read.md に従う。
import { readFileSync } from "node:fs";
import { join } from "node:path";

export interface ListedLesson {
  id: string;
  title: string;
  // 「主に扱う内容」の末尾に「（動画のみのレッスン）」がある。
  videoOnly: boolean;
  // 「主要素」の列の要素 ID（書いた順）。
  primaryElements: string[];
}

export interface ListedChapter {
  id: string;
  title: string;
  description: string;
  lessons: string[];
  // 表の行ごとの内容（lessons と同じ順）。
  rows: ListedLesson[];
  // 章ごとのファイル名。
  file: string;
}

function frontmatterTitle(text: string): string | undefined {
  const match = /^---\n([\s\S]*?)\n---/.exec(text);
  const line = match?.[1].split("\n").find((l) => l.startsWith("title:"));
  if (line === undefined) return undefined;
  const value = line.slice("title:".length).trim();
  return value.startsWith('"') && value.endsWith('"')
    ? value.slice(1, -1)
    : value;
}

function body(text: string): string[] {
  const end = /^---\n[\s\S]*?\n---\n/.exec(text);
  return (end ? text.slice(end[0].length) : text).split("\n");
}

export function readChapterFile(dir: string, file: string): ListedChapter {
  const text = readFileSync(join(dir, file), "utf8");
  const title = frontmatterTitle(text);
  if (title === undefined) {
    throw new Error(`${file}: frontmatter に title がない`);
  }
  const space = title.indexOf(" ");
  if (space < 0) {
    throw new Error(`${file}: title が「章 ID + 章のタイトル」の形でない`);
  }
  const lines = body(text);
  const description = lines.find((l) => l.trim() !== "")?.trim();
  if (description === undefined) {
    throw new Error(`${file}: 本文に章の説明がない`);
  }
  const rows: ListedLesson[] = [];
  for (const line of lines) {
    if (!/^\|\s*ch\d+-[A-Za-z0-9]+\s*\|/.test(line)) continue;
    const cells = line.split(/(?<!\\)\|/).map((c) => c.trim());
    // 先頭と末尾の「|」の外側は空になる。
    const primary = cells[4] ?? "";
    rows.push({
      id: cells[1],
      title: cells[2] ?? "",
      videoOnly: (cells[3] ?? "").endsWith("（動画のみのレッスン）"),
      primaryElements: primary
        .split("、")
        .map((e) => e.replace(/`/g, "").trim())
        .filter((e) => e !== ""),
    });
  }
  const lessons = rows.map((r) => r.id);
  return {
    id: title.slice(0, space),
    title: title.slice(space + 1).trim(),
    description,
    lessons,
    rows,
    file,
  };
}

// index.md に並べた章の順に、章を返す。
export function readChapterList(dir: string): ListedChapter[] {
  const index = readFileSync(join(dir, "index.md"), "utf8");
  const files: string[] = [];
  for (const match of index.matchAll(/\]\((ch\d+-[^)\s]+\.md)\)/g)) {
    files.push(match[1]);
  }
  return files.map((file) => readChapterFile(dir, file));
}
