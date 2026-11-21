# Progrust の概要

* [ADR 0060](0060-switch-lessons-from-slides-to-videos.md) - レッスンの教材を、スライドからナレーションつきの動画に切り替える

# アーキテクチャ

* [ADR 0003](0003-react-typescript-frontend.md) - フロントエンドに React と TypeScript を使う
* [ADR 0004](0004-monaco-without-completion.md) - コードエディタに Monaco Editor を使い、補完・LSP 連携は行わない
* [ADR 0016](0016-do-not-install-rust.md) - Rust ツールチェーンを同梱・自動導入せず、導入手順の案内だけを表示する
* [ADR 0017](0017-search-default-tool-locations.md) - 外部ツールを PATH 以外の既定のインストール先からも探す
* [ADR 0084](0084-keep-record-templates-as-assets.md) - 学習記録・分析のテンプレートを、`.md.tmpl` の asset としてバンドルに置く

# 教材データ形式

* [ADR 0006](0006-element-ids-are-permanent.md) - 要素 ID を全データで共通にし、公開後は変更・削除しない
* [ADR 0007](0007-derive-introduced-lesson.md) - 要素の導入レッスンを、要素マスタに持たせず学習順から求める
* [ADR 0008](0008-judge-mode-per-lesson.md) - 演習の判定方式を、標準出力の比較と隠しテストからレッスンごとに選ぶ
* [ADR 0048](0048-slide-colors-follow-chapter-color.md) - スライドの主役・補助の色を、開いている章の章カラーに合わせる（ADR 0076 で動画に引き継いだ）
* [ADR 0049](0049-slide-accent-light-chapter-color.md) - スライドの補助の色を、同じ章カラーの淡い段にする（ADR 0076 で動画に引き継いだ）
* [ADR 0050](0050-semantic-colors-fixed-in-slides.md) - スライドでは、意味と色が結びついた要素を章によらない固定の色にする（ADR 0076 で動画に引き継いだ）
* [ADR 0051](0051-slides-outside-chapter-use-chapter-4-color.md) - 章に属さずに表示するスライドの主役の色を、第4章の章カラーにする（ADR 0076 で動画に引き継いだ）
* [ADR 0053](0053-mark-and-wavy-error-in-code.md) - コードの強調とエラーの箇所を、行全体ではなく字句単位で示す（ADR 0076 で動画に引き継いだ）
* [ADR 0054](0054-diff-with-line-prefix-and-fixed-colors.md) - コードの Diff を、行の先頭の記号で書き、章によらない固定の色で示す（ADR 0076 で動画に引き継いだ）
* [ADR 0055](0055-mark-fixed-yellow.md) - マーカー（span.mark）を、章によらない固定の黄にする（ADR 0076 で動画に引き継いだ）
* [ADR 0056](0056-show-line-numbers-in-code-blocks.md) - コードブロックに行番号を表示し、文で行番号を指してよいことにする（ADR 0076 で動画に引き継いだ）
* [ADR 0057](0057-scopes-on-code-and-no-code-in-diagrams.md) - 変数が生きている範囲はコードに重ねた枠で示し、図の中にコードを書かない（ADR 0076 で動画に引き継いだ）
* [ADR 0061](0061-render-videos-with-remotion-player.md) - 動画を、書き出したファイルにせず、アプリの中で Remotion の Player で描画する
* [ADR 0062](0062-write-video-scenes-in-unrestricted-tsx.md) - 動画のシーンを、制限のない React（TSX）で書く
* [ADR 0063](0063-generate-narration-with-voicepeak-and-bundle.md) - ナレーションの音声を教材の作者の環境で VOICEPEAK から作り、圧縮してアプリに同梱する
* [ADR 0064](0064-narrator-natsuki-karin-with-shared-voice-settings.md) - ナレーターを夏色花梨とし、声の設定を 1 か所に持って後から変えられるようにする
* [ADR 0065](0065-separate-text-and-say-in-script.md) - 台本で、もとの文（text）と読ませる文（say）を分ける
* [ADR 0066](0066-trim-comma-pauses-to-0-2-seconds.md) - 「、」の間を、音声を作ったあとに 0.2 秒へ切り詰める
* [ADR 0068](0068-compose-at-1280x720-and-zoom-per-scene.md) - 動画を基準サイズ 1280×720 で組み、場面ごとに内容のある範囲へ寄せて見せる
* [ADR 0076](0076-carry-over-slide-visual-and-writing-decisions.md) - スライドで決めた見た目と書き方の判断を、動画でも引き継ぐ
* [ADR 0092](0092-play-narration-with-html5audio.md) - 文の音声を、AAC の .m4a・1 文 1 ファイルのまま、Remotion の Html5Audio で鳴らす
* [ADR 0093](0093-scope-lesson-css-with-build-time-prefix.md) - レッスン専用の CSS を、ビルド時に全セレクタの前へレッスンの属性セレクタを付けて、そのレッスンの動画の中に閉じ込める
* [ADR 0094](0094-close-lint-bypasses-in-video-code.md) - 動画のコードの lint に、検査をすり抜ける書き方と CSS の @import の禁止を加え、lint が検出する範囲を検証で試した範囲に定める
* [ADR 0096](0096-do-not-use-staticfile-in-video-code.md) - 動画のコードで Remotion の staticFile() を使わず、教材チェックツールの lint でエラーにする

