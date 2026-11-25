---
type: Decision
title: "ADR 0249"
description: "秘密の情報の検査を、パッチまで固定した gitleaks で行い、確かめる中身（push していないコミットの中身・差分・ファイルの名前、ステージした変更、作業ツリーの変更）をスクリプトが git で、置き換えの参照と出力の形を変える設定に左右されずに取り出して gitleaks の stdin に渡し、毎回の自己点検で検出できることを確かめ、実装のループでは GitHub の origin の main の道具と SHA で、フックを動かさずに、すべての push の直前に確かめて、失敗したら直しに回さずに push せずに止まり、守るのは誤って書いたときまでとし、CI では公式のリリースを SHA-256 で確かめて別のジョブで履歴全体を確かめる"
---

- 日付: 2026-11-25
- 関連仕様: [開発環境と依存の版](../spec/architecture/development-environment.md)「秘密の情報の検査」、[開発環境と依存の版](../spec/architecture/development-environment.md)「開発に要るもの」、[開発環境と依存の版](../spec/architecture/development-environment.md)「開発で使うコマンド」、[テストの方針](../spec/non-functional/testing.md)「CI」、[テストの方針](../spec/non-functional/testing.md)「実行するとき」、[テストの方針](../spec/non-functional/testing.md)「テストの種類と道具」、[リポジトリ構成](../spec/architecture/repository.md)、[実装のループ](../conventions/implementation-loop.md)「機械のチェック」、[実装のループ](../conventions/implementation-loop.md)「回す場所と始め方」、[実装のループ](../conventions/implementation-loop.md)「保護するパス」、[実装のループ](../conventions/implementation-loop.md)「直しと行き詰まり」、[実装のループ](../conventions/implementation-loop.md)「止まるとき」
- 置き換え: なし

## 背景

リポジトリを公開するにあたり、秘密の情報を gitleaks で確かめることにした。CI で確かめ、push の前にも確かめ、実装のループの機械のチェックにも足す（[ADR 0248](0248-publish-repository-on-github.md)）。入れ方と確かめ方の細部は、決まっていなかった。

2026-11-25 に、次のことを確かめた。

- Homebrew の gitleaks は 8.30.1 で、GitHub の gitleaks の最新のリリース（v8.30.1、2026-03-21）と同じだった。作者の Mac に `brew install gitleaks` で入れると、`gitleaks version` は `8.30.1` を返した。
- サブコマンドとオプション（`gitleaks --help`）
  - サブコマンドは `git`・`dir`・`stdin`・`version` である。
  - グローバルのオプションに `--ignore-gitleaks-allow`・`--redact`・`--exit-code`・`--report-format`・`--gitleaks-ignore-path` がある。
  - 設定は、`--config`、環境変数、対象のパスの `.gitleaks.toml` の順で探す。
  - 公式のリリースの README によると、設定の `[extend]` の `useDefault = true` で、組み込みの既定の規則を引き継げる。
- `gitleaks git` が確かめないもの
  - `gitleaks git` は `git log -p -U0` を実行する。`--log-opts` を付けないときは、`--full-history --all` も付ける。
  - 使い捨てのリポジトリで試すと、次の 3 つは、どれも見つからずに終了コード 0 だった: コミットのメッセージ、`.gitattributes` で `-diff` にしたファイル、マージコミットで衝突を直すときに書いた行。
  - 使い捨てのリポジトリで、コミットの blob を消して git が読めないようにすると、gitleaks は `ERR` を出しながら、「no leaks found」と終了コード 0 で終わった。
  - `--pre-commit` は `git diff` を、`--pre-commit --staged` は `git diff --staged` を確かめる。どちらも、git に入っていない新しいファイルを確かめない。
