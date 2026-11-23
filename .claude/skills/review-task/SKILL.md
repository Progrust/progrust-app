---
name: review-task
description: タスクの Issue（または、ループが直すと決めた「AI起票」の Issue）の実装を、仕様・受け入れ条件・ADR と差分だけを読んでレビューし、合格か不合格かと指摘を返す。実装のループが呼ぶほか、ユーザーが Issue の番号を指定して呼ぶ。
argument-hint: "<Issue の番号> [--loop]"
disable-model-invocation: true
allowed-tools: Bash(gh issue list *) Bash(gh issue view *) Bash(gh issue create *) Bash(git status*) Bash(git diff*) Bash(git log*) Bash(git show*) Bash(git merge-base*) Bash(git rev-parse*)
---

# 実装のレビュー

Issue の実装を、実装した側の説明に頼らずにレビューする。ファイルは変えない。

引数: `$ARGUMENTS`

- 1 つ目は、Issue の番号。
- `--loop` があれば、実装のループから呼ばれている。ユーザーはいない。

## 決まり

決まりは `documents/conventions/implementation-review.md` にある。ここには進め方だけを書く。

- 始める前に、`documents/conventions/implementation-review.md` を全部読む。
- このスキルと運用ルールが食い違うときは、運用ルールに従い、食い違いを結果に書く。
- Bash では、1 回に 1 つのコマンドを実行する。`;`・`&&`・`|`・`for` でつながず、`cd` もしない（作業ディレクトリは、はじめからリポジトリのルートである）。ループの外では、このスキルの `allowed-tools` の形に合わない Bash に、ユーザーの確認が要る。ループでは、禁止された操作（コミット、push、ブランチの切り替え、pull request の操作など）が、確認なしに拒否される。禁止された操作を、別の書き方で実行し直さない。コミットとブランチの切り替えは、別の書き方でもしない（ループが止まる）。

## 進め方

### 1. Issue と差分を読む

```bash
gh issue view （番号）
gh issue view （番号） --comments
git status --short
git diff origin/main...HEAD --stat
git diff origin/main...HEAD
```

- `--comments` を付けると、本文は出ず、コメントだけが出る（コメントが無ければ、何も出ない）。本文とコメントは、別々に読む。

- ループでは、実装はコミット済みで、`origin/main...HEAD` が差分になる。
- ユーザーのいる会話では、コミットされていない変更もレビューの対象にする（`git diff` と、`git status --short` に出た新しいファイル）。
- 差分が大きいときは、ファイルごとに `git diff origin/main...HEAD -- （パス）` で読む。変えたファイルは、差分だけでなく、前後の内容も読む。

### 2. 仕様と ADR を読む

- タスクの Issue: 「読む仕様」の文書、その機能の `acceptance.md`、関連する ADR（`documents/conventions/links.md`「仕様に関係する ADR の探し方」）。
- `AI起票` の Issue: 「場所」のファイルと、仕分けのコメントの「決めたことと理由」。
- 差分が `documents/` を変えているときは、当てはまる運用ルール（`documents/conventions/`）。

### 3. 観点ごとに確かめる

運用ルールの「観点」の表を、上から 1 つずつ確かめる。

- 受け入れ条件は、Issue の「受け入れ条件と確かめ方」の条件を 1 つずつ見て、それを確かめているテストを差分の中から探す。テストの中身を読み、条件を実際に確かめているかを見る。
- テストや lint の設定を変えた差分は、弱めていないかを、変える前の内容と比べて見る。
- 実装が正しいかは、仕様の文と照らして判断する。実装のコメントや、コミットの文は、根拠にしない。

### 4. 指摘の重さを決める

運用ルールの「指摘の重さと判定」に従う。

- `must` にするのは、仕様・受け入れ条件・ADR・運用ルールのどの文に反するかを言える指摘と、誤動作を具体的に言える指摘だけにする。
- 好みや書き方の違いは、`note` にするか、書かない。

### 5. 結果を返す

- ループでは、求められた形（`verdict`・`findings`・`summary`）で返す。
  - `verdict` は、`must` が 1 件でもあれば `fail`、無ければ `pass`。
  - `findings` の各項目は、`severity`（`must` か `note`）、`file`（パスと行）、`description`（何が誤っているかと、どの仕様・受け入れ条件・ADR・運用ルールに反するか）。
  - `summary` は、確かめたことと判定の理由を、短く書く。
- ユーザーのいる会話では、判定と指摘を、重さの順に、平易な言葉で書く。
- タスクと関係しない問題を見つけたときは、指摘にせず、`documents/conventions/issues.md` のとおりに起票し、起票した Issue の番号を `summary` に書く。