# カリキュラム

* [ADR 0077](0077-curriculum-uses-trpl-as-coverage-guide.md) - カリキュラムを、TRPL を網羅の目安にして独自の章立てで組み立てる
* [ADR 0078](0078-curriculum-spec-as-source-of-chapters-json.md) - カリキュラムの唯一の定義を `curriculum.md` とし、`chapters.json` をそのデータ形式とする

# 教材作成ガイドライン

* [ADR 0001](0001-learners-solve-on-their-own.md) - 学習者に演習を自力で解かせることを、機能を設計するうえでの原則とする
* [ADR 0002](0002-target-experienced-programmers.md) - 想定する学習者を他のプログラミング言語の経験者とする
* [ADR 0058](0058-do-not-explain-errors-by-code-only.md) - エラーをエラーコードだけで説明しない（ADR 0076 で動画に引き継いだ）
* [ADR 0059](0059-plain-style-in-slide-headings.md) - スライドの見出し・箇条書き・図の語句に、常体と体言止めを許す（ADR 0076 で動画に引き継いだ）
* [ADR 0069](0069-video-and-sentence-length-as-guidelines.md) - 動画の長さと 1 文の長さを目安とし、超えてよいことにする
* [ADR 0095](0095-render-check-all-lessons-in-ci.md) - CI で、描画のチェックを毎回すべてのレッスンに行い、描画のチェックで見つかるのは描画したフレームのエラーだけだと仕様に書く

# コード実行・判定

* [ADR 0009](0009-embed-hidden-tests-as-module.md) - 隠しテストを、対象ファイルの末尾に子モジュールとして組み込む
* [ADR 0010](0010-allow-listed-crates.md) - 演習で許可リストの外部クレートを使えるようにする
* [ADR 0011](0011-prepare-crates-on-first-launch.md) - 許可クレートを初回起動時に一括でビルドし、ロックファイルを同梱して全演習で共有する
* [ADR 0012](0012-separate-build-dir-without-crates.md) - ビルド先を、許可クレートを使う演習と使わない演習で分ける
* [ADR 0013](0013-interactive-stdin.md) - 実行時の標準入力を、コンソールから対話的に入力する方式にする
* [ADR 0015](0015-show-failed-case-details.md) - 判定で失敗したテストケースの入力・期待する出力・実際の出力を表示する

# レッスン進行・進捗管理

* [ADR 0018](0018-no-lesson-order-lock.md) - レッスンの受講順を制限せず、推奨レッスンを示す
* [ADR 0019](0019-autosave-drafts.md) - 編集中のコードを自動保存し、正解した後は初期コードから始める
* [ADR 0071](0071-save-playback-position-by-sentence.md) - 動画の再生位置を文の単位で保存し、開き直したら続きから見られるようにする