- そのほかの gitleaks のふるまい
  - 行に `gitleaks:allow` と書くと、その行の検出が外れた。`--ignore-gitleaks-allow` を付けると、外れなかった。
  - `gitleaks dir` は、`.gitignore` に従わない（`.gitignore` に書いた `work/` の中のファイルも確かめた）。
  - gitleaks は、見つけたときに `--exit-code` の値（既定は 1）で終わる。設定のファイルが TOML として読めないときと、対象のディレクトリが無いときも、1 で終わった。
  - `gitleaks stdin` に `--report-format json` を付けると、見つかったものの行番号（`StartLine`）が出る。`--redact` を付けると、値（`Secret`・`Match`）は `REDACTED` になった。
- このリポジトリのすべての参照の、差分をテキストとして取った `git log -p -U0 -m --text` の出力は約 15MB で、`gitleaks stdin` で約 4 秒で確かめられた（作者の Mac）。
- 公式のリリース v8.30.1 のファイル
  - `gitleaks_8.30.1_linux_x64.tar.gz` と `gitleaks_8.30.1_checksums.txt` がある。
  - checksums のファイルを取って読むと、`gitleaks_8.30.1_linux_x64.tar.gz` の SHA-256 は `551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb` だった。取ったファイルの値と、GitHub の API が返す値も同じだった。
- 実装のループの push（`tools/implement-loop.py`）
  - ループは、タスクの変更をコミットし、origin の main を取り込んでから、機械のチェックを実行する（`verify`）。
  - 行き詰まりのタスクは、ブランチを push して残す（`mark_stuck`）。main の取り込みで 2 回衝突したときと、実装する側が待つ Issue を挙げずに「進められない」と答えたときは、機械のチェックを通らずに、行き詰まりの push に進む。
  - 保護するパスを origin の main の状態に戻す処理（`revert_protected`）は、`verify` の中にしかない。そのため、行き詰まりの push の時点では、タスクのブランチの `tools/check-secrets.py` と `.gitleaks.toml` が、書き換えられているか、消されていることがある。ループの `claude` の権限は、`Edit` を禁止していても、Bash からの書き換えは止めない。
  - ループには、始めるときに `node` と `npm` の版を確かめる仕組みがある（[ADR 0195](0195-loop-checks-node-and-npm-series-before-start.md)）。
- 別の会話の AI エージェントがレビューし、最初の作りに、すり抜ける経路を見つけた。レビューは、リポジトリの外の写しで実際に試していた。次の経路は、上の事実と同じ理由のものである。
  - 行き詰まりの push で、タスクのブランチの道具が使われる。
  - マージコミット、`.gitattributes`、`gitleaks:allow`、git の失敗、コミットのメッセージ。
  - 標準ライブラリと同じ名前のファイルを `tools/` に置くと、`python3 tools/check-secrets.py` がそれを読み込む。
  - `npm run` は `node_modules/.bin` を `PATH` の先頭に足すので、依存が `gitleaks` という名前のコマンドを入れていると、それが使われる。
  - Claude も、使い捨てのディレクトリで、最後の 2 つが起こることを確かめた（`tools/tempfile.py` を置くと `python3 tools/…` では読み込まれ、`python3 -I` では読み込まれなかった。`node_modules/.bin/gitleaks` を置くと、`npm run` のスクリプトの `gitleaks` は、それを実行した）。
- 2 回目のレビューは、ループの `claude` が、作業ツリーと共有する `.git` を書き換えて検査をすり抜ける経路を、写しで見つけた。
  - ループの経路: `git replace` で、main の `tools/check-secrets.py` の blob を差し替える。`git fetch . +（最初のコミット）:refs/remotes/origin/main` で、手元の `origin/main` を、道具がまだ無いコミットに向ける（道具が無いとして、検査が飛ばされた）。`post-commit`・`post-merge` のフックで、リモートの追跡ブランチを HEAD に向ける（ループのコミットが push 済みと扱われ、範囲が空になった）。
  - スクリプトの経路: `git replace` で、秘密の情報を含む blob を、きれいな blob に置き換えてからコミットする。リポジトリの設定の `i18n.logOutputEncoding=UTF-16`（`UTF-16LE`・`UTF-32` も）で、メッセージの中が見つからない。`log.diffMerges=combined`・`dense-combined` で、マージで足した行が見つからない。
  - Claude は、直した後のスクリプトとループのテストで、これらが止まる・見つかることを確かめた。直す前の形に戻した写しでは、それぞれのテストが失敗した。
