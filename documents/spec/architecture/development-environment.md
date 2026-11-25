---
type: Specification
title: "開発環境と依存の版"
description: "アプリの開発に要るもの（Node.js、npm、Rust ツールチェーン、Xcode Command Line Tools、gitleaks）、パッケージマネージャー（npm）、主な依存の版と版の固定のしかた（`package.json`・`package-lock.json`・`Cargo.lock`）、版を上げるときに確かめること、アプリの識別子（`com.progrust.desktop`）と開発時の識別子、lint と整形の道具、秘密の情報の検査（gitleaks の版の固定、`npm run secrets:check` が確かめるものと確かめないもの、自己点検、版を変えるとき）、git に入れないエージェントのスキル（Remotion のスキル）を `skills-lock.json` から入れ直す手順、開発時の起動・本番ビルドなどの npm のコマンドの名前を定める。"
---

## 開発に要るもの

アプリは、作者の Mac（[対応環境](../non-functional/environments.md)）で開発し、ビルドする。次のものを入れる。

| もの | 版 | 用途 |
|---|---|---|
| Node.js | 25.9 系 | フロントエンドのビルド、Node.js で動く開発用ツール |
| npm | 11.12 系 | パッケージマネージャー（「パッケージマネージャー」） |
| Rust ツールチェーン（rustup の stable） | 学習者の環境に必要な rustc の最低バージョン以上（[対応環境](../non-functional/environments.md)） | バックエンドのビルドとテスト |
| Xcode Command Line Tools | 作者の Mac の macOS で入るもの | Tauri のビルド |
| gitleaks | 8.30.1 | 秘密の情報の検査（「秘密の情報の検査」） |

- バックエンドのビルドには、学習者のプログラムのビルドに使うもの（[外部ツール（cargo・claude）への依存](external-tools.md)）と同じ Rust ツールチェーンを使う。リポジトリに、ツールチェーンの版を固定するファイル（`rust-toolchain.toml`）を置かない。`src-tauri/Cargo.toml` の `rust-version` には、学習者の環境に必要な rustc の最低バージョンを書く。
- Claude Code は、AI 機能を開発で動かすときに使う。必要条件は、学習者の環境と同じく [外部ツール（cargo・claude）への依存](external-tools.md)「検出」に従い、開発のための最低の版は定めない。開発で動かした版は 2.1.329 である。
- 教材の作者だけに要るもの: VOICEPEAK と ffmpeg（ffprobe を含む）。使い方は [動画の音声](../content-format/video/voice.md)。アプリのビルドと実行には要らない。

## パッケージマネージャー

- JavaScript の依存は npm で入れる。yarn・pnpm・bun は使わない。
- `package.json` と `package-lock.json` は、リポジトリのルートに 1 組だけ置く。フロントエンドと、Node.js で動く開発用ツール（`tools/` の教材チェックツールなど）は、この 1 組の依存を使う。
- `package.json` の `engines` に、Node.js と npm の系列（上の表）を書く。
- 依存を入れるときは `npm ci` を使い、ロックファイルのとおりに入れる。依存を足すときと版を上げるときだけ、`npm install` でロックファイルを更新する。
- Rust の依存は cargo で入れる。`src-tauri/Cargo.lock` をリポジトリに入れる。
- `package-lock.json` と `src-tauri/Cargo.lock` は、どちらもリポジトリに入れる。

## 主な依存の版

| 依存 | 版 |
|---|---|
| `tauri`（crate）、`@tauri-apps/api`、`@tauri-apps/cli` | 2.12 系 |
| `react`、`react-dom` | 19.3 系 |
| `vite` | 8.3 系 |
| `typescript` | 6.0 系 |
| `monaco-editor` | 0.58 系 |
| `@monaco-editor/react` | 4.7 系 |
| `remotion` と `@remotion/` で始まるすべてのパッケージ | 4.0.545 |
| `eslint` | 10.15 系 |
| `typescript-eslint` | 8.73 系 |
| `stylelint` | 17.16 系 |

- 表の「系列」は、メジャーとマイナーの版（`2.12` など）を指す。`package.json` と `src-tauri/Cargo.toml` には、その系列の中のパッチの更新だけを許す範囲（`~2.12.7` の形）を書く。パッチの版は、ロックファイル（`package-lock.json`・`src-tauri/Cargo.lock`）を正とし、この文書に書かない。
- `remotion` と `@remotion/` で始まるパッケージは、すべて同じ版にし、パッチまで固定する。`package.json` には、範囲の記号（`^`・`~`）を付けずに表の版を書く。
- `tauri` の crate と `@tauri-apps/api`・`@tauri-apps/cli` は、同じ系列にそろえる。Tauri のプラグインを使うときは、`tauri-plugin-{名前}` の crate と `@tauri-apps/plugin-{名前}` を、互いに同じ系列にそろえる。
- 表にない依存（ほかのライブラリ、テストと整形の道具、間接の依存）は、入れたときの版をロックファイルで固定する。`package.json` と `Cargo.toml` の範囲は、npm と cargo の既定の書き方（`^`）でよい。
- 表にない依存を入れるときは、`engines` が上の表の Node.js の系列を含み、表の依存の版と peer 依存が合う版を選ぶ。
- 許可クレート（`runner/`）の版は、この文書の対象にしない（[許可クレートと作業領域](../exercise-runner/crates-and-workspace.md)）。

