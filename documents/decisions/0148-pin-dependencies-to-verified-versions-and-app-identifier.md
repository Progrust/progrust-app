---
type: Decision
title: "ADR 0148"
description: "依存の版を技術検証で動いた系列にそろえて仕様に書き、細かい版はロックファイルを正とし、パッケージマネージャーを npm に、アプリの識別子を com.progrust.desktop（開発時の起動は com.progrust.desktop.dev）にする"
---

- 日付: 2026-11-22
- 関連仕様: [開発環境と依存の版](../spec/architecture/development-environment.md)、[アプリの技術構成](../spec/architecture/tech-stack.md)、[データの保存場所](../spec/architecture/data-locations.md)「学習者データ（`{app_data_dir}`）」、[画面の見た目とテーマ](../spec/screens/appearance.md)「デザインシステムの写し」
- 置き換え: なし

## 背景

[アプリの技術構成](../spec/architecture/tech-stack.md) は、採用技術を「Tauri 2」「React + TypeScript（ビルドは Vite）」のように名前だけで定めていた。依存の版、パッケージマネージャー、開発に要るもの、アプリの識別子（Tauri の `identifier`）、lint と整形の道具、開発で使うコマンドは、仕様のどこにも無かった。アプリのコードを書き始める前に、これらを決めておく必要があった。

実装前の技術検証では、次の版の組み合わせで、作者の Mac の WKWebView での動画の再生・音声・Monaco Editor・CSP・アセットプロトコルなどを確かめてある（各文書の「環境」）。