- ループの `claude` は `bypassPermissions` で動き、禁止はコマンドの文字列で判定するので、別の書き方のコマンドはすり抜ける（[ADR 0189](0189-run-loop-claude-with-bypass-permissions.md)「影響」）。わざとすり抜ける `claude` は、検査を通さずに、直接 push することもできる。

## 決定

次のことは、すべて Claude が技術的な判断で決めた。

- **gitleaks の版**: パッチの版まで固定する（8.30.1）。作者の Mac には Homebrew で入れ、`brew pin gitleaks` で、`brew upgrade` で版が変わらないようにする。
  - 理由: 検出の規則は gitleaks に組み込まれていて、パッチの版でも変わりうる。版が変わると、同じ変更が、手元では通って CI では失敗する（または逆になる）ことがある。Homebrew は古い版を選んで入れにくいので、固定した版から動かないようにする。
- **スクリプトと実行の仕方**: 確かめるのは、Python のスクリプト `tools/check-secrets.py` にし、`npm run secrets:check` から実行する。`npm run check` には入れない。スクリプトは `python3 -I` で実行する。gitleaks と git は、`PATH` のうち、リポジトリの中と `node_modules` の中を除いた場所から探す。
  - 理由: 開発で使うコマンドは npm のスクリプトにする決まりである（[開発環境と依存の版](../spec/architecture/development-environment.md)「開発で使うコマンド」）。標準ライブラリだけの Python なら、ほかの検査の道具（`tools/check-documents.py`）と同じく、`node_modules` が無くても動く。
  - `npm run check` に入れると、CI の `check` のジョブにも gitleaks が要る。
  - `-I` と `PATH` の絞り方は、「背景」の、ほかのファイルやコマンドが代わりに使われる経路を防ぐためである。
- **確かめる中身の取り出し方**: 確かめる中身は、スクリプトが git で取り出し、`gitleaks stdin` に渡す。`gitleaks git` は使わない。git が失敗したら、スクリプトは失敗する。確かめるのは、次の 3 つである。
  1. push していないコミット（`HEAD --not --remotes`。`--base` を指定したときは、そのコミットを基点にする。`--all` のときは、すべての参照）の、コミットの中身と、差分の足した行と、足した・写した・名前を変えたファイルの名前。コミットの中身は、`git cat-file --batch` で、保存されているとおりのバイトを取り、`encoding` のヘッダがあれば UTF-8 に直したものも確かめる。マージコミットは、親ごとの差分を取る（`--diff-merges=separate`）。
  2. ステージした変更の、足した行と、ファイルの名前。
  3. 作業ツリーの変更の、足した行と、ファイルの名前。インデックスの写しに `git add --all` し、写しと HEAD の差分を取る。
  - 差分の取り方: どのファイルもテキストとして取り（`--text`）、textconv と外部の diff を使わない（`--no-textconv`・`--no-ext-diff`）。
  - git の設定: システムとグローバルの設定（XDG の場所のものを含む）は読まない。置き換えの参照と grafts は使わない（`GIT_NO_REPLACE_OBJECTS`・`GIT_GRAFT_FILE`）。出力の形を変える設定（色、パスの書き方、`log.diffMerges`、`i18n.logOutputEncoding`、フック）は、コマンドで決める。リポジトリの設定のほかのものは読む。
  - 理由（git の設定）: 「背景」の 2 回目のレビューの経路を防ぐ。リポジトリの設定を丸ごと読まない方法は、git に無い（`GIT_CONFIG_NOSYSTEM`・`GIT_CONFIG_GLOBAL` はシステムとグローバルの設定だけに効く）ので、出力の形を変えるものを決めて固める。
  - 理由（コミットの中身を `cat-file` で取ること）: push されるのは保存されているバイトなので、`git log` の文字コードの変換に左右されない。作者やコミットした人の名前も、同じく公開される。
  - 理由（ファイルの名前）: ファイルの名前も、push で公開される。足した行を確かめるのと同じ手間で確かめられる。
  - 足した行の選び方: 差分の hunk の行数を数えて、中身の行と見出しの行を分ける。見つかった行番号から、コミット・ファイルの場所を出す。
  - 理由: 「背景」のとおり、`gitleaks git` は、メッセージ、`-diff` のファイル、マージの衝突の直しを確かめず、git の失敗でも通る。中身を自分で取り出せば、確かめる範囲と、git の失敗を、スクリプトが決められる。
  - 理由（足した行だけを確かめること）: push で新しく公開されるのは、足した行である。消した行は、前のコミットで足したときに確かめている（または、すでに push されている）。
  - 理由（3 つに分けること）: push されるのはコミットで、まだコミットしていない変更も、すぐ後にコミットされて push される。ステージした変更を別に確かめるのは、ステージした後に作業ツリーだけを戻すと、作業ツリーの差分には現れずに、コミットに入るからである。`gitleaks dir` は `.gitignore` に従わないので、作業ツリーの変更は git に集めさせる。
