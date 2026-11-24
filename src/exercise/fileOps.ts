// multi レイアウトのファイル操作の可否と名前の検証
// （documents/spec/lesson-flow/drafts.md「multi レイアウトのファイル操作」、
// documents/spec/exercise-runner/project-assembly.md「受け付けるファイルパス」）。
// 画面の組み立てから分けて、自動テストで確かめられるようにする。
import type { DraftFiles } from "../commands/progress";

// 削除・名前変更できない、必須のファイル。
export const MAIN_FILE = "src/main.rs";

// 名前が受け付けられないときの理由。受け付けられるときは null。
export function validatePath(path: string, files: DraftFiles): string | null {
  if (path.startsWith("/") || path.includes("\\")) {
    return "パスは src/ か tests/ から始まる相対パスにしてください。";
  }
  const segments = path.split("/");
  if (segments.some((segment) => segment === "" || segment === "..")) {
    return "パスに空の部分や .. は使えません。";
  }
  if (segments.some((segment) => segment === ".")) {
    return "パスに . は使えません。";
  }
  if (segments[0] !== "src" && segments[0] !== "tests") {
    return "パスは src/ か tests/ の下にしてください。";
  }
  const name = segments[segments.length - 1];
  if (segments.length < 2 || !name.endsWith(".rs") || name === ".rs") {
    return "ファイルの拡張子は .rs にしてください。";
  }
  const lower = path.toLowerCase();
  for (const existing of Object.keys(files)) {
    const other = existing.toLowerCase();
    // 大文字と小文字だけが違う名前は、同じファイルとして扱う（macOS のファイルシステムに合わせる）。
    if (other === lower) {
      return "同じ名前のファイルがあります。";
    }
    // 既にあるファイルを、フォルダとして使うことはできない（逆も同じ）。
    if (lower.startsWith(`${other}/`) || other.startsWith(`${lower}/`)) {
      return "同じ名前のファイルかフォルダがあります。";
    }
  }
  return null;
}

// 編集・削除・名前変更ができないか（readonly_files のファイル）。
export function isReadonly(path: string, readonlyFiles: string[]): boolean {
  return readonlyFiles.includes(path);
}

export function canDelete(path: string, readonlyFiles: string[]): boolean {
  return path !== MAIN_FILE && !isReadonly(path, readonlyFiles);
}

export function canRename(path: string, readonlyFiles: string[]): boolean {
  return canDelete(path, readonlyFiles);
}

// タブの並び。src/main.rs を先頭に、残りは追加した順。
export function tabOrder(files: DraftFiles): string[] {
  const rest = Object.keys(files).filter((path) => path !== MAIN_FILE);
  return MAIN_FILE in files ? [MAIN_FILE, ...rest] : rest;
}

export function addFile(files: DraftFiles, path: string): DraftFiles {
  return { ...files, [path]: "" };
}

export function deleteFile(files: DraftFiles, path: string): DraftFiles {
  return Object.fromEntries(
    Object.entries(files).filter(([existing]) => existing !== path),
  );
}

// 名前を変える。タブの位置は変えない。
export function renameFile(
  files: DraftFiles,
  from: string,
  to: string,
): DraftFiles {
  return Object.fromEntries(
    Object.entries(files).map(([path, content]) => [
      path === from ? to : path,
      content,
    ]),
  );
}
