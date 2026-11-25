---
type: Decision
title: "ADR 0250"
description: "Remotion のスキルを、コミットの履歴は書き換えずに、今後のコミットから外して .gitignore し、手元では skills-lock.json から版を固定した skills の CLI で入れ直し、実装のループの作業ツリーには用意しない"
---

- 日付: 2026-11-25
- 関連仕様: [リポジトリ構成](../spec/architecture/repository.md)「git に入れないもの」、[開発環境と依存の版](../spec/architecture/development-environment.md)「エージェントのスキル」、[実装のループ](../conventions/implementation-loop.md)「回す場所と始め方」
- 置き換え: なし

## 背景

Issue #266 は、リポジトリを公開する前に、再配布の許可がはっきりしないファイルを外すことを目的の 1 つにし、Remotion のスキル（`.claude/skills/remotion-best-practices/`）を、`git rm --cached` と `.gitignore` でリポジトリから外し、`skills-lock.json` は残して、入れ直す手順を確かめて文書に書くとしている（[ADR 0248](0248-publish-repository-on-github.md)）。

スキルは 130 個のファイルで、コミット `24a0128` で、`skills-lock.json` と一緒に入った。`skills-lock.json` には、配布元（GitHub の `remotion-dev/skills`）と、入れたコミット（`6f55706d1069e512430582ad86e4e57dc07afe1b`）と、ハッシュがある。

Claude は、2026-11-25 に、作者に次の事実と説明を示した。

- 配布元（GitHub の `remotion-dev/skills`）には、ライセンスのファイルが無い（2026-11-25 に GitHub の API で確かめた。license は null）。再配布の許可がはっきりしないので、リポジトリから外す。
- `git rm --cached` で外しても、過去のコミットにはスキルのファイルが残り、公開される。
- スキルのファイルは、配布元が GitHub で公開しているもので、秘密の情報ではない。公式の導入ツールがプロジェクトのフォルダにコピーして入れたものである。
- 履歴を書き換えると、コミットの番号がすべて変わり、Issue に記録した「確認したコミット」が指す先が無くなる。
- GitHub が pull request ごとに保管しているコミットは、こちらからは書き換えられないとされ、消すには GitHub のサポートへの依頼か、リポジトリの作り直しが要るとされる。この 2 つは、確かめていないことも、作者に伝えた。
- 配布元にだめだと言われたときは、そのときに対応する。

同じ日に、Claude が確かめたことは、次のとおりである。

- `skills-lock.json` を作ったのは、npm のパッケージ `skills`（GitHub の `vercel-labs/skills`。2026-11-25 の最新は 1.9.0）の CLI である。`npx skills@1.9.0 --help` によると、`experimental_install`（別名 `install`）が、`skills-lock.json` からスキルを入れ直す。
- リポジトリの外の一時ディレクトリに `skills-lock.json` だけを写して、`npx skills@1.9.0 experimental_install -a claude-code` を実行すると、`skills-lock.json` のコミットのスキルが入り、`skills-lock.json` は変わらなかった。入ったファイル（130 個）は、リポジトリの `.claude/skills/remotion-best-practices/` と同じだった。
- ただし、置き場所は、リポジトリの形と違った。本体は `.agents/skills/remotion-best-practices/` に入り、`.claude/skills/remotion-best-practices` は、そこへのシンボリックリンクになった。前から `.claude/skills/remotion-best-practices/` があるときも、シンボリックリンクに置き換わった。
- 実装のループは、作業ツリーを戻すたびに、`git clean -fdx -- .claude` で、`.claude/` の中の git に入っていないファイルを、無視の対象のものも含めて消す（`tools/implement-loop.py` の `reset_worktree`）。ループの `claude` が `.claude/settings.local.json` などを書いて、自分の権限や設定を変えるのを防ぐためのもので、テスト（`test_worktree_is_reused_and_cleaned`）もある。
- `skills-lock.json` のほかに、リポジトリの文書・スキル・スクリプトで、Remotion のスキルを参照しているものは無かった（`git grep`）。ループの記録のディレクトリ（`~/Library/Logs/progrust-implement-loop/`）にも、スキルの名前は出てこなかった。

## 決定

- Remotion のスキルは、今後のコミットから外すだけにし、コミットの履歴は書き換えずに、そのまま公開する。作者が、「背景」の、Claude が示した事実と説明を受けて、2026-11-25 に決めた。作者自身の言葉での理由は、記録していない。
- `.gitignore` に `/.agents/` と `/.claude/skills/remotion-best-practices` を書く（[リポジトリ構成](../spec/architecture/repository.md)「git に入れないもの」）。Claude が決めた。理由: 入れ直すと、本体が `.agents/` に入る。`.agents/` を無視しないと、`git add -A` でスキルがまた git に入る（ループのスクリプトも `git add -A` でコミットする）。`.claude/` のほうは、シンボリックリンクにも合うように、末尾に `/` を付けない。
- 手元に入れ直す手順は、`npx skills@1.9.0 experimental_install -a claude-code` にする（[開発環境と依存の版](../spec/architecture/development-environment.md)「エージェントのスキル」）。Claude が決めた。理由: 実際に実行して、同じファイルが入ることを確かめた。CLI の版を固定すると、手順が後から変わりにくい。`experimental_` の名前が付いているので、CLI の版を上げるときは、手順を確かめ直す。
- 実装のループの作業ツリーには、このスキルを用意しない。Claude が決めた。理由: 仕様・運用ルール・スキルのどれも、このスキルを使うことを求めておらず、ループの記録にも使った跡が無い。作業ツリーの `.claude/` に、git に入っていないファイルを置くと、`.claude/` を main と同じに保つ仕組み（「背景」の `reset_worktree`）と食い違う。

## 検討した他の案

- 案: コミットの履歴を書き換えて、スキルのファイルを過去のコミットからも消す。
  - 採用しなかった理由: 作者が選ばなかった。「背景」のとおり、コミットの番号がすべて変わり、pull request ごとに保管されたコミットは消せないとされる。
- 案: スキルをリポジトリに残す。
  - 採用しなかった理由: Issue #266 で、再配布の許可がはっきりしないファイルを外すとした。
- 案: ループのスクリプトが、作業ツリーを戻した後に、スキルを入れ直す（`npx skills` を実行するか、ユーザーのチェックアウトから写す）。
  - 採用しなかった理由: 「決定」のとおり、ループのタスクに要らず、`.claude/` を main と同じに保つ仕組みと食い違う。`npx skills` は、ネットワークと配布元に、ループの 1 周ごとに頼ることになる。

## 影響

- スキルのファイルは、過去のコミット（`24a0128` から #265 のコミットまで）に残り、公開される。配布元に取り下げを求められたときは、そのときに対応を決める。
- 作者の手元のスキルのファイルは、git から外れても残る。git に入っていたころのコミットを取り出してから戻ると、git がスキルのファイルを消すことがある。そのときは、手順のとおりに入れ直す。
- ループのタスクは、このスキルを使えない。Remotion の使い方を調べるのが要るタスクが出て、スキルが要ると分かったときは、ループの作業ツリーへの用意を見直す（新しい ADR にする）。
