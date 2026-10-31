# 設計判断記録（ADR）

仕様書（`../spec`）に定めた仕様について、「なぜその仕様にしたのか」「なぜその仕様に変えたのか」を記録する。

- 仕様書は現時点の事実を書き、ADR は判断の経緯を書く。仕様の内容は ADR に書き写さず、仕様書を参照する。
- 1つの判断につき1ファイルとする。

## 運用ルール

### 書く対象

次のいずれかに当てはまる判断を記録する。文言の調整など、細かい判断は記録しない。

- 他の案を検討したうえで選んだ
- 理由が仕様書から読み取れない
- 後から変えるのが大変
- 後から疑問に思われそう、または別の案が提案されそう

### ファイル

- ファイル名は `NNNN-英語の短い説明.md`（例: `0001-ai-does-not-use-tools.md`）とする。番号は 0001 からの連番で、再利用しない。
- `template.md` をコピーして作る。
- 作成したら、下の索引に1行追加する。

### 変更

- 採用した ADR の「背景」「決定」「検討した他の案」は書き換えない。誤字の修正や、関連仕様の参照先の更新は行ってよい。
- 判断を変えたときは、新しい ADR を作り、「置き換え」に古い ADR の番号を書く。古い ADR の状態は「置き換え済み（→ 新しい番号）」に変え、索引も更新する。
- 判断の対象そのものがなくなったときは、状態を「廃止」に変える。

### 仕様書の「未決事項」との関係

仕様書の「未決事項」が決まったときは、その判断を ADR として記録する。

## 索引

