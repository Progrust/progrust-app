---
type: Decision
title: "ADR 0248"
description: "GitHub のリポジトリを公開し、LICENSE は置かずに再利用を認めず、外部の人の Issue と pull request は受け付けずに Interaction limits で絞り、秘密の情報を gitleaks で CI と push の前に確かめる"
---

- 日付: 2026-11-25
- 関連仕様: [リポジトリ構成](../spec/architecture/repository.md)「公開」、[リポジトリ構成](../spec/architecture/repository.md)「README」、[リポジトリ構成](../spec/architecture/repository.md)「GitHub の設定」、[公開範囲と配布](../spec/non-functional/distribution.md)、[非機能要件の未決事項](../spec/non-functional/open-questions.md)、[開発環境と依存の版](../spec/architecture/development-environment.md)「秘密の情報の検査」、[テストの方針](../spec/non-functional/testing.md)「実行するとき」、[テストの方針](../spec/non-functional/testing.md)「CI」、[実装のループ](../conventions/implementation-loop.md)「機械のチェック」、[実装のループ](../conventions/implementation-loop.md)「マイルストーンの区切り」
- 置き換え: なし

## 背景

Issue #266 の目的は、次のとおりである。非公開のリポジトリでは GitHub Actions の利用時間に上限があり、2026-11-25 に CI のジョブが開始されなくなった（#260）。公開リポジトリでは、標準のランナー（`ubuntu-latest`）の利用は無料で、上限が無い。公開の前に、秘密の情報が漏れないようにし、再配布の許可がはっきりしないファイルを外す。

Issue #266 には、作者が決めたこととして、次のことが書かれている（2026-11-25）。

- VOICEPEAK の音声は、規約の許可の範囲（個人の学習を目的としたページで、内容説明に合成音声を使用する）の使い方で、「素材としての配布」には当たらないと判断した。音声ファイルはリポジトリに残す。
- コミットの履歴の作者のメールアドレスが公開されることは了承済み。

アプリは一般公開せず、作者の Mac でビルドして使い、配布物を作らない（[ADR 0091](0091-no-distribution-or-auto-update-until-publication.md)）。VOICEPEAK の音声と Remotion をアプリに同梱して配布できるかは、一般公開をする場合に決める未決事項である（[非機能要件の未決事項](../spec/non-functional/open-questions.md)）。

外部の人が書いた Issue・コメント・pull request を、AI エージェントと実装のループが読まない仕組みは、公開の前提として、先に作った（[ADR 0247](0247-read-only-owner-written-issues-comments-and-pull-requests.md)、Issue #265）。

公開の前のスキャンの結果は、次のとおりである（2026-11-25）。

- gitleaks 8.30.1（その時点の最新）で、GitHub から取り直したミラー（ブランチ 5、タグ 0、`refs/pull/*` 87。すべての参照から届くコミット 259 個）を `gitleaks git --log-opts="--all"` でスキャンし、0 件だった。Issue 182 件、pull request 84 件、コメント 204 件の文を書き出して `gitleaks dir` でスキャンし、0 件だった。誤検出も無かった。
- その後に main に 1 コミット（#265）が増えたので、作者の Mac の手元のリポジトリのすべての参照（ローカルのブランチ 78、リモートの追跡ブランチ 13、タグ 0。参照から届くコミットは 257 個で、gitleaks の数えでは 249 個。gitleaks は、足した行の無いコミットを数えない）を、同じ版の gitleaks と、このリポジトリの設定（`.gitleaks.toml`）で、`python3 tools/check-secrets.py --all` でスキャンし直し、0 件だった。
- GitHub の `GITHUB_TOKEN` の既定の権限は、すでに読み取りだけである。リポジトリに登録された Secrets・Variables は無い。

## 決定

- GitHub のリポジトリを公開する（[リポジトリ構成](../spec/architecture/repository.md)「公開」）。Issue #266 の目的による。リポジトリの公開は、アプリの一般公開（配布）とは別のものとし、[ADR 0091](0091-no-distribution-or-auto-update-until-publication.md) の判断（アプリを配布しないこと）は変えない（[公開範囲と配布](../spec/non-functional/distribution.md)）。
- 音声ファイルはリポジトリに残す。Issue #266 に書かれた、作者の判断による。アプリに同梱して配布できるかは、この判断に含まれず、未決事項に残す（[非機能要件の未決事項](../spec/non-functional/open-questions.md)「一般公開をする場合に決めること」）。README に、音声の他の用途への使用を認めないことを書き、声優名は書かない。Issue #266 の「作るもの」による。
- リポジトリに LICENSE を置かない。コードは誰でも見られるが、再利用（コピー・改変・再配布）は認めない扱いにし、README にそう書く。
  - 作者が 2026-11-25 に決めた。作者は、Claude が示した次の説明を受けて決めた。作者自身の言葉での理由は、記録していない。
  - Claude が示した説明: 後から LICENSE を足すことはできるが、一度付けたオープンなライセンスは取り消しにくい。
  - README の書き方（GitHub の利用規約で認められる、GitHub の中での閲覧と fork を除くこと）は、Claude が決めた。理由: GitHub の利用規約（「GitHub Terms of Service」の D.5、2026-11-25 に確かめた）は、リポジトリを公開すると、ほかの利用者に、GitHub の機能が認める範囲で、閲覧と fork による複製を許すと書いている。README がそれと食い違わないようにする。
