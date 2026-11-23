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
  // 「模範解答」「初期コード」の判定の作業領域。既定（省略したとき）は、判定だけを行うプログラムが決める
  // `src-tauri/target/content-check/`。テストで差し替える。
  judgeWorkspaceDir?: string;
}

// 検査の項目。項目を足すときは、この形の関数を checks に並べる。
export interface Check {
  name: string;
  run: (options: CheckOptions) => Finding[];
}