# Claude Code 連携

* [ADR 0020](0020-ai-returns-output-app-writes.md) - AI にツールを使わせず、標準出力で返させた結果をアプリが検証してから保存する
* [ADR 0021](0021-isolated-claude-invocation.md) - `claude -p` を空の一時ディレクトリで、学習者のユーザー設定を読み込まずに呼び出す
* [ADR 0022](0022-learning-works-without-ai.md) - AI 機能が使えなくても学習を続けられるようにする
* [ADR 0023](0023-model-per-purpose.md) - Claude のモデルを用途ごとに選べるようにする
* [ADR 0024](0024-run-jobs-serially.md) - AI のジョブを 1 件ずつ直列に実行する

# 質疑応答

* [ADR 0025](0025-chat-history-only-while-open.md) - 質疑応答の会話履歴を、画面を開いている間だけ保持する
* [ADR 0072](0072-do-not-pause-video-on-question.md) - 質問を送信しても、動画を一時停止しない

# 学習記録・分析

* [ADR 0026](0026-separate-facts-from-interpretation.md) - 学習データの事実はアプリが計算し、AI には解釈だけを任せる
* [ADR 0027](0027-markdown-with-frontmatter.md) - 学習記録と分析ファイルを Markdown + YAML frontmatter で保存する
* [ADR 0028](0028-separate-record-and-analysis-sessions.md) - 学習記録と分析を、別々の AI セッションで行う
* [ADR 0029](0029-keep-full-attempt-logs.md) - 試行ログにコード全体のスナップショットを記録し、学習記録の生成後も保持する
* [ADR 0030](0030-incremental-analysis-update.md) - 分析ファイルを「現在の分析 + 新しい記録 + 集計値」で差分的に更新する
* [ADR 0031](0031-app-computes-base-level.md) - 習熟レベルの基準をアプリが計算し、AI は ±1 の範囲でのみ調整する
* [ADR 0032](0032-reuse-stumble-pattern-labels.md) - 躓きの pattern ラベルを、既存のラベル一覧から再利用させる
* [ADR 0074](0074-do-not-log-video-viewing.md) - 動画の視聴を試行ログに記録しない

# 演習問題生成

* [ADR 0033](0033-generate-in-background-with-priority.md) - 演習問題の生成をバックグラウンドのジョブで行い、他のジョブより優先する
* [ADR 0034](0034-validate-generated-exercises.md) - 生成した問題を自動で検証し、不合格なら修正を 2 回まで依頼する
* [ADR 0035](0035-single-difficulty-scale.md) - 問題生成の難易度と複雑さを、1 つの 5 段階の「難易度」にまとめる

# 画面仕様

* [ADR 0036](0036-keep-generated-on-history-reset.md) - 学習履歴をすべてリセットしても、生成した問題は残す
* [ADR 0039](0039-ui-visual-direction-chapter-colors.md) - 画面のビジュアルデザインを、開いている章の色をアクセントにする方向性で進める
* [ADR 0040](0040-twelve-chapter-colors.md) - 章カラーを 12 色とし、白い文字で 4.5:1 以上の明るさにそろえ、13 章目からは循環させる
* [ADR 0041](0041-success-in-chapter-color.md) - 正解・完了を章カラーで表し、成功専用の色を持たない
* [ADR 0042](0042-ink-accent-outside-chapters.md) - 章に属さない画面のアクセントを墨色にする
* [ADR 0043](0043-lucide-icons.md) - アイコンに Lucide を使う
* [ADR 0044](0044-design-system-as-source-of-visuals.md) - 画面の見た目の値はデザインシステムを唯一の定義とし、仕様書には書かない
* [ADR 0045](0045-judge-result-above-problem-collapsed.md) - 判定結果をコンソールから分けて問題文の上に表示し、不正解の詳細は既定で畳む
* [ADR 0046](0046-explanation-in-pass-modal.md) - 正解したら、解説・質疑応答・次へ進むボタンをまとめたモーダルを開く
* [ADR 0052](0052-chapter-8-sky-blue-no-gray.md) - 第8章の章カラーを鉄灰から空色に変え、章カラーに灰色を使わない
* [ADR 0067](0067-captions-as-app-feature.md) - 字幕を動画の中に置かず、アプリの機能として表示できるようにする
* [ADR 0070](0070-video-controls-sentence-jump-and-5s-skip.md) - 動画の再生操作に、文ごとの移動と 5 秒のスキップを持たせる
* [ADR 0073](0073-search-lessons-by-tags-and-title.md) - 検索の対象を、レッスンに設定した検索ワード（タグ）とタイトルにする
* [ADR 0075](0075-video-design-system-progrust-video.md) - 動画の見た目を、スライドのデザインシステムを改めた「Progrust Video」に定める

