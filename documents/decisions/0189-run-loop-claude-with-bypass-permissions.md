---
type: Decision
title: "ADR 0189"
description: "実装のループが呼ぶ claude を、実装・レビュー・仕分けのすべてで権限のモード bypassPermissions で動かし、コミット・push・ブランチの切り替え・pull request の操作・保護するパスの書き換えなどだけを deny で禁止し、claude を呼んだ前後で作業ツリーの HEAD が変わったらループを止める"
---

- 日付: 2026-11-23
- 関連仕様: [実装のループ](../conventions/implementation-loop.md)「権限」、[実装のループ](../conventions/implementation-loop.md)「止まるとき」
- 置き換え: なし

## 背景

ループの `claude` の権限は、モード `dontAsk` と、許可する操作を書いたファイルで決めていた（[ADR 0161](0161-run-implementation-loop-locally-and-park-human-work.md)）。ADR 0161 は、`bypassPermissions` を使わない理由に、公式の資料がそのモードを隔離した環境だけで使うものとしていることを挙げた。

2026-11-23 にループを回したところ、記録に、権限で拒否された操作が、実装とレビューの会話で 1 回あたり 1〜3 件ずつ残っていた。拒否されたのは、`sed -i`、`cat > （ファイル） <<EOF`、`grep`、`find`、`node`、`python3 -`、`rm`、`cd` から始まるコマンドなどである。レビュー（タスク #88）でも、`grep` と `find` が拒否された。多くの場合、`claude` は許可された Edit・Write のツールでやり直し、タスクはマージまで進んだ。しかし、拒否のたびに遠回りをしていた。それまでの試運転では、権限の不足でループが止まったこともある（[ADR 0183](0183-stop-loop-when-blocked-by-permission-denials.md)、[ADR 0184](0184-allow-loop-to-delete-files-with-git-clean.md)）。

ユーザーは「別にこのPCであればぶっ壊れても構わないのでフル権限を与えてもいい」と述べた。Claude は、実装する側だけを `bypassPermissions` にし、レビューと仕分けは許可の一覧を広げる案を推奨した。ユーザーは、実装・レビュー・仕分けのすべてを `bypassPermissions` にする案を選び、その理由として「拒否での遠回りを無くしたい」を選んだ。

2026-11-23 に公式の資料（permission-modes、permissions、headless、sandboxing のページ）で読んだことは、次のとおり。資料は、要約を通して読んだ。

- deny のルールは、`bypassPermissions` を含むどのモードでも効く。allow のルールは、`bypassPermissions` では使われない。
- `bypassPermissions` でも、ask のルールと、重要なパスを消す `rm`・`rmdir` には、確認が要る。
- `bypassPermissions` では、`.git` や `.claude` のような保護されたパスへの書き込みにも、確認が要らない。
- Bash のルールは、コマンドの文字列で判定する。別の書き方のコマンド（例: `/usr/bin/git push`）は、ルールに当たらない。
- macOS では、root や `sudo` で `bypassPermissions` を使うと、起動しない。

同じ日に、ユーザーが、使い捨てのリポジトリで Claude Code 2.1.329 を呼んで確かめたことは、次のとおり。呼び方は、ループと同じく、`claude -p --permission-mode bypassPermissions --settings （リポジトリの外のファイル） --output-format json` である。`deny` には `Edit(.claude/**)` と `Bash(git commit*)` を書いた。

- `claude` は、確認を待たずに起動し、`deny` に当たらない Bash（ファイルを作る `echo`）を実行した。
- `git commit --allow-empty -m test1` は拒否された。`echo x > b.txt && git add b.txt && git commit -m test2` のように、つないだコマンドの中のコミットも拒否された。
- Write ツールでの `.claude/x.txt` への書き込みは拒否された。設定のファイルをリポジトリの外に置いても、`Edit(.claude/**)` は、作業ディレクトリの中のパスに当たった。
- 拒否された 3 件は、どれも結果の `permission_denials` に入った。

## 決定