## 版を上げるとき

- 表の依存の系列（Remotion はパッチの版）を上げるときは、同じ変更で、この文書の表と、`package.json`・`Cargo.toml`・ロックファイルを直す。
- 系列の中のパッチの更新は、表を直さずに、ロックファイルの更新だけで行ってよい。
- 版を上げたら（パッチの更新を含む）、次のことを確かめる。
  - コミット前の確認（[テストの方針](../non-functional/testing.md)「実行するとき」）が通る。
  - 本番ビルド（`npm run build`）で、[非機能要件の受け入れ条件](../non-functional/acceptance.md) の CSP の条件を満たす。
  - Tauri・React・Vite・Monaco Editor・Remotion の版を上げたときは、本番ビルドを人が操作して、動画の再生（音声つきの再生、シーク、テーマの切り替え）と、演習画面のエディタ（日本語の入力、ショートカット）が動くことを確かめる。Monaco Editor の版を上げたときは、あわせて、CDN を読まず、同梱の Web Worker で動くことを確かめる（[アプリの技術構成](tech-stack.md)）。

## アプリの識別子

| 項目 | 値 |
|---|---|
| Tauri の `identifier` | `com.progrust.desktop` |
| Tauri の `productName` | `Progrust` |
| 開発時の起動の `identifier` | `com.progrust.desktop.dev` |

- `{app_data_dir}`（[データの保存場所](data-locations.md)）は、作者の Mac では `~/Library/Application Support/com.progrust.desktop/` になる。
- 開発時の起動（`npm run dev`）は、開発時の識別子を使う。開発時の起動の `{app_data_dir}` は `~/Library/Application Support/com.progrust.desktop.dev/` になり、本番ビルドのアプリの学習者データを読み書きしない。開発時の識別子は、開発時の起動だけに重ねる設定のファイル（`src-tauri/tauri.dev.conf.json`）に書き、`npm run dev` が Tauri CLI の `--config` でこのファイルを渡す。

## lint と整形

| 対象 | lint | 整形 |
|---|---|---|
| TypeScript・TSX | ESLint（typescript-eslint）と、TypeScript の型検査（`tsc --noEmit`） | Prettier |
| CSS | stylelint | Prettier |
| Rust（`src-tauri/`） | Clippy（警告をエラーとして扱う） | rustfmt |

- TypeScript の型検査は、アプリのコードと、教材の動画のコード（`content/lessons/{lesson_id}/video/` の `.ts`・`.tsx`）を、同じ設定（`tsconfig.json`）で検査する。
- 動画のコードの禁止事項の検査は、教材チェックツールの lint で行う（[動画のコードの禁止事項](../content-format/video/prohibited-code.md)）。アプリの lint と同じ ESLint・stylelint の版を使う。
- Clippy と rustfmt は、Rust ツールチェーンに付いているものを使う。
- デザインシステムのファイル（`design-system/`。[デザインシステムのファイルと変更の手順](../screens/design-system-files.md)）は、lint と整形の対象から外す。

## 秘密の情報の検査

秘密の情報（API キー、トークン、秘密鍵など）が、push するものに入っていないかを、gitleaks で確かめる。誤って秘密の情報を書いてしまったときに、push の前に止めるためのものである（下の「守る範囲」）。

- gitleaks は、上の表の版（8.30.1）に固定する。パッチの版まで同じでなければならない。作者の Mac には Homebrew で入れ（`brew install gitleaks`）、`brew upgrade` で版が変わらないように `brew pin gitleaks` する。
- 確かめるのは `tools/check-secrets.py` で、`npm run secrets:check`（下の「開発で使うコマンド」）から実行する。Python の `-I`（スクリプトのあるディレクトリと環境変数 `PYTHONPATH` などを使わない）を付けて実行する。
- 確かめる前に、次のことを確かめる。どれかが成り立たないときは、終了コード 2 で失敗する。
  - gitleaks が見つかり、`gitleaks version` が固定した版と同じである。gitleaks と git は、`PATH` のうち、確かめるリポジトリの中と `node_modules` の中を除いた場所から探す。
  - 自己点検: その場で作ったにせの値（GitHub のトークン、AWS のアクセスキー、秘密鍵の形）を、同じ設定で gitleaks に渡すと、それぞれの規則（`github-pat`・`aws-access-token`・`private-key`）で見つかる。にせの値は、ファイルに書かない。