# 非機能要件

* [ADR 0014](0014-run-learner-code-without-sandbox.md) - 学習者のコードをサンドボックスに入れずに実行する
* [ADR 0090](0090-support-only-authors-mac-until-completion.md) - アプリの完成までは、対応環境を作者の Mac だけとし、一般公開しない前提で開発する
* [ADR 0091](0091-no-distribution-or-auto-update-until-publication.md) - アプリの完成までは配布と更新の仕組みを作らず、作者の Mac でビルドして使い、教材はアプリに同梱する

# ドキュメントの運用ルール

* [ADR 0079](0079-manage-documents-as-okf-bundle.md) - ドキュメントを `documents/` に集約し、OKF v0.3 のバンドルとして AI エージェントが管理する
* [ADR 0080](0080-use-minimal-okf-frontmatter.md) - OKF の frontmatter のうち type・title・description・status・superseded_by だけを使い、type を6つに固定する
* [ADR 0081](0081-split-specs-into-one-topic-per-file.md) - 仕様を1トピック1ファイルに分け、受け入れ条件と未決事項は機能ごとに1ファイルにまとめる
* [ADR 0082](0082-reference-by-links-without-section-numbers.md) - 文書間の参照を Markdown リンクで書き、節番号での参照を廃止する
* [ADR 0083](0083-express-adr-status-in-okf-frontmatter.md) - ADR の状態を OKF の status と superseded_by で表し、番号・ファイル名・本文は変えない
* [ADR 0085](0085-check-documents-with-own-checker.md) - ドキュメントを独自の検査スクリプトで検査して hook で自動実行し、OKF の公式 validator は手動で実行する
* [ADR 0086](0086-list-all-affected-specs-and-find-adrs-by-path-search.md) - ADR の関連仕様に、判断が内容を決めている仕様をすべて載せ、仕様に関係する ADR を関連仕様の行の検索で探す
* [ADR 0087](0087-note-carry-over-in-adr-description.md) - 後の ADR が別の対象に引き継いだ ADR の description の末尾に、引き継いだ ADR と対象を示す注記を足す
* [ADR 0088](0088-agents-file-found-problems-as-issues.md) - AI エージェントが作業中に見つけた問題を、原因の調査や改善案なしに、確認せずに GitHub の Issue に起票し、その決まりを運用ルールに置く
* [ADR 0089](0089-keep-verification-reports-in-documents.md) - 技術検証の結果を `documents/verification/` に1つの Issue につき1文書で残し、そのための type `Verification Report` を加える

# 置き換え済み・廃止

* [ADR 0005](0005-slides-in-restricted-html.md) - スライドを、使える部品を決めた HTML で書く
* [ADR 0037](0037-bundle-content-and-auto-update.md) - 教材をアプリに同梱し、GitHub Releases と Tauri の自動更新でアプリごと配布する
* [ADR 0038](0038-no-os-code-signing-for-now.md) - OS のコード署名を当面行わない
* [ADR 0047](0047-separate-slide-design-system.md) - スライドの見た目を、画面とは別のデザインシステム「Progrust Slides」に定める
