---
type: Convention
title: "参照とリンクの書き方"
description: "他の文書への参照を Markdown リンクで書くこと、本文の相対パスと frontmatter の `/` 始まりのパス、コードブロックの中でのパスの書き方、仕様と ADR のリンクの向き、ファイルを移動・改名したときのリンクの直し方を定める。"
---

## リンクの書き方

- 他の文書への参照は、Markdown リンクで書く。
- 節番号での参照は書かない。節番号を表す記号（`§`）も使わない。
- 本文のリンクは、その文書のディレクトリからの相対パスで書く。

```markdown
質問文は試行ログの `chat` イベントとして記録する（[質疑応答の試行ログへの記録](attempt-log.md)）。
関連仕様: [教材チェックツール](../spec/content-guidelines/check-tool.md)
```

- リンク先の文書の中の小見出しを指すときは、リンクの後に「小見出し」を続ける（例: `[質疑応答で AI に渡す入力](ai-input.md)「画面に出すコードの渡し方」`）。同じ文書の中の小見出しを指すときは、リンクにせず「小見出し」だけを書く。
- frontmatter に書くパス（`superseded_by`）は、バンドルのルートからのパスを `/` から書く（例: `superseded_by: /decisions/0062-write-video-scenes-in-unrestricted-tsx.md`）。

## コードブロックの中のパス

- コードブロックの中（ディレクトリ図のコメントなど）にはリンクが書けない。バンドルのルートからのパスを、先頭に `/` を付けずに書く。
- 小見出しを指すときは、パスの後に「小見出し」を続ける。
- 検査は、この形のパスが実在することを確かめる（[ドキュメントの検査](checks.md)）。

```
├── settings.json        # 設定（spec/screens/settings-screen.md）
├── validate/            # 生成問題の検証用プロジェクト（spec/exercise-generation/output-and-validation.md「検証」）
```

## 仕様と ADR のリンクの向き

- 仕様と ADR のリンクは、ADR から仕様への片方向だけにする。仕様から ADR へのリンクを書かない。
- ある仕様に関係する ADR を探すときは、`documents/decisions/` をその仕様のパスで検索する。

## 移動と改名

- ファイルを移動・改名したら、同じ変更の中で、そのファイルへのリンクとパスをすべて直す。
- 移動・改名する前のパスに、スタブ（`status: deprecated` の concept）を残さない。