- 確かめるものは、次のとおり。中身は、スクリプトが git で取り出し、gitleaks の `stdin` のサブコマンドに渡す。git か gitleaks が失敗したときは、終了コード 2 で失敗する。どれかに秘密の情報があれば、場所（コミット・ファイル）と規則の名前を表示して、終了コード 1 で失敗する。見つかった値は、出力に出さない。
  - push していないコミット: HEAD から届き、どのリモートの追跡ブランチ（`--remotes`）からも届かないコミット。`--base （SHA）` を付けたときは、追跡ブランチの代わりに、そのコミット（push 先にすでにあるもの。何度でも付けられる）から届かないコミット。`--all` を付けたときは、すべての参照から届くコミット。確かめるのは、コミットの中身（メッセージ・作者など。保存されているとおりのバイトと、`encoding` のヘッダがあれば UTF-8 に直したもの）、差分の足した行、足した・写した・名前を変えたファイルの名前である。マージコミットは、親ごとの差分を確かめる。
  - ステージした変更: インデックスと HEAD の差分の、足した行と、足した・名前を変えたファイルの名前。
  - 作業ツリーの変更: git のインデックスの写しに `git add --all` し、写しと HEAD の差分の、足した行と、足した・名前を変えたファイルの名前。git に入っていない新しいファイルを含み、`.gitignore` で無視するもの（`node_modules/`、`src-tauri/target/`、`work/` など）は含まない。本物のインデックスは変えない。
- git は、次のように実行する。
  - システムとグローバルの設定（XDG の場所の設定・除外・属性を含む）を読まない。グローバルの除外で無視しているファイルも、作業ツリーの変更として確かめる。
  - 置き換えの参照（`refs/replace`）と grafts を使わない。
  - 出力の形を変えるリポジトリの設定は、コマンドで決める（色、パスの書き方、textconv と外部の diff、マージの差分の形 `log.diffMerges`、メッセージの文字コード `i18n.logOutputEncoding`、フック）。差分は、どのファイルもテキストとして取る（`--text`。`.gitattributes` で `-diff`・`binary` としたファイルと、NUL を含むファイルも）。
  - リポジトリの設定（`.git/config`）のほかのものは読む。
- 確かめないもの・見つからないもの
  - 圧縮したもの（`.gz`・`.zip`、`.m4a` などの圧縮された形式）の中と、UTF-16 などの、ASCII の文字が 1 バイトにならない文字コードで書いたファイルの中。base64 にしたものは、gitleaks が戻して確かめるので見つかる。バイナリのファイルでも、そのままの文字として入っているものは見つかる。
  - ブランチ・タグの名前。git のノート（`refs/notes`）は、`--all` を付けたときだけ確かめる。
  - `--all` を付けないときの、HEAD から届かないコミット（ほかのローカルのブランチにだけあるもの）。
  - 自己点検の 3 つのほかの規則を設定で外したときは、自己点検では気づかない。
  - push のほかの形で GitHub に書く文（pull request のタイトル・本文、Issue の本文・コメント）。実装のループが書くものも、確かめていない。
  - リポジトリの設定で、上で決めていない設定（`git add` の中身を変える clean のフィルタなど）が中身を変えるときは、変えた後の中身を確かめる。
- 行に書いた `gitleaks:allow` は効かない（`--ignore-gitleaks-allow`）。`.gitleaksignore` も使わない。除くときは、設定の `.gitleaks.toml` に書く。
- gitleaks の設定は、リポジトリのルートの `.gitleaks.toml` にする。gitleaks に組み込まれた既定の規則をそのまま引き継ぎ（`[extend]` の `useDefault = true`）、誤検出を除くときだけ、ここに除く規則を足す。スクリプトは、`--config` で別の設定を、`--repo` で別のリポジトリを指せる。指さないときは、スクリプトの 1 つ上のディレクトリの `.gitleaks.toml` と、そのディレクトリのリポジトリを使う。
- スクリプトのテストは `tools/tests/test_check_secrets.py` で、本物の git と gitleaks を使う（[テストの方針](../non-functional/testing.md)「テストの種類と道具」）。
- 版を変えるときは、同じ変更で、上の表、`tools/check-secrets.py` の `GITLEAKS_VERSION`、`.github/workflows/ci.yml` の `GITLEAKS_VERSION` と `GITLEAKS_SHA256`（その版の公式のリリースの `gitleaks_{版}_checksums.txt` にある、`gitleaks_{版}_linux_x64.tar.gz` の値）を直し、スクリプトのテストを実行する。
- 実装のループも、機械のチェックと push の前に実行する（[実装のループ](../../conventions/implementation-loop.md)「機械のチェック」）。CI での実行は [テストの方針](../non-functional/testing.md)「CI」。