- **gitleaks に渡すオプション**: `--ignore-gitleaks-allow` を渡し、`.gitleaksignore` を読まないように、空のディレクトリを `--gitleaks-ignore-path` に渡す。
  - 理由: 行の中の注記や、設定の外のファイルで、検出を外せないようにする。除くのは、保護するパスの `.gitleaks.toml` だけにする。
- **自己点検**: 毎回、確かめる前に、その場で作ったにせの値（GitHub のトークン、AWS のアクセスキー、秘密鍵の形）を、同じ設定で `gitleaks stdin` に渡し、それぞれの規則（`github-pat`・`aws-access-token`・`private-key`）で見つからなければ失敗する。にせの値は、ファイルに書かない。
  - 理由: 設定が空になった、`useDefault = false` になった、にせの gitleaks が使われた、などのときに、何も見つからないまま通ることを防ぐ。値を固定の文字列で書くと、その文字列自体が検出される。規則を 3 つにしたのは、よく使われる種類のうちで、にせの値をその場で作れるものだからである。AWS のアクセスキーの形のにせの値は、どの文字も 1 回だけ使って作る。gitleaks のこの規則は、文字のばらつき（エントロピー）が 3 未満の値を検出しないので、同じ文字が重なる乱数で作ると、正しい設定でも、まれに見つからないからである（レビューで、同じ文字の重なりを許した作り方の値 50,000 個のうち 8 個が見つからなかった）。
- **終了コード**: 0 は見つからなかった、1 は見つかった、2 は確かめられなかった（gitleaks が無い・版が違う・git や gitleaks の失敗・自己点検の失敗）とする。gitleaks には `--exit-code 99` を渡し、見つけたときと、gitleaks の失敗を分ける。
  - 理由: 既定のままでは、設定の誤りを、秘密の情報が見つかったものとして報告してしまう。版の違いを、違う規則で通ったことにしない。
- **設定**: 設定は `.gitleaks.toml` に置き、既定の規則をそのまま引き継ぐ。いまは除外を書かない。
  - 理由: 公開の前のスキャンで、誤検出が無かった（[ADR 0248](0248-publish-repository-on-github.md)「背景」）。取り出し方を変えた後に、このリポジトリのすべての参照を確かめ直しても、0 件だった。
- **スクリプトのテスト**: スクリプトのテスト `tools/tests/test_check_secrets.py` を置く。一時ディレクトリに作った git のリポジトリで、本物の git と gitleaks を使い、「背景」の経路を含めて確かめる。CI の `secrets` のジョブで実行し、スクリプト・設定・gitleaks の版を変えたときにも実行する。gitleaks が無いときは、飛ばさずに失敗する。
  - 理由: 偽の検査を使うループのテストでは、検査そのものが正しいかを確かめられない。gitleaks が入っているのは `secrets` のジョブだけである。