- Tauri CLI 2.12.6、`tauri` 2.12.7、`@tauri-apps/api` 2.12.5、React 19.3.1、Vite 8.3.8、TypeScript 6.0、`remotion`・`@remotion/player` 4.0.545、Node.js v25.9.0、npm 11.12.1（[Remotion Player の WebView での動作と性能の技術検証](../verification/issue-1-remotion-player-webview.md)、[レッスンごとの動画の遅延読み込みの技術検証](../verification/issue-4-lazy-load-lesson-video.md)、[音声をアプリ同梱データから asset:// で読む再生の技術検証](../verification/issue-46-serve-narration-via-asset-protocol.md)）
- `monaco-editor` 0.57.0、`@monaco-editor/react` 4.7.0（[Monaco Editor を Tauri の中で使えるかの技術検証](../verification/issue-21-monaco-in-tauri.md)）
- `eslint` 10.15.0、`typescript-eslint` 8.73.0、`stylelint` 17.16.0、`@remotion/bundler`・`@remotion/renderer` 4.0.545（[動画の描画チェックを CI で実行できるかの技術検証](../verification/issue-7-render-check-in-ci.md)）
- Claude Code 2.1.329（[claude -p の隔離起動の技術検証](../verification/issue-8-claude-isolated-invocation.md)）
- Xcode は Command Line Tools だけ（[署名なしの macOS アプリの自動更新の技術検証](../verification/issue-19-updater-unsigned-macos.md)）
- rustc は、ほとんどの検証で 1.99.1、Issue #46 の検証で 1.100.0 だった。学習者の環境に必要な rustc の最低バージョンは 1.100.0 である（[ADR 0115](0115-minimum-rustc-1-100.md)）。

Tauri の公式の資料と CLI のソースでは、次のことを確かめた（2026-11-22）。

- `identifier` は逆ドメイン名の形で、英数字・ハイフン・ピリオドだけを使える。ほかのアプリと重ならない値にする。既定の `com.tauri.dev` は `tauri build` が拒否する。
- `identifier` は、macOS のバンドル ID になり、`app_data_dir()`（`{app_data_dir}`）と WebView のデータのディレクトリの名前に使われる。変えると、アプリは別の場所のデータを読むようになり、それまでのデータを読まなくなる。
- `tauri build` は、`identifier` が `.app` で終わると警告する。
- Tauri CLI は、`tauri` の crate と `@tauri-apps/api`（プラグインは crate と npm のパッケージの組）の版のずれを調べる。

Remotion の公式の資料は、`remotion` と `@remotion/` で始まるパッケージをすべて同じ版にし、`^` を外して版を固定するよう求めている。Remotion 4 は、4.0 のまま、パッチの版を上げて新しい版を出している。

作者は、ドメイン progrust.com を持っている。

ユーザーは、アプリの操作感に関わらない技術的に詳細な部分の判断を Claude に任せている。実装を始める前の調査で、Claude は、開発環境と依存の版を Claude が決めて仕様に書くことと、版を技術検証で動いた版にそろえることを提案し、ユーザーは提案のとおりでよいと示した。そのほかの内容は Claude が決めた。以下の理由は Claude が書いた。

## 決定

- 開発に要るもの、パッケージマネージャー、主な依存の版、版の固定のしかた、版を上げるときに確かめること、アプリの識別子、lint と整形の道具、開発で使うコマンドの名前を、新しい仕様 [開発環境と依存の版](../spec/architecture/development-environment.md) に定める。[アプリの技術構成](../spec/architecture/tech-stack.md) には版を書かず、この仕様を参照する。
- 依存の版は、技術検証で動いた版の系列（メジャーとマイナー）にそろえる。
  - 理由: その組み合わせで、作者の Mac の WKWebView での動画の再生、音声、Monaco Editor、CSP、アセットプロトコルを確かめてある。別の系列にすると、確かめたことが成り立つかが分からなくなる。
- 仕様には系列だけを書き、パッチの版はロックファイル（`package-lock.json`・`src-tauri/Cargo.lock`）を正とする。`package.json`・`Cargo.toml` には、系列の中のパッチの更新だけを許す範囲を書く。
  - 理由: パッチの版まで仕様に書くと、ロックファイルを更新するたびに仕様も直すことになり、二重に管理することになる。系列を仕様に書いておけば、検証した組み合わせから外れる変更（系列を上げること）を、仕様の変更として扱える。
- Remotion は例外として、すべてのパッケージを同じ版にし、パッチまで固定して、仕様にもその版を書く。
  - 理由: Remotion の資料が求めている。Remotion 4 では、パッチの版が新しい版にあたるので、系列（4.0）だけでは、検証した版から外れる変更を止められない。
- Tauri の crate と JavaScript 側のパッケージを同じ系列にそろえる。
  - 理由: 両者は IPC でやりとりする組で、Tauri CLI も版のずれを調べる。
- 技術検証で版を確かめていない依存（テストと整形の道具など）は、仕様に版を書かず、入れたときの版をロックファイルで固定する。
  - 理由: 確かめていない版を仕様に書いても、根拠にならない。
- Node.js は 25.9 系、npm は 11.12 系にする。パッケージマネージャーは npm にする。
  - 理由: 技術検証で使ったものである。Node.js の依存を、フロントエンドと教材チェックツールで 1 組の `package.json` にまとめると、Remotion のパッケージを同じ版にそろえやすい。
- Rust ツールチェーンは、版を固定するファイル（`rust-toolchain.toml`）を置かず、学習者のプログラムのビルドと同じ rustup の stable を使う。`Cargo.toml` の `rust-version` に、学習者の環境の最低バージョンを書く。
  - 理由: 作者の Mac では、開発者と学習者が同じ人で、同じツールチェーンを使う。固定するファイルを置くと、rustup が別の版のツールチェーンを入れ、バックエンドと学習者のプログラムで rustc の版が分かれる。最低バージョンを上げるのは作者が決めたときだけである（[ADR 0117](0117-raise-min-rustc-only-when-author-decides.md)）。
- Claude Code の最低の版は、開発のためには定めない。必要条件は [外部ツール（cargo・claude）への依存](../spec/architecture/external-tools.md)「検出」のまま変えず、開発で動かした版を書くだけにする。
  - 理由: 必要条件は学習者の環境についての決まりで、開発環境の話ではない。
- 版を上げたら、コミット前の確認と、本番ビルドでの CSP の確認を行う。Tauri・React・Vite・Monaco Editor・Remotion を上げたときは、本番ビルドで動画の再生とエディタを人が操作して確かめる。
  - 理由: CSP の違反は本番ビルドでしか見つからない（[ADR 0140](0140-check-csp-in-production-build.md)）。Monaco と Remotion は、版を上げたときに CSP と動作を確かめると決めてある（[ADR 0111](0111-bundle-monaco-and-set-csp.md)、[ADR 0141](0141-allow-data-uri-in-media-src.md) の「影響」）。動画の再生とエディタは、WKWebView での動作を技術検証で確かめた部分で、自動テストでは確かめていない。
- アプリの識別子を `com.progrust.desktop` にする。
  - 理由: 作者の持つドメイン progrust.com を逆にした形で、ほかのアプリと重ならない。`com.progrust.app` は、`.app` で終わるので `tauri build` が警告する。最後の語は、アプリの種類（デスクトップのアプリ）を表す語にした。
- 開発時の起動（`tauri dev`）では、識別子を `com.progrust.desktop.dev` にし、本番ビルドのアプリと別の `{app_data_dir}` を使う。
  - 理由: 作者は、開発しながら、本番ビルドのアプリで実際に学習する。開発中のコードが学習者データ（進捗、学習記録、分析ファイル）を書き換えたり壊したりしないようにする。識別子は後から変えるとデータの場所が変わるので、本番ビルドの識別子とあわせて最初に決める。
- lint は ESLint・typescript-eslint・stylelint・Clippy、整形は Prettier と rustfmt にする。
  - 理由: ESLint と stylelint は、教材チェックツールの lint で使うと決めてあり（[動画のコードの禁止事項](../spec/content-format/video/prohibited-code.md)）、技術検証で版を確かめてある。同じ道具をアプリにも使うと、道具と設定が 1 組で済む。Clippy と rustfmt は Rust ツールチェーンに付いている。Prettier は、TypeScript と CSS の整形で広く使われている。
- 本番ビルドのコマンドは、`.app` だけを作る `tauri build --bundles app` にする。
  - 理由: 配布物（インストーラー）は作らない（[ADR 0091](0091-no-distribution-or-auto-update-until-publication.md)）。技術検証も、この形で本番ビルドを作った。

## 検討した他の案

- 案: パッチの版まで仕様に書く
  - 採用しなかった理由: ロックファイルと仕様の二重管理になる
- 案: 仕様に版を書かず、ロックファイルだけに任せる
  - 採用しなかった理由: 技術検証で確かめた組み合わせから外れる変更（系列を上げること）が、仕様の変更として見えなくなる
- 案: パッケージマネージャーに pnpm を使う
  - 採用しなかった理由: 技術検証は npm で行った。個人の開発で、npm より速いことやディスクの節約の利点は小さい
- 案: アプリの識別子を `com.progrust.app` にする
  - 採用しなかった理由: `.app` で終わる識別子は、`tauri build` が警告する
- 案: 開発時の起動も本番ビルドと同じ識別子にする
  - 採用しなかった理由: 開発中のコードが、作者が実際に学習しているデータを読み書きする
- 案: `rust-toolchain.toml` で Rust ツールチェーンの版を固定する
  - 採用しなかった理由: バックエンドのビルドと学習者のプログラムのビルドで、rustc の版が分かれる

## 影響

- 依存の系列を上げるときは、仕様の表を直し、確かめ直す手間が要る。系列の中のパッチの更新は、ロックファイルの更新だけで行える。
- Node.js は 25.9 系にそろえる。Node.js の系列を上げるときも、同じく仕様の表を直す。
- 作者の Mac では、本番ビルドのアプリの学習者データは `~/Library/Application Support/com.progrust.desktop/` に、開発時の起動のデータは `~/Library/Application Support/com.progrust.desktop.dev/` に置かれる。開発時の起動で本番のデータを使って確かめたいときは、データを写して使う。
- 識別子を後から変えると、それまでの学習者データを読まなくなる。変えるときは、データを移す手順も決める。
- 開発時の識別子は、Tauri CLI の `--config` で設定のファイルを重ねて変える。Tauri の公式の資料は、`dev` と `build` で `--config` に JSON のファイルを渡すと、元の設定に重ねる（JSON Merge Patch）と書いている。このプロジェクトでは試していないので、実装のときに、開発時の起動の `{app_data_dir}` が別の場所になることを確かめる。
- デザインシステムの写しは編集しない決まり（[画面の見た目とテーマ](../spec/screens/appearance.md)「デザインシステムの写し」）なので、lint と整形の対象から外す。
