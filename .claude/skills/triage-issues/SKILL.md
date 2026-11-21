---
name: triage-issues
description: 開いている「AI起票」かつ「未判断」の Issue をまとめて仕分け、ユーザーの判断が要るもの以外を処理する（直す・保留にする・閉じる）。
argument-hint: "[Issue の番号 …]"
disable-model-invocation: true
allowed-tools: Bash(gh issue list *) Bash(gh issue view *) Bash(gh issue comment *) Bash(gh issue edit *) Bash(gh issue close *) Bash(gh issue create *) Bash(git rev-parse --short HEAD) Bash(git status *) Bash(git diff *) Bash(python3 tools/check-documents.py*)
---

# 未判断の Issue の仕分け

ユーザーは、AI エージェントが起票した Issue を1件ずつ見て判断する負担を減らすために、このスキルを呼ぶ。ユーザーに尋ねるのは、ユーザーの判断が要る Issue だけにする。それ以外は、自分で判断して最後まで処理する。

引数: `$ARGUMENTS`

- 引数が空なら、開いている `AI起票` かつ `未判断` の Issue を全部対象にする。
- 引数に Issue の番号があれば、その Issue だけを対象にする。開いていない Issue と、`AI起票` と `未判断` の両方が付いていない Issue は、対象にせず、報告でそう伝える。

## 決まり

仕分けと処理の決まりは、`documents/conventions/issues.md` にある。ここには進め方だけを書く。

- 始める前に、`documents/conventions/issues.md` を全部読む。特に「未判断の Issue の仕分け」「保留の Issue」「対応しない Issue を閉じるとき」に従う。
- このスキルと運用ルールが食い違うときは、運用ルールに従い、食い違いを報告で伝える。

## 進め方

### 1. 対象を集める

```bash
gh issue list --state open --label "AI起票" --label "未判断" --limit 200 --json number,title,labels,createdAt
git rev-parse --short HEAD
git status --short
```

- 対象が 0 件なら、そう伝えて終わる。
- ここで集めた Issue だけを対象にする。この後に起票した Issue は、この回では仕分けない。
- `git status --short` に変更が出たら、仕分けの前からあった変更として覚えておき、報告で、仕分けでした変更と分けて書く。

### 2. 全部の Issue を調べる

処理を始める前に、対象の Issue を全部調べる。

- `gh issue view （番号） --comments` で、本文とコメントを読む。
- 「場所」にあるファイルの、いまの内容を読む。「確認したコミット」より後に直っていないか、問題の前提が無くなっていないかを確かめる。
- 関連する ADR を探して読む（`documents/conventions/links.md`「仕様に関係する ADR の探し方」）。
- 同じファイルや同じ原因の Issue は、まとめて考える。

### 3. 仕分けを決める

Issue ごとに、運用ルールの「仕分けの種類」のどれかに決める。「ユーザーに回す Issue」に当てはまるかどうかを、必ず確かめる。迷ったら、ユーザーに回す。

### 4. ユーザーの判断が要る Issue を、先にまとめて尋ねる

ユーザーがその場にいるうちに答えられるよう、処理を始める前に尋ねる。

- AskUserQuestion を使う。1回に4問までなので、多いときは何回かに分ける。
- 質問は、それだけを読んで分かるように書く。Issue の番号、何が問題か、何を決めてほしいかを、技術的な詳細を省いた平易な言葉で書く。
- 選択肢は、推奨する答えを先頭にする。選択肢の説明には、それを選ぶとどうなるかを書く。
- ユーザーが答えた Issue は、その答えのとおりに、次の手順で処理する。コメントには「ユーザーの判断」と書く。
- ユーザーが答えなかった Issue は、何もせず、`未判断` のままにする。

### 5. 処理する

番号の小さい Issue から処理する。どの Issue も、コメントを書いてから、ラベルを変え、閉じる。`gh` のコマンドは、他のコマンドとつなげずに、1つずつ実行する。

コメントは次のように書く。日付は今日の日付にする。内容は運用ルールの「コメントと報告」に従う。

```bash
gh issue comment （番号） --body-file - <<'EOF'
（1行目。下の表）

（続き）
EOF
```

| 仕分け | コメントの1行目 | 続きの見出し | コメントの後のコマンド |
| --- | --- | --- | --- |
| 解消している | `解消していたので閉じる（YYYY-MM-DD、Claude の判断）。` | `## 確かめたこと` | `gh issue edit （番号） --remove-label "未判断"`、`gh issue close （番号） --reason "completed"`（前提が無くなったときは `"not planned"`） |
| 直す | `直したので閉じる（YYYY-MM-DD、Claude の判断）。` | `## 決めたことと理由`、`## 直したファイル` | `gh issue edit （番号） --remove-label "未判断"`、`gh issue close （番号） --reason "completed"` |
| 対応するが、この仕分けでは直さない | `対応すると決めた。この仕分けでは直していない（YYYY-MM-DD、Claude の判断）。` | `## 対応すると決めた理由`、`## 直すのに要る変更` | `gh issue edit （番号） --remove-label "未判断"` |
| 保留 | `保留にした（YYYY-MM-DD、Claude の判断）。` | `## 保留の理由`、`## 対応するきっかけ` | `gh issue edit （番号） --remove-label "未判断" --add-label "保留"` |
| 対応しない | `対応しないと決めたので閉じる（YYYY-MM-DD、Claude の判断）。` | `## 閉じた理由`、`## また問題になる条件` | `gh issue edit （番号） --remove-label "未判断"`、`gh issue close （番号） --reason "not planned"` |

直すときは、次のようにする。

- 直す前に、`CLAUDE.md` の「必ず守ること」のとおり、関連する ADR を確かめる。理由のある仕様変更には、ADR を追加する。
- ADR に書く理由が、自分の技術的な判断でないと分かったら、直すのをやめて、ユーザーに尋ねる（手順 4）。
- 直し始めてから、大きな変更が要ると分かったら、その Issue のために自分がした編集だけを、編集し直して元に戻し、「対応するが、この仕分けでは直さない」にする。
- 作業ツリーには、ユーザーの未コミットの変更や、他の Issue のために直した変更がある。`git checkout`・`git restore`・`git stash`・`git reset`・`git clean` は使わない。自分の編集だけをきれいに戻せないときは、そこで止めて、どのファイルに途中の変更が残っているかを報告する。
- `documents/` を変えたら、`python3 tools/check-documents.py` を実行し、指摘をすべて直してから、コメントを書いて閉じる。
- コメントの「直したファイル」には、直したファイル、追加した ADR、確認したコミット（`git rev-parse --short HEAD` の値に、未コミットの変更を加えた作業ツリーであること）を書く。
- 直している途中で別の問題を見つけたら、運用ルールのとおり起票する。その Issue は、この回では仕分けない。
- コミットしない。

### 6. 報告する

応答の最後に、次のことを書く。ユーザーは技術的な詳細に詳しくないので、理由は平易な言葉で短く書く。

- 仕分けた Issue の表（番号、タイトル、仕分け、理由）。ユーザーが答えて決めた Issue には、そのことを書く。
- `未判断` のまま残した Issue と、残した理由。
- 直したファイルと追加した ADR の一覧。変更がコミットされていないこと。
- この回で新しく起票した Issue の番号とタイトル。
- 判断を変えたい Issue があれば、番号を伝えてもらえば処理をやり直せること。