- **実装のループ**:
  - 機械のチェックに秘密の情報の検査を足し、一覧の先頭にする。失敗したら、直しに回さず、push せずに、仕組みの失敗として止まる。ループを始めるときに、`--check-version`（版と自己点検）を確かめ、通らなければ始めない。
  - 検査は、タスクのブランチの道具ではなく、origin の main の `tools/check-secrets.py` と `.gitleaks.toml` を記録のディレクトリに取り出して実行する（`--repo` でループの作業ツリーを指す）。main の SHA は、手元の追跡ブランチではなく、`git ls-remote` で GitHub から得て、手元に無ければ取ってくる。main に道具があるのに取り出せないときと、始めるときにあった道具が途中で無くなったときは、止まる。
  - 確かめる範囲は、その main の SHA と、origin にタスクのブランチがあれば `git ls-remote` で得たその SHA を基点にする（`--base`）。
  - ループ自身の git は、フックを動かさず（`core.hooksPath` を `/dev/null`）、置き換えの参照と grafts を使わずに実行する。
  - ループの push は、すべて 1 つの関数（`push_branch`）を通し、その直前に、main の道具で検査を実行する。pull request のための push と、行き詰まりの push のどちらも同じである。
  - 理由（直しに回さないこと）: ループはコミットを書き換えないので、秘密の情報を含むコミットは、直しの回で消しても、push すれば履歴に残る。直しに回すと、直しても通らず、最後は行き詰まりとして push してしまう。見つかったものが本物かどうか、無効にするかどうかは、ユーザーが判断することである。
  - 理由（main の道具を使うこと）: 「背景」のとおり、行き詰まりの push の時点では、タスクのブランチの道具が書き換えられていることがある。ループは、保護するパスの変更を元に戻し、差分に残ればマージしないので、タスクの作業で書き換えた道具が、ループのマージで main に入ることは無い（わざとすり抜ける操作は、下の「守る範囲」のとおり、守りの外である）。
  - 理由（`ls-remote`・フック・置き換えの参照）: 「背景」の 2 回目のレビューの経路を、少ない手間で防げる。手元の追跡ブランチと、置き換えの参照と、フックは、ループの `claude` が作業ツリーと共有する `.git` で変えられる。
  - 理由（途中で道具が無くなったら止まること）: ループのマージでは、保護するパスの変更は main に入らない。ふつうに作業していれば、origin の main から道具が消えるのは、ユーザーのいる会話で消したときだけで、ループを回している途中では起きない。起きたら、何かがおかしいので、飛ばさずに止まる。
  - 理由（push を 1 か所に寄せること）: どの経路の push も、検査を通った後にだけ行われることを、コードの形で保つ。
  - 理由（始める前に確かめること）: [ADR 0195](0195-loop-checks-node-and-npm-series-before-start.md) と同じく、どのタスクも同じ理由で止まることを防ぐ。
  - 出力は、記録のディレクトリにだけ残し、pull request と Issue には書かない。理由: 値は伏せてあるが、見つかったファイルは分かる。止まったときに読むのは、作者である。
  - これは、[ADR 0161](0161-run-implementation-loop-locally-and-park-human-work.md) の行き詰まりと止まるときの決まりと、[ADR 0162](0162-pick-model-by-difficulty-and-review-in-separate-session.md) の直しの回数の決まりに、秘密の情報の検査が失敗したときの例外の条件を足すものである。どちらの ADR の文も変えないので、注記は足さない。
- **保護するパス**: `.gitleaks.toml`・`.gitleaksignore`・`tools/check-secrets.py`・`tools/tests/test_check_secrets.py` を、保護するパスに足す。
  - 理由: ループがこれらを変えると、自分の変更が検査を通るように、検査を弱めることができてしまう。ループは main の道具を使うが、変えたものがマージされれば、次から main の道具になる。