| 番号 | タイトル | 状態 | 関連仕様 |
| --- | --- | --- | --- |
| [0001](0001-learners-solve-on-their-own.md) | 学習者に演習を自力で解かせることを、機能を設計するうえでの原則とする | 採用 | `content-guidelines.md`, `qa-chat.md`, `architecture.md` |
| [0002](0002-target-experienced-programmers.md) | 想定する学習者を他のプログラミング言語の経験者とする | 採用 | `content-guidelines.md` |
| [0003](0003-react-typescript-frontend.md) | フロントエンドに React と TypeScript を使う | 採用 | `architecture.md` |
| [0004](0004-monaco-without-completion.md) | コードエディタに Monaco Editor を使い、補完・LSP 連携は行わない | 採用 | `architecture.md` |
| [0005](0005-slides-in-restricted-html.md) | スライドを、使える部品を決めた HTML で書く | 採用 | `content-format.md`, `content-guidelines.md` |
| [0006](0006-element-ids-are-permanent.md) | 要素 ID を全データで共通にし、公開後は変更・削除しない | 採用 | `content-format.md`, `learning-record-and-analysis.md` |
| [0007](0007-derive-introduced-lesson.md) | 要素の導入レッスンを、要素マスタに持たせず学習順から求める | 採用 | `content-format.md` |
| [0008](0008-judge-mode-per-lesson.md) | 演習の判定方式を、標準出力の比較と隠しテストからレッスンごとに選ぶ | 採用 | `content-format.md`, `exercise-runner.md` |
| [0009](0009-embed-hidden-tests-as-module.md) | 隠しテストを、対象ファイルの末尾に子モジュールとして組み込む | 採用 | `exercise-runner.md` |
| [0010](0010-allow-listed-crates.md) | 演習で許可リストの外部クレートを使えるようにする | 採用 | `exercise-runner.md`, `content-format.md` |
| [0011](0011-prepare-crates-on-first-launch.md) | 許可クレートを初回起動時に一括でビルドし、ロックファイルを同梱して全演習で共有する | 採用 | `exercise-runner.md` |
| [0012](0012-separate-build-dir-without-crates.md) | ビルド先を、許可クレートを使う演習と使わない演習で分ける | 採用 | `exercise-runner.md` |
| [0013](0013-interactive-stdin.md) | 実行時の標準入力を、コンソールから対話的に入力する方式にする | 採用 | `exercise-runner.md` |
| [0014](0014-run-learner-code-without-sandbox.md) | 学習者のコードをサンドボックスに入れずに実行する | 採用 | `non-functional.md`, `exercise-runner.md` |
| [0015](0015-show-failed-case-details.md) | 判定で失敗したテストケースの入力・期待する出力・実際の出力を表示する | 採用 | `exercise-runner.md` |
| [0016](0016-do-not-install-rust.md) | Rust ツールチェーンを同梱・自動導入せず、導入手順の案内だけを表示する | 採用 | `architecture.md` |
| [0017](0017-search-default-tool-locations.md) | 外部ツールを PATH 以外の既定のインストール先からも探す | 採用 | `architecture.md` |
| [0018](0018-no-lesson-order-lock.md) | レッスンの受講順を制限せず、推奨レッスンを示す | 採用 | `lesson-flow.md` |
| [0019](0019-autosave-drafts.md) | 編集中のコードを自動保存し、正解した後は初期コードから始める | 採用 | `lesson-flow.md` |
| [0020](0020-ai-returns-output-app-writes.md) | AI にツールを使わせず、標準出力で返させた結果をアプリが検証してから保存する | 採用 | `claude-integration.md`, `learning-record-and-analysis.md` |
| [0021](0021-isolated-claude-invocation.md) | `claude -p` を空の一時ディレクトリで、学習者のユーザー設定を読み込まずに呼び出す | 採用 | `claude-integration.md` |
| [0022](0022-learning-works-without-ai.md) | AI 機能が使えなくても学習を続けられるようにする | 採用 | `claude-integration.md`, `architecture.md` |
| [0023](0023-model-per-purpose.md) | Claude のモデルを用途ごとに選べるようにする | 採用 | `claude-integration.md` |
| [0024](0024-run-jobs-serially.md) | AI のジョブを 1 件ずつ直列に実行する | 採用 | `claude-integration.md` |
| [0025](0025-chat-history-only-while-open.md) | 質疑応答の会話履歴を、画面を開いている間だけ保持する | 採用 | `qa-chat.md` |
| [0026](0026-separate-facts-from-interpretation.md) | 学習データの事実はアプリが計算し、AI には解釈だけを任せる | 採用 | `learning-record-and-analysis.md` |
| [0027](0027-markdown-with-frontmatter.md) | 学習記録と分析ファイルを Markdown + YAML frontmatter で保存する | 採用 | `learning-record-and-analysis.md` |
| [0028](0028-separate-record-and-analysis-sessions.md) | 学習記録と分析を、別々の AI セッションで行う | 採用 | `learning-record-and-analysis.md` |
| [0029](0029-keep-full-attempt-logs.md) | 試行ログにコード全体のスナップショットを記録し、学習記録の生成後も保持する | 採用 | `learning-record-and-analysis.md` |
| [0030](0030-incremental-analysis-update.md) | 分析ファイルを「現在の分析 + 新しい記録 + 集計値」で差分的に更新する | 採用 | `learning-record-and-analysis.md` |
| [0031](0031-app-computes-base-level.md) | 習熟レベルの基準をアプリが計算し、AI は ±1 の範囲でのみ調整する | 採用 | `learning-record-and-analysis.md` |
| [0032](0032-reuse-stumble-pattern-labels.md) | 躓きの pattern ラベルを、既存のラベル一覧から再利用させる | 採用 | `learning-record-and-analysis.md` |
| [0033](0033-generate-in-background-with-priority.md) | 演習問題の生成をバックグラウンドのジョブで行い、他のジョブより優先する | 採用 | `exercise-generation.md`, `claude-integration.md` |
| [0034](0034-validate-generated-exercises.md) | 生成した問題を自動で検証し、不合格なら修正を 2 回まで依頼する | 採用 | `exercise-generation.md` |
| [0035](0035-single-difficulty-scale.md) | 問題生成の難易度と複雑さを、1 つの 5 段階の「難易度」にまとめる | 採用 | `exercise-generation.md` |
| [0036](0036-keep-generated-on-history-reset.md) | 学習履歴をすべてリセットしても、生成した問題は残す | 採用 | `screens.md` |
| [0037](0037-bundle-content-and-auto-update.md) | 教材をアプリに同梱し、GitHub Releases と Tauri の自動更新でアプリごと配布する | 採用 | `non-functional.md` |
| [0038](0038-no-os-code-signing-for-now.md) | OS のコード署名を当面行わない | 採用 | `non-functional.md` |
| [0039](0039-ui-visual-direction-chapter-colors.md) | 画面のビジュアルデザインを、開いている章の色をアクセントにする方向性で進める | 採用 | `screens.md` |
| [0040](0040-twelve-chapter-colors.md) | 章カラーを 12 色とし、白い文字で 4.5:1 以上の明るさにそろえ、13 章目からは循環させる | 採用 | `screens.md` |
| [0041](0041-success-in-chapter-color.md) | 正解・完了を章カラーで表し、成功専用の色を持たない | 採用 | `screens.md` |
| [0042](0042-ink-accent-outside-chapters.md) | 章に属さない画面のアクセントを墨色にする | 採用 | `screens.md` |
| [0043](0043-lucide-icons.md) | アイコンに Lucide を使う | 採用 | `screens.md` |
| [0044](0044-design-system-as-source-of-visuals.md) | 画面の見た目の値はデザインシステムを唯一の定義とし、仕様書には書かない | 採用 | `screens.md` |
| [0045](0045-judge-result-above-problem-collapsed.md) | 判定結果をコンソールから分けて問題文の上に表示し、不正解の詳細は既定で畳む | 採用 | `screens.md`, `exercise-runner.md` |
| [0046](0046-explanation-in-pass-modal.md) | 正解したら、解説・質疑応答・次へ進むボタンをまとめたモーダルを開く | 採用 | `screens.md`, `lesson-flow.md`, `qa-chat.md` |
