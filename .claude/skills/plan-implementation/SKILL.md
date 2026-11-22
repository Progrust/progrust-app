---
name: plan-implementation
description: 実装計画を作る。最初はマイルストーンを最後のものまで決め、次に進めるマイルストーンの分だけ、未決事項を決めてから、タスクの Issue を GitHub に作る。マイルストーンの区切りでも呼び、次のマイルストーンのタスクを作る。
argument-hint: "[マイルストーンの番号]"
disable-model-invocation: true
allowed-tools: Bash(gh issue list *) Bash(gh issue view *) Bash(gh issue create *) Bash(gh issue edit *) Bash(gh issue comment *) Bash(gh api *milestones*) Bash(gh label list*) Bash(git rev-parse --short HEAD) Bash(git status *) Bash(git log *) Bash(python3 tools/check-documents.py*) Bash(python3 tools/design-system.py check*) Bash(python3 tools/implement-loop.py --status*) Bash(python3 tools/implement-loop.py --dry-run*)
---

# 実装計画を作る

ユーザーは、実装のループが人の確認を待たずに進められるように、このスキルを呼ぶ。計画の質が、ループの質を決める。タスクの Issue は、それだけを読んで、安いモデルでも実装を始められるように書く。

引数: `$ARGUMENTS`

- 引数が空なら、次に進めるマイルストーン（タスクの Issue がまだ無い、番号がいちばん小さいマイルストーン）を対象にする。マイルストーンが 1 つも無いときは、手順 2 でマイルストーンを決めるところから始める。
- 引数にマイルストーンの番号（`M2` の `2`）があれば、そのマイルストーンを対象にする。

## 決まり

決まりは `documents/conventions/implementation-plan.md` にある。ここには進め方だけを書く。

- 始める前に、`documents/conventions/implementation-plan.md` を全部読む。`documents/conventions/implementation-loop.md` の「タスクの選び方」「モデル」「保護するパス」も読む。
- このスキルと運用ルールが食い違うときは、運用ルールに従い、食い違いを報告で伝える。

## 進め方

### 1. いまの状態を集める

```bash
gh api "repos/{owner}/{repo}/milestones?state=all&per_page=100"
gh issue list --state all --label "実装タスク" --limit 500 --json number,title,state,labels,milestone
python3 tools/implement-loop.py --status
git log --oneline -20
```

- 済んだマイルストーンがあるときは、閉じたタスクの Issue と、行き詰まり・人の確認待ち・未判断の Issue を見て、計画に関わること（作り直しが要るもの、先のマイルストーンに回ったもの）を拾う。
- リポジトリにいま何があるか（`src/`・`src-tauri/`・`tools/` など）を確かめる。

### 2. マイルストーンを決める（最初だけ）

マイルストーンが 1 つも無いときに行う。

- `documents/spec/index.md` から、すべての機能の `scope.md` と `acceptance.md` を読む。
- 運用ルールの「マイルストーン」のとおりに、最後のものまで切る。最初は `M0 土台` にする。
- マイルストーンの一覧（タイトルと、済んだときに作者がアプリでできるようになること）を、表でユーザーに示し、AskUserQuestion で、この切り方でよいかを尋ねる。作者がどの順で何を触れるようになるかは、作者が決めることだからである。
- 決まったら、マイルストーンを作る。

```bash
gh api "repos/{owner}/{repo}/milestones" -f title="M0 土台" -f description="（済んだときに作者ができるようになること）"
```

### 3. 対象のマイルストーンの仕様を読む

- 対象のマイルストーンで扱う機能の、仕様の concept、`acceptance.md`、`open-questions.md`、関連する ADR、技術検証の結果を読む。
- 量が多いときは、機能ごとに読む。読んだ内容から、作るものを洗い出す。

### 4. 未決事項を決める

運用ルールの「未決事項を先に決める」に従う。

- 対象のマイルストーンの実装に要る未決事項を、すべて挙げる。
- AI エージェントが決められる事項は、決めて仕様に反映し、ADR を追加し、`open-questions.md` から消す。`CLAUDE.md` の「必ず守ること」に従う。
- ユーザーの判断が要る事項は、AskUserQuestion でまとめて尋ねる（1 回に 4 問まで。多いときは何回かに分ける）。質問は、技術的な詳細を省いた平易な言葉で書き、推奨する答えを先頭にする。答えを受けて、仕様に反映し、ADR を追加する。理由が分からないときは、理由も尋ねる。
- 仕様を読んでいて見つけた誤り・抜け・矛盾は、`documents/conventions/issues.md`「起票する問題」のとおりにする。「いましている作業」は、対象のマイルストーンの計画とする。
- `documents/` を変えたら、`python3 tools/check-documents.py` を実行し、指摘をすべて直す。

### 5. タスクに分ける

運用ルールの「タスクの切り方」「タスクの Issue」「難易度」に従う。

- 作るものを、前提の順に並べ、1 件ずつのタスクに分ける。
- タスクごとに、読む仕様、作るもの、受け入れ条件と確かめ方の案、人が確かめること、前提、範囲の外、難易度、`ループ対象外` にするかどうかを決める。
- 分け終えたら、次のことを確かめる。
  - 対象のマイルストーンで作る範囲に当たる受け入れ条件が、どれかのタスクに載っている。
  - どのタスクも、前提のタスクが済んでいれば、それだけでマージでき、マージした後に機械のチェックが通る。
  - 前提が輪になっていない。
  - 保護するパスを変えるタスクが、`ループ対象外` になっている。
  - 人が確かめることが、マイルストーンの説明（済んだときに作者ができるようになること）を確かめられるものになっている。

### 6. Issue を作る

- 作る前に、タスクの一覧（タイトル、難易度、前提、ループ対象外かどうか）を、表でユーザーに示す。確認は求めず、そのまま作る。
- 前提になるタスクから先に作り、前提の行には、作った Issue の番号を書く。

```bash
gh issue create --title "（タイトル）" --label "実装タスク,難易度:中" --milestone "M1 （名前）" --body-file - <<'EOF'
## 目的

…

## 読む仕様

- …

## 作るもの

- …

## 受け入れ条件と確かめ方

- … — 自動テスト

## 人が確かめること

- なし

## 前提

- 前提: なし

## 範囲の外

- なし
EOF
```

- `gh` のコマンドは、他のコマンドとつなげずに、1 つずつ実行する。
- 作り終えたら、`python3 tools/implement-loop.py --dry-run` を実行し、ループが最初に選ぶタスクが、意図したものであることを確かめる。

### 7. 報告する

応答の最後に、次のことを書く。ユーザーは技術的な詳細に詳しくないので、平易な言葉で短く書く。

- 対象のマイルストーンと、済んだときに作者ができるようになること。
- 作ったタスクの表（番号、タイトル、難易度、前提、ループ対象外かどうか）。
- 決めた未決事項と、追加した ADR。ユーザーが答えて決めたものには、そのことを書く。
- 残した未決事項と、残した理由。
- 起票せずに直した問題と、起票した Issue の番号とタイトル。
- 次にユーザーがすること（`ループ対象外` のタスクは、`/implement-task （番号）` で進めること。ループは `python3 tools/implement-loop.py` で始めること）。
- `documents/` の変更がコミットされていないこと。ループは origin の main から作業するので、ループを始める前に、コミットして push する必要があること。