- **CI**: ほかのものとは別のジョブ（`secrets`）で、`actions/checkout` の `fetch-depth: 0` で履歴の全部を取り、スクリプトのテストと `python3 -I tools/check-secrets.py --all` を実行する。
  - gitleaks は、固定した版の公式のリリースの Linux 用のファイルを取り、ワークフローに書いた SHA-256 と合うことを `sha256sum --check` で確かめてから使う。
  - サードパーティの Action（`gitleaks-action`）は使わない。権限は `contents: read` のままにする。
  - 理由: `check` のジョブは Tauri のビルドで長いので、待たずに結果が出るようにする。`fetch-depth` の既定（1）では、最後のコミットしか確かめられない。
  - 理由: SHA-256 をワークフローに書けば、配布元のファイルが差し替えられても気づける。スクリプトを使えば、CI でも版と自己点検を確かめられ、手元と同じコマンドで確かめられる。サードパーティの Action は、Action の中身と版を別に確かめる必要がある。
- **版を変えるとき**: 同じ変更で、仕様の表、スクリプトの `GITLEAKS_VERSION`、ワークフローの `GITLEAKS_VERSION` と `GITLEAKS_SHA256` を直す（[開発環境と依存の版](../spec/architecture/development-environment.md)「秘密の情報の検査」）。
- **守る範囲**: この検査が守るのは、人や AI エージェントが、誤って秘密の情報を書いてしまったときと、うっかり検査を弱めてしまったときである。わざと検査をすり抜けようとする操作（作業ツリーと共有する `.git` の参照・設定・オブジェクトを書き換える、`origin` の URL を変える、検査を通さずに直接 push する、など）は、守りの外とする。そのときに残る守りは、GitHub の Push protection と Secret scanning と、CI の検査である。仕様と運用ルールにも、この範囲を書く（[開発環境と依存の版](../spec/architecture/development-environment.md)「秘密の情報の検査」、[実装のループ](../conventions/implementation-loop.md)「機械のチェック」）。
  - 理由: 「背景」のとおり、ループの `claude` の権限は、わざとすり抜ける操作を止められない。`.git` を書き換える経路は多く、すべてを閉じるには、ループの `claude` を、リポジトリの `.git` に書けない環境に閉じ込める必要がある。それは、[ADR 0189](0189-run-loop-claude-with-bypass-permissions.md) の権限の限界と同じ問題で、この検査だけでは解けない。安く固められる経路だけを固め、守れる範囲を、言い過ぎずに書く。
- **push の前の確認**: git のフック（`pre-push`）では行わず、手で `npm run secrets:check` を実行する決まりにする（[テストの方針](../spec/non-functional/testing.md)「実行するとき」）。
  - 理由: フックの設定はリポジトリの外（`.git/hooks` や `core.hooksPath` の設定）にあり、clone ごとに入れる手間が要る。ループの作業ツリーとフックを共有すると、ループの push の仕組みにも影響する。コミット前の確認（`npm run check`）も、手で実行する決まりにしてある。

## 検討した他の案

- 案: gitleaks を系列（8.30）だけで固定する。
  - 採用しなかった理由: 「決定」のとおり、パッチの版でも規則が変わりうる。
- 案: `gitleaks git` の `--log-opts` に `-m --text --no-textconv` などを足し、作業ツリーとステージした変更は `--pre-commit` で確かめ、git の失敗は gitleaks の出力の `ERR` を見て決める。
  - 採用しなかった理由: コミットのメッセージを確かめられない。git の失敗を、gitleaks のログの文字で見分けることになる。
- 案: 作業ツリーの全部のファイルを `gitleaks dir` で確かめる。
  - 採用しなかった理由: `.gitignore` に従わないので、`work/` などの git に入らないものまで確かめてしまう。