### 守る範囲

- この検査が止めるのは、人や AI エージェントが、誤って秘密の情報を書いてしまったときと、うっかり検査を弱めてしまったとき（設定を空にした、gitleaks の版が変わった、など）である。
- わざと検査をすり抜けようとする操作は、守りの外である。たとえば、リポジトリの `.git`（参照、設定、フック、オブジェクト）を書き換える操作や、検査を通さずに直接 push する操作である。実装のループの `claude` の権限は、コマンドの文字列で禁止を判定するので、このような操作を止められない（[実装のループ](../../conventions/implementation-loop.md)「権限」）。
- そのときに残る守りは、GitHub の Push protection と Secret scanning（[リポジトリ構成](repository.md)「GitHub の設定」）と、CI の検査（[テストの方針](../non-functional/testing.md)「CI」）である。

## エージェントのスキル

Remotion のスキル（`remotion-best-practices`）は、git に入れない（[リポジトリ構成](repository.md)「git に入れないもの」）。配布元（GitHub の `remotion-dev/skills`）と、入れたコミットは、リポジトリのルートの `skills-lock.json` にある。

- 手元に入れ直すときは、リポジトリのルートで次のコマンドを実行する。`skills-lock.json` に書かれたコミットのファイルが入り、`skills-lock.json` は変わらない。

  ```sh
  npx skills@1.9.0 experimental_install -a claude-code
  ```

  - スキルの本体は `.agents/skills/remotion-best-practices/` に入り、`.claude/skills/remotion-best-practices` には、そこへのシンボリックリンクが置かれる。前からあった `.claude/skills/remotion-best-practices/` は、シンボリックリンクに置き換わる。
  - 入れる前に、`--dry-run` を付けて実行すると、入れるものだけを表示する。
- 実装のループの作業ツリーには、このスキルは無い（[実装のループ](../../conventions/implementation-loop.md)「回す場所と始め方」）。

## 開発で使うコマンド

開発で使うコマンドは、`package.json` の npm のスクリプトにする。

| コマンド | 内容 |
|---|---|
| `npm run dev` | 開発時の起動（`tauri dev --config src-tauri/tauri.dev.conf.json`。開発時の識別子を使う） |
| `npm run build` | 本番ビルド（`tauri build --bundles app`）。`.app` だけを作り、インストーラー（`.dmg` など）は作らない（[公開範囲と配布](../non-functional/distribution.md)）。できた `.app` は `src-tauri/target/release/bundle/macos/Progrust.app` に置かれる |
| `npm run icon` | アプリのアイコンのファイルを、元の絵から作り直す（[アプリのアイコン](../screens/app-icon.md)「変える手順」） |
| `npm run lint` | lint と型検査（上の表の lint のすべて） |
| `npm run format` | 整形（上の表の整形のすべて）。`npm run format:check` は、整形が済んでいるかを確かめるだけで、ファイルを変えない |
| `npm test` | 自動テスト（Vitest と `cargo test`。[テストの方針](../non-functional/testing.md)）。`cargo test` と、`npm run lint` の Clippy は、Cargo の feature `content-check` を付けて、教材チェックツールが使う判定のプログラム（[教材チェックツール](../content-guidelines/check-tool.md)）も対象にする |
| `npm run check` | コミット前の確認。`npm run lint`、`npm run format:check`、`npm test` を順に実行する |
| `npm run secrets:check` | 秘密の情報の検査（上の「秘密の情報の検査」。`python3 -I tools/check-secrets.py`）。push の前に実行する。`npm run secrets:check -- --all` で、すべての参照から届くコミットを確かめる |
| `npm run content:check` | 教材チェックツール（[教材チェックツール](../content-guidelines/check-tool.md)）。`content/` のすべての教材を検査する |
| `npm run runner:lock` | 許可クレートの `Cargo.lock`（`runner/Cargo.lock`）を、`runner/allowed-crates.toml` から作り直す（[許可クレートと作業領域](../exercise-runner/crates-and-workspace.md)「許可クレート」）。クレートの索引を読むので、ネットワークが要ることがある |
| `npm run content:voice` | 音声を作るツール（[動画の音声](../content-format/video/voice.md)）。作者の環境の VOICEPEAK と ffmpeg を使う |

- Tauri が開発時の起動と本番ビルドの前に実行するフロントエンドだけのコマンド（`beforeDevCommand`・`beforeBuildCommand`）は、`npm run dev:web`（Vite の開発サーバー）と `npm run build:web`（Vite のビルド）にする。
