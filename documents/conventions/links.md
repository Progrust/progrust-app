---
type: Convention
title: "参照とリンクの書き方"
description: "他の文書への参照を Markdown リンクで書くこと、本文の相対パスと frontmatter の `/` 始まりのパス、コードブロックの中でのパスの書き方、仕様と ADR のリンクの向き、仕様に関係する ADR の探し方、ファイルを移動・改名したときのリンクの直し方を定める。"
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
- ADR から仕様へのリンクは、ADR の本文の「関連仕様」の行に書く。載せる仕様の決め方は [ADR の運用ルール](decision-records.md)「本文」に従う。
- 技術検証の結果から仕様・ADR へはリンクしてよい。仕様から技術検証の結果へのリンクは書かない。ADR からは、判断の根拠として技術検証の結果へリンクしてよい（[技術検証の結果の書き方](verification-reports.md)）。

## 仕様に関係する ADR の探し方

ある仕様に関係する ADR は、次の順に探す。

1. `documents/decisions/` の ADR の「関連仕様」の行を、その仕様が属する機能のディレクトリのパス（例: `spec/qa-chat/`）で検索する。運用ルールに関係する ADR は、`conventions/` のファイルのパスで検索する。

   ```bash
   grep -l '^- 関連仕様:.*spec/qa-chat/' documents/decisions/*.md
   ```

2. その機能が他の機能の決まりに従うと定めているとき（その機能の `scope.md` に参照先がある）は、参照先の機能のパスでも同じように検索する。変える concept が他の機能の concept をリンクで参照しているときは、その concept のパスでも検索する。
   - 例: 質疑応答は `claude -p` の呼び出し方を [Claude Code 連携](../spec/claude-integration/index.md) に従うと定めているので、`spec/claude-integration/` でも検索する。
3. 見つかった ADR の決定内容を `decisions/index.md` の説明で確かめ、関係する ADR を開く。

- 変える範囲が、機能のディレクトリより狭いとき（1つの concept だけを変えるときや、字幕のように機能の一部だけを変えるとき）は、ディレクトリのパスの代わりに、その内容を定めている concept のパス（例: `spec/screens/video-screen.md`）で検索する。あわせて、その concept があるディレクトリの `index.md` のパス（例: `spec/screens/index.md`）でも検索する。ディレクトリの仕様の全体を対象にする ADR は、`index.md` にリンクしているからである。どの concept が定めているか分からないときは、先に `documents/spec/` をその語で検索して concept を見つける。
- `decisions/index.md` の見出しは、ADR を「関連仕様」の先頭の仕様で分けたものである。ある仕様の見出しの下にあるのは、その仕様に関係する ADR の一部だけなので、見出しだけを見て済ませない。
- ADR の本文（「背景」など）で触れているだけの仕様は「関連仕様」に載らない。ある語に触れた ADR を広く探すときは、`documents/decisions/` をその語で検索する。

## 移動と改名

- ファイルを移動・改名したら、同じ変更の中で、そのファイルへのリンクとパスをすべて直す。
- 移動・改名する前のパスに、スタブ（`status: deprecated` の concept）を残さない。