- 案: ループで、秘密の情報の検査の失敗も、ほかのチェックと同じく直しに回す。
  - 採用しなかった理由: 「決定」のとおり、コミットした秘密の情報は、直しの回では消せない。
- 案: ループで、タスクのブランチの道具を使い、行き詰まりの push の前にも、保護するパスを main の状態に戻す。
  - 採用しなかった理由: 戻すと、行き詰まりのブランチに残す作業が変わる。道具を main から取り出すほうが、作業に触れずに済む。
- 案: CI で `gitleaks-action` を使う。
  - 採用しなかった理由: 「決定」のとおり。
- 案: `pre-push` のフックで、push のたびに自動で確かめる。
  - 採用しなかった理由: 「決定」のとおり。

## 影響

- push の前に `npm run secrets:check` を実行し忘れると、CI が検出するまで気づかない。公開リポジトリでは、そのときには、すでに公開されている。作者が push するときは、忘れないようにする。
- 確かめないもの・見つからないもの（[開発環境と依存の版](../spec/architecture/development-environment.md)「秘密の情報の検査」）
  - 圧縮されたファイル（`.gz`・`.zip`、`.m4a` などの圧縮された形式）の中と、UTF-16 などで書いたファイルの中（作者の Mac で、UTF-16 で書いたにせの値は見つからなかった）。
  - ブランチ・タグの名前と、git のノート。
  - リポジトリの設定（`.git/config`）で決めた clean のフィルタが、`git add` の中身を変えるときは、変えた後の中身を確かめる。
  - `--all` を付けないときは、HEAD から届かないコミット（ほかのローカルのブランチにだけあるもの）を確かめない。
  - 自己点検の 3 つのほかの規則を、設定で外したとき。
  - グローバルの除外を読まないので、グローバルの除外で無視しているファイルも確かめる。そこに秘密の情報があると、push しないファイルでも失敗する。
- わざとすり抜けようとするループの `claude` は、守りの外である（「決定」の「守る範囲」）。
- ループが GitHub に書く文（pull request のタイトル・本文、Issue のコメント）と、ループの `claude` が直接書く Issue とコメントは、この検査をしない。Issue #267 に起票した。
- 作業ツリーの検査では、インデックスの写しへの `git add` で、どこからも参照されない blob が、手元の git に書かれる。push はされず、git の片付け（`gc`）で消える。
- CI の限界は、[テストの方針](../spec/non-functional/testing.md)「CI」にある。
  - 衝突のある pull request では、ワークフローが走らない。
  - pull request では、その pull request の版の道具と設定が使われる。
  - GitHub の公式の文書（「Events that trigger workflows」、2026-11-25 に確かめた）は、`pull_request` のワークフローが衝突のある pull request では走らないこと、pull request のマージのブランチで動くことを書いている。
  - これらは、push の前の検査、fork からの pull request のワークフローの承認、Interaction limits で補う（[ADR 0248](0248-publish-repository-on-github.md)）。
- CI の `secrets` のジョブは、取ってきたすべてのブランチを確かめる。origin に秘密の情報を含むブランチが 1 つでもあると、ほかのブランチと pull request の CI も失敗する。
- `brew pin gitleaks` をしたので、`brew upgrade` で gitleaks は上がらない。gitleaks を上げるときは、`brew unpin gitleaks` してから上げ、「決定」の 3 か所（仕様の表、スクリプト、ワークフロー）を同じ変更で直す。
- 誤検出でループが止まったときは、作者が、ユーザーのいる会話で `.gitleaks.toml` に除外を足す。
- CI のジョブは、公開して GitHub Actions が走るようになるまで試せない。
  - 手元で確かめたこと: YAML として読めること、gitleaks を入れる手順（ダウンロードと SHA-256 の確かめ）が通ること。手元では `sha256sum --strict` の代わりに、macOS の `shasum -a 256 --check --strict` で確かめた。
  - ubuntu-latest の `sha256sum` で通ることは、まだ確かめていない。
