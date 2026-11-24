---
name: implement-task
description: 実装計画のタスクの Issue（または、ループが直すと決めた「AI起票」の Issue）を 1 件、仕様どおりに実装し、テストを書いて、機械のチェックを通す。実装のループが呼ぶほか、ユーザーが Issue の番号を指定して呼ぶ。
argument-hint: "<Issue の番号> [--loop] [fix]"
disable-model-invocation: true
allowed-tools: Bash(gh issue list *) Bash(gh issue view *) Bash(gh issue create *) Bash(git status*) Bash(git diff*) Bash(git log*) Bash(git show*) Bash(git rev-parse*) Bash(git clean -f -- *) Bash(python3 tools/check-documents.py*) Bash(python3 tools/design-system.py check*) Bash(npm run *) Bash(npm test*) Bash(cargo *)
---

# タスクの実装

Issue を 1 件、最後まで実装する。途中で確認を求めない。

引数: `$ARGUMENTS`

- 1 つ目は、Issue の番号。
- `--loop` があれば、実装のループから呼ばれている。ユーザーはいない。
- `fix` があれば、直しの回である。`fix` の後ろに、通らなかった理由（指摘）が続く。

## 決まり

決まりは `documents/conventions/implementation-task.md` にある。ここには進め方だけを書く。

- 始める前に、`documents/conventions/implementation-task.md` を全部読む。`AI起票` の Issue を直すときは、`documents/conventions/issues.md` の「起票する問題」と「直すとき」も読む。
- このスキルと運用ルールが食い違うときは、運用ルールに従い、食い違いを結果の報告に書く。
- Bash では、1 回に 1 つのコマンドを実行する。`;`・`&&`・`|`・`for` でつながず、`cd` もしない（作業ディレクトリは、はじめからリポジトリのルートである）。ループの外では、このスキルの `allowed-tools` の形に合わない Bash に、ユーザーの確認が要る。ループでは、禁止された操作（コミット、push、ブランチの切り替え、pull request の操作など）が、確認なしに拒否される。禁止された操作を、別の書き方で実行し直さない。コミットとブランチの切り替えは、別の書き方でもしない（ループが止まる）。

## 進め方

### 1. Issue を読む

```bash
gh issue view （番号）
gh issue view （番号） --comments
git status --short
```

- `--comments` を付けると、本文は出ず、コメントだけが出る（コメントが無ければ、何も出ない）。本文とコメントは、別々に読む。

- Issue が開いていて、`実装タスク` が付いていることを確かめる。当てはまらないときは、実装せず、そのことを報告して終わる（ループでは、判断待ちではなく、進められない理由として返す）。
- `git status --short` に変更が出たら、始める前からあった変更として覚えておき、自分の変更と分ける。ループでは、作業ツリーに変更が無い状態で呼ばれる。

### 2. 仕様と ADR を読む

- タスクの Issue: 「読む仕様」の文書、その機能の `acceptance.md`、関連する ADR、その機能の技術検証の結果。
- `AI起票` の Issue: 「場所」のファイルのいまの内容、仕分けのコメントの「決めたことと理由」、関連する ADR。
- 関連する ADR は、`documents/conventions/links.md`「仕様に関係する ADR の探し方」の手順で探す。

### 3. 実装する

- 「作るもの」を、仕様どおりに作る。「範囲の外」のものは作らない。
- 「受け入れ条件と確かめ方」の条件ごとに、確かめ方の区分を決め、自動テストで確かめられる条件には、テストを書く。
- 仕様を変える必要が出たときと、問題を見つけたときは、運用ルールの「仕様を変える必要が出たとき」「見つけた問題」のとおりにする。
- `documents/` を変えるときは、先に、当てはまる運用ルール（`documents/conventions/`）を読む。

### 4. 機械のチェックを通す

`documents/conventions/implementation-loop.md`「機械のチェック」のコマンドを実行し、すべて通るまで直す。

- チェックを通すために、テスト・lint の規則・仕様・受け入れ条件を弱めない（運用ルールの「テストと確かめ方」）。
- 自分の変更と関係のない理由で、もともと失敗しているチェックがあるときは、直そうとせず、そのことを結果の報告に書く。

### 5. 差分を見直す

```bash
git status --short
git diff
```

- タスクと関係のない変更、消し忘れた一時的なコード、保護するパス（`documents/conventions/implementation-loop.md`「保護するパス」）の変更が無いことを確かめる。
- 一時的なファイル（データを作るためのスクリプトなど）は、なるべく作らない。作ったときは、結果を報告する前に消す。git に入っていないファイルは `git clean -f -- （パス）` で、git に入っているファイルは `git rm （パス）` で消す。消せないファイルを、中身を空にして残さない。
- 受け入れ条件を 1 つずつ見て、テストがあるか、人が確かめることに入っているかを確かめる。

### 6. 結果を報告する

運用ルールの「結果の報告」の項目を返す。

- ループでは、求められた形（`status`・`pr_title`・`summary`・`fixed_problems`・`human_checks`・`blocked_reason`・`blocking_issues`）で返す。
  - 文で書く項目（`pr_title`・`summary`・`fixed_problems`・`human_checks`・`blocked_reason`）は、日本語で書く（運用ルールの「結果の報告」）。
  - 済んだときは `status` を `done` にする。
  - ユーザーの判断が無いと進められないときは、`status` を `blocked` にし、`blocked_reason` に理由を、`blocking_issues` に、判断を求めて起票した Issue の番号を入れる。自分の変更は、そのままにしてよい（ループが捨てる）。
  - `pr_title` は、いまのリポジトリのコミットの書き方に合わせる（`feat: …`、`fix: …`、`docs: …` で始め、変更の内容を日本語の 1 文で続ける）。
- コミット、push、ブランチの切り替えはしない。
- ユーザーのいる会話では、応答の最後に、同じ項目と、起票した Issue の番号とタイトルを、平易な言葉で書く。変更がコミットされていないことも書く。

## 直しの回（fix）

`fix` の後ろの指摘を読み、通らなかった理由を直す。

- 機械のチェックの失敗: 出力を読んで、原因を直す。チェックのほうを弱めない。
- レビューの指摘: 1 件ずつ、指摘が挙げた仕様・受け入れ条件・ADR を読み直して直す。指摘が誤っていると、仕様の文を示して言えるときだけ、直さずに、その理由を `summary` に書く。
- CI の失敗: `documents/spec/non-functional/testing.md`「CI」に従う。作者の Mac で通り、CI でだけ失敗するときは、アプリのコードを Linux に合わせて直さない。
  - 失敗がタスクの変更と関係しない（タスクで変えていないテストが、たまに失敗するなど）と、失敗の出力と差分から言えるときは、ファイルを変えずに起票し、そのことを `summary` に書く。ループは、ファイルが変わらなくても CI をやり直す。
- 直した後に、手順 4 と 5 をやり直し、手順 6 の形で結果を返す。`fixed_problems` と `human_checks` には、この回で新しく加わったものだけを書く。