- ループが呼ぶ `claude` を、実装・レビュー・仕分けのすべてで、権限のモード `bypassPermissions` で動かす。ユーザーが選んだ（「背景」を参照）。ADR 0161 の「`claude` の権限は、`dontAsk` と、許可する操作を書いたファイル（実装・レビュー・仕分けで別）で決め、`--settings` で渡す。」を、「`claude` の権限は、`bypassPermissions` と、禁止する操作を書いたファイル（実装・レビュー・仕分けで別）で決め、`--settings` で渡す。」に変える。ADR 0184 の「`rm` は許可しない。」も、成り立たなくなる。
- 次の細部は Claude が決めた。決まりは [実装のループ](../conventions/implementation-loop.md)「権限」に定める。
  - 3 つのファイルの `deny` に、保護するパスの書き換え、git のコミット・push・ブランチの切り替え・コミットを作る操作、pull request を作る・変える操作、`gh api`、リポジトリやラベルなどを変える GitHub の操作、Issue を消すことを書く。理由: ADR 0161 と [ADR 0163](0163-loop-merges-pull-requests-by-itself.md) のとおり、コミット・push・ブランチの切り替え・pull request の操作は、スクリプトだけが行う。`gh api` では、pull request のマージも、ブランチの書き換えもできる。保護するパスは、ループが変えてはならない。
  - 読むだけの操作（`gh pr view`、`gh run view` など）と、ファイルを元に戻す操作（`git restore`、`git checkout -- （パス）`）は禁止しない。`git checkout` と `git reset` は、まとめて禁止しない。理由: 拒否での遠回りを無くすという、ユーザーの選んだ理由に反する。`git checkout` でのブランチの切り替えと、`git reset` でのコミットの取り消しは、下の HEAD の比較で気づける。
  - 実装とレビューの `deny` には、Issue を閉じる・開き直す・書き換える・コメントすることも書く。理由: これまでも、実装する側とレビューする側には、これらを許可していなかった。これらは、仕分けをする側とスクリプトが行う。
  - レビューする側と仕分けをする側がファイルを変えないこと（[ADR 0162](0162-pick-model-by-difficulty-and-review-in-separate-session.md)、[ADR 0164](0164-loop-triages-issues-between-tasks.md)）は、変えない。これまでは権限で書けなくしていたが、これからは、スキルの決まりと、スクリプトがレビューと仕分けの後に作業ツリーの変更を捨てることで守る。
  - 3 つのファイルの `allow` は消さずに残す。理由: 切り替えの途中で、古いスクリプト（`dontAsk`）が新しいファイルを読んでも、これまでどおり動く。`dontAsk` に戻すときにも使える。
- `claude` を呼んだ前後で、作業ツリーの HEAD（コミットとブランチ）を比べ、変わっていたら、仕組みの失敗としてループを止める。決まりは [実装のループ](../conventions/implementation-loop.md)「止まるとき」に定める。Claude が提案し、ユーザーが選んだ。理由: `deny` はコマンドの文字列で判定するので、別の書き方のコマンドはすり抜ける。コミットとブランチの切り替えは、すり抜けても、スクリプトが気づける。

## 検討した他の案

- 案: 実装する側だけを `bypassPermissions` にし、レビューと仕分けは、許可の一覧に読み取りのコマンドを足す。
  - 採用しなかった理由: Claude が推奨したが、ユーザーは、すべてを `bypassPermissions` にする案を選んだ。レビューでも `grep` と `find` が拒否されていた。
- 案: `bypassPermissions` にせず、拒否されたコマンドを許可の一覧に足していく。
  - 採用しなかった理由: 拒否されるコマンドは、タスクごとに形が違う。許可の一覧は保護するパスにあるので、足すたびに、ユーザーのいる会話が要る。
- 案: モード `auto`（分類器が操作を審査する）か、サンドボックスの中の Bash を自動で許可する設定を使う。Claude が資料を読んで考えた案で、ユーザーには示していない。
  - 採用しなかった理由: ユーザーは、この PC が壊れても構わないとしているので、拒否を減らすための審査や隔離は要らない。どちらも、非対話の `claude -p` でどう動くかを、確かめていない。

## 影響

- ループの `claude` は、禁止した操作のほかは、この Mac の上で何でも実行できる。リポジトリの外のファイルを消すことも、ネットワークを使うこともできる。ユーザーは、この PC が壊れても構わないとしている。
- 禁止は、別の書き方のコマンドですり抜けられる。HEAD の比較で気づけるのは、作業ツリーでのコミットとブランチの切り替えだけである。別の書き方の push や `gh` の操作、ユーザーのチェックアウトの変更には、気づけない。このリポジトリではブランチ保護が使えないので、main への直接の push も止められない（ADR 0161）。
- レビューと仕分けの `claude` も、ファイルを書ける。書いた変更は、スクリプトが、レビューと仕分けの後に捨てる。
- 禁止に当たった操作は、これまでどおり、権限で拒否された操作として記録（`permission_denials`）に残る（「背景」）。実装する側がそれで進められないと答えたときは、ADR 0183 のとおり止まる。
- 切り替えるときは、ループを止め、この変更を main に入れ、ループを始め直す。始め直したら、少ない件数（`--max-tasks 1`）で回して、`claude` が `bypassPermissions` で起動し、タスクを進められることを確かめる。
