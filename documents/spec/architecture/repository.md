---
type: Specification
title: "リポジトリ構成"
description: "リポジトリ直下のディレクトリ（`.github/workflows/`、`documents/`、`design-system/`、`content/`、`prompts/`、`runner/`、`tools/`、`src/`、`src-tauri/`）とファイル（`README.md`、`.gitleaks.toml`、`.gitignore`、`skills-lock.json`、`package.json`、`package-lock.json`）の役割と、git に入れないもの（Remotion のスキルなど）と、リポジトリを GitHub で公開することと、そのための GitHub の設定（外部の人の Issue・pull request を受け付けない設定と、その期限を含む）を定める。"
---

```
progrust-app/
├── .github/workflows/   # CI の定義（spec/non-functional/testing.md）
├── documents/           # ドキュメント（仕様、設計判断記録、運用ルール）
├── design-system/       # デザインシステムと画面デザインのファイル（spec/screens/design-system-files.md、spec/screens/ui-design-files.md）
│   ├── progrust/        # 画面のデザインシステム「Progrust」
│   ├── progrust-video/  # 動画のデザインシステム「Progrust Video」
│   └── ui-design/       # 画面デザイン「Progrust UIデザイン」
├── content/             # 教材の原本（動画のコードと音声を含む。形式は spec/content-format/index.md）
├── prompts/             # AI への指示文の原本
├── runner/              # 許可クレートの定義（allowed-crates.toml と Cargo.lock。spec/exercise-runner/crates-and-workspace.md）
├── tools/               # 教材チェックツール、音声を作るツール、デザインシステムの検査の道具、実装のループのスクリプトなどの開発用ツール（spec/content-guidelines/check-tool.md、spec/content-format/video/voice.md、spec/screens/design-system-files.md、conventions/implementation-loop.md）
├── src/                 # フロントエンド
│   └── video/           # 動画の共通部品（プレーヤー・タイムライン・教材用の部品。spec/content-format/video/scenes.md）
├── src-tauri/           # バックエンド
│   ├── build.rs         # ビルドスクリプト。同梱する教材と許可クレートの定義（`runner/`）を選んで写す（spec/architecture/data-locations.md「アプリ同梱データ（`{app_resources}`）」）
│   ├── build/           # ビルドスクリプトの部品。テストからも使う
│   ├── src/bin/         # 教材チェックツールが使う、判定だけを行うコマンドラインのプログラム。Cargo の feature `content-check` を付けたときだけビルドする（spec/content-guidelines/check-tool.md）
│   ├── app-icon.svg     # アプリのアイコンの元の絵（spec/screens/app-icon.md）
│   ├── icons/           # アプリに入れるアイコンのファイル。元の絵から作る（spec/screens/app-icon.md「変える手順」）
│   └── tests/           # バックエンドのテストと、テスト用の偽のツール（spec/non-functional/testing.md）
├── README.md            # GitHub でリポジトリを見に来た人向けの説明（下の「README」）
├── .gitleaks.toml       # 秘密の情報の検査（gitleaks）の設定（spec/architecture/development-environment.md「秘密の情報の検査」）
├── .gitignore           # git に入れないもの（下の「git に入れないもの」）
├── skills-lock.json     # git に入れないエージェントのスキルの、配布元と版（spec/architecture/development-environment.md「エージェントのスキル」）
├── package.json         # JavaScript の依存と npm のコマンド（spec/architecture/development-environment.md）
└── package-lock.json    # JavaScript の依存のロックファイル
```

## README

リポジトリのルートの `README.md` は、GitHub でこのリポジトリを見に来た外部の人が読む。次のことを書く。

- アプリ『Progrust』の概要。作者が自分の Rust の学習のために作っている個人開発のプロジェクトで、開発の途中であり、アプリを配布していないこと（[公開範囲と配布](../non-functional/distribution.md)）。目指しているアプリの姿と、いまできていることを分けて書く。いまできていることは、大まかに書き、細かい進み具合は書かない。
- 動画の音声（`content/` の `.m4a`）は VOICEPEAK で作ったもので、他の用途への使用を認めないこと。ナレーターの声優名は書かない（[非機能要件の未決事項](../non-functional/open-questions.md)「一般公開をする場合に決めること」）。
- ライセンスの扱い。リポジトリに LICENSE を置かない。コードと教材は誰でも見られるが、GitHub の利用規約が公開リポジトリについて認める範囲（GitHub の中での閲覧と fork）を除き、再利用（コピー・改変・再配布）は認めない。
- Issue と pull request を受け付けていないこと（下の「GitHub の設定」）。
- 開発者向けの細かい手順は書き写さず、`documents/` の文書を指す。作者の名前、メールアドレス、手元のパスなどの個人の情報は書かない。

## git に入れないもの

`.gitignore` で、次のものを git に入れない。

| もの | 内容 |
| --- | --- |
| `work/` | 一時作業用（`CLAUDE.md` の「その他」） |
| `node_modules/`、`dist/`、`src-tauri/target/`、`src-tauri/gen/`、`__pycache__/`、`.DS_Store` | 依存とビルドの出力、OS やツールが作るもの |
| `.agents/`、`.claude/skills/remotion-best-practices` | Remotion のスキル（配布元は GitHub の `remotion-dev/skills`）。リポジトリで配らない。手元では `skills-lock.json` から入れ直す（[開発環境と依存の版](development-environment.md)「エージェントのスキル」） |

- `.claude/skills/` のうち、ここに挙げないスキル（`implement-task` など、このリポジトリのために書いたもの）は git に入れる。

## 公開

- リポジトリは GitHub で公開（public）する。コードと教材（動画の音声を含む）とドキュメントと、コミットの履歴、Issue、pull request が、誰でも見られる。
- リポジトリの公開とアプリの一般公開（配布）との関係は、[公開範囲と配布](../non-functional/distribution.md) にある。
- push の前と CI で、秘密の情報が入っていないかを gitleaks で確かめる（[開発環境と依存の版](development-environment.md)「秘密の情報の検査」、[テストの方針](../non-functional/testing.md)「CI」）。

## GitHub の設定

リポジトリの設定は、作者が GitHub で行う。

| 設定 | 値 |
| --- | --- |
| 公開範囲 | public |
| Interaction limits（Issue・コメント・pull request を書ける人の制限） | 共同作業者だけ（API の `limit` は `collaborators_only`）。期限は最長の 6 か月（`expiry` は `six_months`） |
| fork からの pull request のワークフロー | すべての外部の人のものを、承認してから実行する（API の `approval_policy` は `all_external_contributors`） |
| `GITHUB_TOKEN` の既定の権限 | 読み取りだけ（`default_workflow_permissions` は `read`）。ワークフローも `contents: read` を書く |
| Secret scanning と Push protection | 有効 |
| Dependabot alerts | 有効 |

- Interaction limits は、期限が来ると外れ、誰でも Issue・コメント・pull request を書けるようになる。期限が来る前に、設定し直す。期限は `gh api repos/{所有者}/{リポジトリ}/interaction-limits` の `expires_at` で見る。実装のループのマイルストーンの区切りでも確かめる（[実装のループ](../../conventions/implementation-loop.md)「マイルストーンの区切り」）。
- 外部の人が書いた Issue・コメント・pull request は、AI エージェントも実装のループも読まない（[Issue の起票](../../conventions/issues.md)「オーナー以外が書いたもの」）。
