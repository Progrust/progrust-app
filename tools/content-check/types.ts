// 教材チェックツールの型（documents/spec/content-guidelines/check-tool.md）。

export type Severity = "error" | "warning";

// 検査で見つけた 1 件。レッスンとファイルが分かるようにする。
export interface Finding {
  severity: Severity;
  // チェック項目の名前（「カリキュラム」など）。
  check: string;
  // 問題のあるファイル。教材のディレクトリからの相対パスか、一覧のディレクトリのファイル名。
  file: string;
  // 問題のあるレッスンの ID。レッスンに関係しないときは付けない。
  lesson?: string;
  message: string;
}

export interface CheckOptions {
  // 検査する教材のディレクトリ（既定は `content/`）。
  contentDir: string;
  // 章とレッスンの一覧のディレクトリ（既定は `documents/spec/curriculum/chapters/`）。
  curriculumDir: string;
}

// 検査の項目。項目を足すときは、この形の関数を checks に並べる。
export interface Check {
  name: string;
  run: (options: CheckOptions) => Finding[];
}