- 外部の人からの Issue と pull request は受け付けない。
  - 次のことは、作者が 2026-11-25 に決めた: 受け付けないこと、公開した後に GitHub の Interaction limits を設定すること、README に受け付けていないことを書くこと、オーナー以外が書いたものを読まない仕組み（[ADR 0247](0247-read-only-owner-written-issues-comments-and-pull-requests.md)）と二重の守りにすること。設定は作者が行う。
  - 作者が決めるときに、Claude が示した説明: Interaction limits は、Issue・コメント・pull request を書ける人を、一定期間、共同作業者だけに絞る設定で、最長 6 か月で切れるので、切れる前に設定し直す必要がある。作者自身の言葉での理由は、記録していない。
  - 設定の値（`limit` を `collaborators_only`、`expiry` を最長の `six_months` にすること）は、Claude が決めた細部である。理由: `collaborators_only` は、`limit` の 3 つの値のうち、書ける人をいちばん狭く（リポジトリに書き込みの権限のある人だけに）絞る値で、外部の人からの Issue と pull request を受け付けないという、作者が決めたことに合う。`six_months` は、設定し直す回数がいちばん少ない。
  - Interaction limits は、期限が来ると外れる。2026-11-25 に GitHub の公式の文書（REST API の「Interactions」と、「Limiting interactions in your repository」）で確かめたことは、次のとおりである。設定は `PUT /repos/{owner}/{repo}/interaction-limits` で、`limit` は `existing_users`・`contributors_only`・`collaborators_only` のどれか、`expiry` は `one_day`（既定）・`three_days`・`one_week`・`one_month`・`six_months` のどれかで、期限が来ると自動で外れる。最長は 6 か月である。設定し直す必要があることを後から分かるように、期限の見方を [リポジトリ構成](../spec/architecture/repository.md)「GitHub の設定」に書き、実装のループのマイルストーンの区切りでユーザーがすることに、期限を確かめることを足す（[実装のループ](../conventions/implementation-loop.md)「マイルストーンの区切り」）。どこに書くかは、Claude が決めた。理由: マイルストーンの区切りは、ユーザーがまとめて状態を見ると決めてある時点で、6 か月より短い間隔で来る見込みである。
- 秘密の情報を gitleaks で確かめる。CI で push と pull request のたびに実行することは、Issue #266 の「作るもの」による。CI だけでなく、push の前にも実行し、実装のループの機械のチェックにも足すことは、作者が 2026-11-25 に決めた。作者の理由: CI が検出するのは push の後で、公開リポジトリでは、検出した時点ですでに漏れている。入れ方と確かめ方の細部は、Claude が技術的な判断で決めた（[ADR 0249](0249-scan-secrets-with-pinned-gitleaks-before-push-and-in-ci.md)）。
- Remotion のスキルをリポジトリから外す。Issue #266 の「作るもの」による。外し方は [ADR 0250](0250-keep-remotion-skill-out-of-git-and-restore-from-lock.md)。
- 公開の前のスキャンの結果（「背景」）を、Issue #266 の受け入れ条件「コミットの履歴全体を gitleaks でスキャンして、漏れが見つからない」の記録とする。

## 検討した他の案

- 案: オープンなライセンスの LICENSE を置く（Issue #266 の「作者が決めること」の、置くほうの選択肢）。
  - 採用しなかった理由: 作者が、置かないことを選んだ。「決定」に書いた説明のとおり、一度付けたオープンなライセンスは取り消しにくい。
- 案: Interaction limits を使わず、オーナー以外が書いたものを読まない仕組み（ADR 0247）だけで守る。
  - 採用しなかった理由: 作者が、二重の守りにすると決めた。
- 案: gitleaks を CI だけで実行する。
  - 採用しなかった理由: 作者の理由（「決定」）のとおり、CI が検出した時点で、公開リポジトリではすでに漏れている。

## 影響

- コード、教材、ドキュメント、コミットの履歴（作者のメールアドレスを含む）、Issue、pull request が、誰でも見られるようになる。
- push する前に `npm run secrets:check` を実行する手間が増える。gitleaks が入っていない、または版が違うと、実装のループは始まらない。
- Interaction limits は、最長でも 6 か月で外れる。作者は、期限が来る前に設定し直す。
- [ADR 0149](0149-testing-policy.md)、[ADR 0161](0161-run-implementation-loop-locally-and-park-human-work.md)、[ADR 0163](0163-loop-merges-pull-requests-by-itself.md) は、リポジトリが private であることを、理由や影響の一部にしている（macOS の runner の料金、GitHub Actions の無料の枠、private のリポジトリでブランチ保護が使えないこと）。リポジトリを公開すると、この前提は変わる。ここでは、それらの判断（CI を ubuntu-latest で実行すること、ループを作者の Mac で回すこと、ループが自分でマージすること）を変えない。ADR 0149 の「影響」のとおり、見直すときは新しい ADR にする。
- 公開と、公開の直後の GitHub の設定（公開範囲、fork からの pull request のワークフローの承認、Secret scanning、Push protection、Dependabot alerts、Interaction limits）は、作者が行う。設定の値は [リポジトリ構成](../spec/architecture/repository.md)「GitHub の設定」にある。
- CI は、公開して GitHub Actions が走るようになるまで試せない。秘密の情報を含む変更で CI が失敗することは、公開した後に、作者が試しのブランチで確かめる。
