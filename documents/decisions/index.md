# Progrust の概要

* [ADR 0060](0060-switch-lessons-from-slides-to-videos.md) - レッスンの教材を、スライドからナレーションつきの動画に切り替える

# アーキテクチャ

* [ADR 0003](0003-react-typescript-frontend.md) - フロントエンドに React と TypeScript を使う
* [ADR 0004](0004-monaco-without-completion.md) - コードエディタに Monaco Editor を使い、補完・LSP 連携は行わない
* [ADR 0016](0016-do-not-install-rust.md) - Rust ツールチェーンを同梱・自動導入せず、導入手順の案内だけを表示する
* [ADR 0017](0017-search-default-tool-locations.md) - 外部ツールを PATH 以外の既定のインストール先からも探す
* [ADR 0084](0084-keep-record-templates-as-assets.md) - 学習記録・分析のテンプレートを、`.md.tmpl` の asset としてバンドルに置く
* [ADR 0111](0111-bundle-monaco-and-set-csp.md) - Monaco Editor を CDN を使わずに同梱し、アプリに CSP を設定して読み込み先を自身と Tauri の IPC に限り（Monaco のため style-src に 'unsafe-inline' を許す）、演習画面のショートカットを Monaco のコマンドとキャプチャの keydown で受ける
* [ADR 0117](0117-raise-min-rustc-only-when-author-decides.md) - rustc の最低バージョンはアプリのリリースに合わせて上げず、作者が決めたときに上げる。バージョンが足りないときは rustup update stable を案内する
* [ADR 0128](0128-serve-narration-from-resources-via-asset-protocol.md) - 動画の音声をフロントエンドのビルドに含めず、アプリ同梱データに置いて、Tauri のアセットプロトコル（asset://）で読む
* [ADR 0147](0147-define-command-and-event-payloads-in-architecture.md) - フロントエンドとバックエンドの間のコマンドとイベントの入力・出力の形を `spec/architecture/commands/` の 1 か所に定め、要求をフロントエンドが作る `request_id` で見分け、エラーを共通の形で返す（ADR 0167 でフロントエンドに許可する権限にイベントの受け取りを加えた）
* [ADR 0148](0148-pin-dependencies-to-verified-versions-and-app-identifier.md) - 依存の版を技術検証で動いた系列にそろえて仕様に書き、細かい版はロックファイルを正とし、パッケージマネージャーを npm に、アプリの識別子を com.progrust.desktop（開発時の起動は com.progrust.desktop.dev）にする
* [ADR 0166](0166-choose-unlisted-deps-that-support-node-series.md) - 表にない依存は、Node.js の系列（25.9 系）を `engines` が含み、表の依存の版と peer 依存が合う版を入れ、そのために Vitest を 4 系、jsdom を 29 系、`@testing-library/jest-dom` を 6 系、`stylelint-config-standard` を 40 系にする
* [ADR 0167](0167-allow-listening-to-backend-events.md) - フロントエンドに許可する Tauri の権限に、バックエンドが送るイベントの受け取り（listen・unlisten）を加え、それ以外の組み込みの API（フロントエンドからのイベントの送信を含む）は許可せず、本アプリのコマンドは権限の一覧に書かずに呼べるままにする
* [ADR 0185](0185-name-temp-and-broken-files-and-define-unreadable.md) - 学習者データの一時ファイルを同じディレクトリの `.{ファイル名}.{プロセス ID}.tmp` にし、退避の `{日時}` を現地時刻の `YYYYMMDDTHHMMSS`（重なるときは `-2`…）にし、読み込めない場合を JSON・形・`schema_version` の誤りに限り、読み込みの失敗とファイルが無い場合は退避しない
* [ADR 0187](0187-stage-bundled-content-in-build-script.md) - 同梱する教材を、ビルドスクリプトがリポジトリの content/ から選んで src-tauri/gen/bundled/content/ に写し、Tauri の bundle.resources はそのディレクトリだけを {app_resources}/content/ に入れる
* [ADR 0199](0199-put-react-in-one-prebundled-chunk-for-dev.md) - 開発時の起動で Vite が前もってまとめる依存（optimizeDeps）の分け方を指定し、react・react-dom・scheduler を 1 つの塊に入れる
* [ADR 0200](0200-type-check-lesson-video-code-in-lint.md) - TypeScript の型検査（`npm run lint` の `tsc --noEmit -p tsconfig.json`）の対象に、教材の動画のコード（`content/lessons/*/video/` の `.ts`・`.tsx`）を加える
* [ADR 0207](0207-schema-version-for-drafts-and-attempt-logs.md) - 下書き（`drafts/*.json`）と試行ログ（`attempts/*.json`）に `schema_version`（いまは 1）を持たせ、読み込めないときは `.broken-{日時}` を付けて退避し、作り直さずに無いものとして扱う
* [ADR 0209](0209-regenerate-runner-lock-with-npm-script.md) - runner/Cargo.lock は、一時ディレクトリに allcrates/ と同じ形のライブラリを作って cargo generate-lockfile で生成する npm のスクリプト（npm run runner:lock）で作り直し、runner/ の 2 つのファイルは、教材と同じ仕組みで src-tauri/gen/bundled/runner/ に写して同梱する
* [ADR 0210](0210-find-rustc-next-to-cargo-and-use-first-usable-place.md) - cargo と rustc は探索順の場所ごとに同じディレクトリの組で調べ、設定のパスは cargo の実行ファイルの絶対パスとし、使えない場所は飛ばして、rustc --version が成功した最初の場所を、バージョンが足りなくても使う
* [ADR 0211](0211-draft-commands-reject-lessons-without-exercise.md) - 演習の無いレッスン（動画のみのレッスン）に `save_draft`・`reset_draft` を呼んだときは、下書きを書かずに `invalid_argument` を返す

# 教材データ形式

* [ADR 0006](0006-element-ids-are-permanent.md) - 要素 ID を全データで共通にし、公開後は変更・削除しない
* [ADR 0007](0007-derive-introduced-lesson.md) - 要素の導入レッスンを、要素マスタに持たせず学習順から求める（ADR 0180 で導入レッスンを求める元を章とレッスンの一覧の「主要素」に変えた）
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
* [ADR 0120](0120-video-only-lessons-have-no-elements-or-attempt-sessions.md) - 動画のみのレッスンには要素を持たせず、試行セッションも作らない
* [ADR 0127](0127-encode-narration-with-ffmpeg-aac-and-measure-with-ffprobe.md) - 文の音声を ffmpeg に組み込みの AAC エンコーダーで .m4a に変換し、durations.json の seconds と cue-reference.json の長さを ffprobe の format の duration で測る
* [ADR 0145](0145-show-unprepared-lessons-as-in-preparation.md) - `chapters.json` には全レッスンを載せたまま、レッスンのディレクトリがまだ無いレッスンを準備中のレッスンとし、レッスン一覧に「準備中」と出して開けないようにする
* [ADR 0158](0158-chapter-review-has-two-to-five-primary-elements.md) - 章末復習レッスンの主要素を、その章のレッスンの主要素から選ぶ 2〜5 件とし、ほかのレッスンは 1〜2 件のままにする
* [ADR 0174](0174-number-chapters-and-lessons-by-position.md) - 画面と動画に出す章とレッスンの番号を `chapters.json` の並び順から決め、章カラーも章の番号で決める
* [ADR 0176](0176-pass-theme-and-chapter-to-video-by-ancestor-attributes.md) - 再生するときのテーマと章カラーを、動画を描く要素の祖先の `data-theme`・`data-chapter` と CSS 変数の継承で動画に渡し、`useLesson()` の材料になる章とレッスン定義は Player の `inputProps` で渡す
* [ADR 0178](0178-do-not-use-player-built-in-controls.md) - Remotion の Player の組み込みの操作（操作の表示、Space キー、クリック、ダブルクリックでの全画面）を使わず、末尾で先頭に戻さず繰り返さない設定にして、再生の操作をすべてアプリの部品で行う
* [ADR 0180](0180-derive-introducing-lesson-from-curriculum-list.md) - 要素の導入レッスンを、各レッスンの `lesson.json` ではなく、章とレッスンの一覧の「主要素」から求め、準備中のレッスンが導入する要素も決まるようにする
* [ADR 0186](0186-round-cue-offsets-to-whole-frames.md) - 合図の関数 `at`・`endOf` が返す位置を整数のフレーム数にし、指定した秒をフレーム数に四捨五入してから足す
* [ADR 0188](0188-write-rust-highlighter-by-hand.md) - 動画のコードの字句の色分けを、ライブラリを使わず、Rust の字句を分ける自作の走査で行う
* [ADR 0190](0190-ignore-lint-disable-comments-in-video-code.md) - 動画のコードの lint では、コードの中の lint を止めるための指定（eslint-disable・stylelint-disable などのコメント）に従わない
* [ADR 0198](0198-find-comma-pauses-by-matching-silences-to-punctuation.md) - 「、」にあたる無音を、−50dBFS より小さい音が 0.2 秒以上続く無音と、say の文の途中の句読点を前から順に対応づけて見つけ、数が合わない文はエラーにする

# カリキュラム

* [ADR 0077](0077-curriculum-uses-trpl-as-coverage-guide.md) - カリキュラムを、TRPL を網羅の目安にして独自の章立てで組み立てる（ADR 0131 で個々のレッスンの理由を記録しない範囲を変えた）
* [ADR 0078](0078-curriculum-spec-as-source-of-chapters-json.md) - カリキュラムの唯一の定義を `curriculum.md` とし、`chapters.json` をそのデータ形式とする
* [ADR 0104](0104-teach-flush-after-print.md) - print! で出したプロンプトを入力の前に表示するための flush を、ch09-05 で教える
* [ADR 0118](0118-teach-background-knowledge-hidden-by-gc-languages.md) - ガベージコレクションのある言語では意識しなくてよいが Rust では必要になる周辺知識を既知とせず、TRPL が扱っていないものもカリキュラムで扱う
* [ADR 0131](0131-record-reasons-for-later-lesson-changes.md) - カリキュラムを組み立てた後に、個々のレッスンを足す・移す・残すなどの変更をしたときは、その理由を ADR に記録する
* [ADR 0157](0157-define-primary-elements-in-curriculum-list.md) - 各レッスンの主要素を章とレッスンの一覧に定めて唯一の定義とし、`lesson.json` の `primary_elements` をそれに合わせ、副要素はレッスンを作るときに決める

# 教材作成ガイドライン

* [ADR 0001](0001-learners-solve-on-their-own.md) - 学習者に演習を自力で解かせることを、機能を設計するうえでの原則とする
* [ADR 0002](0002-target-experienced-programmers.md) - 想定する学習者を他のプログラミング言語の経験者とする（ADR 0118 で既知とする概念の範囲を変えた）
* [ADR 0058](0058-do-not-explain-errors-by-code-only.md) - エラーをエラーコードだけで説明しない（ADR 0076 で動画に引き継いだ）
* [ADR 0059](0059-plain-style-in-slide-headings.md) - スライドの見出し・箇条書き・図の語句に、常体と体言止めを許す（ADR 0076 で動画に引き継いだ）
* [ADR 0069](0069-video-and-sentence-length-as-guidelines.md) - 動画の長さと 1 文の長さを目安とし、超えてよいことにする
* [ADR 0095](0095-render-check-all-lessons-in-ci.md) - CI で、描画のチェックを毎回すべてのレッスンに行い、描画のチェックで見つかるのは描画したフレームのエラーだけだと仕様に書く（ADR 0145 で準備中のレッスンを描画の対象から外した）
* [ADR 0146](0146-one-test-case-allowed-in-ch01-and-ch02.md) - ch01・ch02 のレッスン（章末復習レッスンを含む）の演習は、テストケースを 1 件以上でよいとし、ほかのレッスンは 3 件以上のままにする
* [ADR 0156](0156-define-all-elements-before-lessons-one-per-lesson.md) - 要素マスタの要素をレッスンを作る前にすべて決め、1 レッスン 1 要素を基本にして 145 件を定め、要素 ID の分類を Rust の機能のまとまりで付ける
* [ADR 0191](0191-count-lesson-title-on-rendered-frames.md) - 教材チェックツールの検査「表紙」で、最初のシーンの描画したフレームごとに、表示している `LessonTitle` を、`LessonTitle` 自身が知らせる数で数え、どのフレームでも表示していないときと、2 つ以上表示しているフレームがあるときをエラーにする
* [ADR 0192](0192-report-browser-console-errors-in-render-check.md) - 教材チェックツールの描画のチェックで、描画時の例外に加えて、ブラウザの console に出たエラーもエラーにし、console の警告はエラーにしない
* [ADR 0208](0208-content-check-runs-judge-via-backend-cli.md) - 教材チェックツールの「模範解答」「初期コード」の判定を、バックエンドのクレートに置いた判定だけを行うコマンドラインのプログラム（`src-tauri/src/bin/`）を `cargo run` で起動して行い、作業領域を `src-tauri/target/` の下の専用のディレクトリにする

# コード実行・判定

* [ADR 0009](0009-embed-hidden-tests-as-module.md) - 隠しテストを、対象ファイルの末尾に子モジュールとして組み込む
* [ADR 0010](0010-allow-listed-crates.md) - 演習で許可リストの外部クレートを使えるようにする
* [ADR 0011](0011-prepare-crates-on-first-launch.md) - 許可クレートを初回起動時に一括でビルドし、ロックファイルを同梱して全演習で共有する
* [ADR 0012](0012-separate-build-dir-without-crates.md) - ビルド先を、許可クレートを使う演習と使わない演習で分ける
* [ADR 0013](0013-interactive-stdin.md) - 実行時の標準入力を、コンソールから対話的に入力する方式にする
* [ADR 0015](0015-show-failed-case-details.md) - 判定で失敗したテストケースの入力・期待する出力・実際の出力を表示する
* [ADR 0097](0097-share-crate-builds-via-allcrates-library.md) - 全許可クレートに依存する空のライブラリへの path 依存を、初回準備と演習の両方のプロジェクトに足し、許可クレートの組み合わせによらずビルド結果を共有する（ADR 0132 で path 依存するプロジェクトの範囲を変えた）
* [ADR 0098](0098-separate-build-dir-for-validation.md) - 生成問題の検証のビルド先を学習者のものと分け、許可クレートのビルド結果は初回準備の後に APFS のクローンで複製する
* [ADR 0099](0099-allow-package-cache-wait-during-preparation.md) - 初回準備がクレートをダウンロードしている間に、許可クレートを使わない演習が cargo のパッケージキャッシュのロックで待つことを許し、アプリ専用の CARGO_HOME は使わない
* [ADR 0100](0100-redo-preparation-when-rustc-changes.md) - 初回準備の状態に rustc の版を記録し、起動時に rustc の版が変わっていたら初回準備をやり直す
* [ADR 0101](0101-run-each-hidden-test-in-own-process.md) - test 方式の隠しテストを、テストごとに別のプロセスで実行し、終了コードと結果の行で判定する
* [ADR 0102](0102-stop-learner-process-group-and-batch-output.md) - 学習者のプログラムをプロセスグループごとに止め、実行中の出力を一定時間ごとにまとめて画面に送る
* [ADR 0105](0105-detect-lost-crates-from-offline-build-errors.md) - 演習の --offline のビルドの失敗の文から、許可クレートのソースやビルド結果が失われたことを判定し、初回準備を自動でやり直す。cargo の自動削除への対策として、アプリ専用の CARGO_HOME は使わない（ADR 0137 で E0786 を探す出力を変えた）
* [ADR 0112](0112-run-learner-tests-with-test-button.md) - 演習画面に、学習者が書いたテストを cargo test で動かす「テスト」ボタンをどの演習でも置き、cargo の出力をそのまま表示する
* [ADR 0113](0113-judge-learner-tests-by-count-and-pass.md) - 学習者が書いたテストを、演習ごとに指定した件数以上あってすべて通ることで判定し、テストが実装の誤りを見つけられるかまでは判定しない
* [ADR 0114](0114-allow-integration-and-doc-tests.md) - 学習者が tests/ の統合テストを書けるようにし、ドキュメントテストも実行して、ch11-05 と ch11-06 のレッスンを残す
* [ADR 0125](0125-crates-unavailable-status-without-attempt-log.md) - ビルドが初回準備のやり直しが要る失敗になった実行・提出の結果を、新しい状態 `crates_unavailable` とし、試行ログに記録せず、準備の後に自動ではやり直さない
* [ADR 0132](0132-no-allcrates-dependency-without-crates.md) - 許可クレートを使わない演習のプロジェクトには、全許可クレートに依存する空のライブラリへの path 依存を書かない
* [ADR 0134](0134-allow-unused-dependencies-cargo-lint.md) - 許可クレートを使う演習のプロジェクトの Cargo.toml で、cargo の「使っていない依存」の警告（cargo::unused_dependencies）を止める
* [ADR 0135](0135-show-cargo-stderr-with-quiet-build.md) - 実行・テスト実行・提出のビルドの cargo に --quiet を付け、cargo 自身の標準エラー出力を、コンパイラの診断と合わせてそのまま表示する
* [ADR 0137](0137-find-e0786-in-compiler-diagnostics.md) - 初回準備のやり直しの判定で、ビルド済みの依存の破損（E0786）を、cargo 自身の標準エラー出力ではなく、コンパイラの診断から探す
* [ADR 0138](0138-keep-cargo-stderr-in-attempt-log.md) - 試行ログの `stderr` に、ビルドの cargo 自身の標準エラー出力を、コンパイラの診断とプログラムの標準エラー出力の間に入れて保存する
* [ADR 0139](0139-send-build-output-after-build-ends.md) - ビルドの出力を、ビルド中には送らず、ビルドが終わって結果が決まってから送る
* [ADR 0201](0201-no-stop-button-for-submit.md) - 提出は学習者が途中で止められないままにし、提出中は「停止」ボタンを無効にする
* [ADR 0202](0202-stop-run-and-submit-when-leaving-exercise.md) - 実行・テスト実行・提出の途中で演習画面を離れたときとアプリを終了したときは止め、画面を離れて止めた実行は `stopped` として記録し、止めた提出（`stopped`）とアプリの終了で止めたものは記録しない
* [ADR 0203](0203-stdout-case-fails-on-nonzero-exit.md) - `stdout` 方式の判定で、標準出力が期待値と一致しても、終了コード 0 以外かシグナルで終わったテストケースは `runtime_error`（不正解）にし、テストケースの結果を決める順を定める
* [ADR 0205](0205-pick-hidden-test-executable-from-build-json.md) - `test` 方式で隠しテストを入れたテスト実行ファイルを、ビルドの JSON の出力のうち、`judge.target` のファイルを根に持つテスト用の成果物から決める
* [ADR 0214](0214-stdout-case-output-limit-ignores-stderr.md) - `stdout` 方式の 1 テストケースの出力量の上限を標準出力だけで数え、標準エラー出力は `detail` に使う先頭だけを受け取って、残りはプログラムを止めずに捨てる
* [ADR 0215](0215-pick-run-executable-from-build-json.md) - 実行で起動する実行ファイルを、ビルドの JSON の出力のうち、`src/main.rs` を根に持つテスト用でない bin の成果物から決める
* [ADR 0216](0216-stdout-fail-detail-as-object.md) - 判定結果の `cases[].detail` を、`stdout` 方式の `fail` では入力・期待する出力・実際の出力を分けたオブジェクトにし、`pass` では `null`、ほかでは文字列にする
* [ADR 0217](0217-no-hidden-tests-is-app-failure.md) - `test` 方式で、隠しテストの名前を得る起動（`--list`）が失敗したときと、隠しテストが 1 件も得られなかったときは、判定せずにアプリの側の失敗として終える
* [ADR 0218](0218-read-should-panic-test-results.md) - `test` 方式で、名前の後に ` - should panic` などのテストの種類の表示が付いた行も結果の行とし、`#[should_panic]` のテストの失敗では、libtest の説明を `fail` の `detail` に入れる
* [ADR 0219](0219-learner-tests-fail-on-any-failed-line.md) - `test` 方式の学習者のテストの判定で、` ... FAILED` の行が 1 つでもあれば cargo の終了コードが 0 でも `pass` にせず、cargo を `CARGO_TERM_COLOR=never` を付けて起動する
* [ADR 0220](0220-compile-error-submit-has-no-cases.md) - `compile_error` の提出の判定結果は、`passed` と `total` を 0、`cases[]` を空にする
* [ADR 0221](0221-submit-stderr-from-failed-case-details.md) - 試行ログの `submit` イベントの `stderr` に入れるプログラムの標準エラー出力を、`timeout`・`runtime_error` のテストケースの `detail` を `cases[]` の順に連結したものにする

# レッスン進行・進捗管理

* [ADR 0018](0018-no-lesson-order-lock.md) - レッスンの受講順を制限せず、推奨レッスンを示す（ADR 0145 で準備中のレッスンを開けないことにした）
* [ADR 0019](0019-autosave-drafts.md) - 編集中のコードを自動保存し、正解した後は初期コードから始める
* [ADR 0071](0071-save-playback-position-by-sentence.md) - 動画の再生位置を文の単位で保存し、開き直したら続きから見られるようにする（ADR 0169 で最後の文まで再生したあとに開き直したときの表示位置を変えた）
* [ADR 0119](0119-video-only-lessons-for-background-knowledge.md) - 周辺知識だけを扱うレッスンを演習のない動画のみのレッスンにし、動画を最後まで再生したら完了とする（ADR 0143 で、動画画面に「レッスン一覧へ」を表示しないことにした）
* [ADR 0169](0169-show-video-from-start-after-watching-to-end.md) - 動画を最後の文まで再生したあとに開き直したときは、最後の文の頭ではなく、動画の先頭から表示する
* [ADR 0170](0170-write-undecided-progress-fields-as-null.md) - `progress.json` で、完了していないレッスンの完了のときに決まるフィールドを、省かずに `null`（`completion_count` は 0）で書く
* [ADR 0213](0213-passed-code-is-first-passed-submit.md) - 「前回の正解コードを見る」では、最後に正解した試行セッションの最初に正解した `submit` イベントを返し、正解した `submit` イベントが無い試行ログでは `not_found` にする

# Claude Code 連携

* [ADR 0020](0020-ai-returns-output-app-writes.md) - AI にツールを使わせず、標準出力で返させた結果をアプリが検証してから保存する
* [ADR 0021](0021-isolated-claude-invocation.md) - `claude -p` を空の一時ディレクトリで、学習者のユーザー設定を読み込まずに呼び出す
* [ADR 0022](0022-learning-works-without-ai.md) - AI 機能が使えなくても学習を続けられるようにする
* [ADR 0023](0023-model-per-purpose.md) - Claude のモデルを用途ごとに選べるようにする
* [ADR 0024](0024-run-jobs-serially.md) - AI のジョブを 1 件ずつ直列に実行する
* [ADR 0106](0106-isolated-claude-flags-and-oauth-only.md) - `claude -p` を検証で確かめたフラグの組み合わせで隔離して呼び、既定のシステムプロンプトを置き換え、認証は claude.ai のログインだけを使う
* [ADR 0107](0107-classify-claude-errors-from-stream-json.md) - ジョブも stream-json で呼び出し、AI 呼び出しのエラーを `is_error` と `assistant` の行の `error` の分類コードで見分ける
* [ADR 0124](0124-pass-only-six-env-vars-to-claude.md) - アプリが `claude` を起動するときは、引き継いだ環境変数をそのまま渡さず、HOME・USER・LOGNAME・PATH・SHELL・TMPDIR の 6 つだけを渡す

# 質疑応答

* [ADR 0025](0025-chat-history-only-while-open.md) - 質疑応答の会話履歴を、画面を開いている間だけ保持する
* [ADR 0072](0072-do-not-pause-video-on-question.md) - 質問を送信しても、動画を一時停止しない

# 学習記録・分析

* [ADR 0026](0026-separate-facts-from-interpretation.md) - 学習データの事実はアプリが計算し、AI には解釈だけを任せる
* [ADR 0027](0027-markdown-with-frontmatter.md) - 学習記録と分析ファイルを Markdown + YAML frontmatter で保存する
* [ADR 0028](0028-separate-record-and-analysis-sessions.md) - 学習記録と分析を、別々の AI セッションで行う
* [ADR 0029](0029-keep-full-attempt-logs.md) - 試行ログにコード全体のスナップショットを記録し、学習記録の生成後も保持する（ADR 0125 で試行ログに記録する実行・提出の範囲を変えた）
* [ADR 0030](0030-incremental-analysis-update.md) - 分析ファイルを「現在の分析 + 新しい記録 + 集計値」で差分的に更新する
* [ADR 0031](0031-app-computes-base-level.md) - 習熟レベルの基準をアプリが計算し、AI は ±1 の範囲でのみ調整する
* [ADR 0032](0032-reuse-stumble-pattern-labels.md) - 躓きの pattern ラベルを、既存のラベル一覧から再利用させる
* [ADR 0074](0074-do-not-log-video-viewing.md) - 動画の視聴を試行ログに記録しない
* [ADR 0108](0108-analysis-outputs-only-ai-fields.md) - 分析セッションの AI にはアプリ管理のフィールドを出力させず、AI 管理のフィールドと本文だけを出力させて、アプリが分析ファイルを組み立てる
* [ADR 0110](0110-record-session-log-diff-base-and-new-pattern-labels.md) - 試行ログの差分の基準を圧縮で残したイベントのうち直前のものとし、新しい種類の躓きの pattern ラベルが呼び出しごとに揺れることは許す
* [ADR 0206](0206-attempt-log-exit-code-same-as-run-finished.md) - 試行ログの `run` の `exit_code` を、`run_finished` の `run.exit_code` と同じ値（学習者のプログラム、テスト実行では `cargo test` の終了コード。起動していないときとシグナルで終わったときは `null`）にする
* [ADR 0212](0212-attempt-session-id-collision-and-append-target.md) - 試行セッションの ID が既にある試行ログと重なるときは ID の日時を 1 秒ずつ進め、未完了のセッションが複数あるときは最後に始めたものを続け、実行・提出のイベントはレッスンの未完了のセッションに追記し（無ければ新しく始める）、画面を離れて止めた実行は追記し終えてからセッションを終了し、`passed_at` は最初に正解した日時を残す

# 演習問題生成

* [ADR 0033](0033-generate-in-background-with-priority.md) - 演習問題の生成をバックグラウンドのジョブで行い、他のジョブより優先する
* [ADR 0034](0034-validate-generated-exercises.md) - 生成した問題を自動で検証し、不合格なら修正を 2 回まで依頼する
* [ADR 0035](0035-single-difficulty-scale.md) - 問題生成の難易度と複雑さを、1 つの 5 段階の「難易度」にまとめる
* [ADR 0109](0109-generated-exercise-layout-and-starter-rules.md) - 生成問題の複数ファイルを ch10-04 の完了後に限り、初期コードがビルドできることを検証に加え、判定方式の制約と修正の依頼の入力を生成の仕様に定める
* [ADR 0126](0126-redo-validation-after-crates-are-prepared-again.md) - 生成問題の検証のビルドが初回準備のやり直しが要る失敗になったときは、検証の失敗とせず、初回準備が終わってから同じ出力の検証をやり直す
* [ADR 0159](0159-validate-generated-primary-element-count.md) - 生成問題の主要素の件数（1〜2 件）を出力の検証で確かめ、外れたら修正を依頼する

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
* [ADR 0070](0070-video-controls-sentence-jump-and-5s-skip.md) - 動画の再生操作に、文ごとの移動と 5 秒のスキップを持たせる（ADR 0142 で文ごとの移動をなくした）
* [ADR 0073](0073-search-lessons-by-tags-and-title.md) - 検索の対象を、レッスンに設定した検索ワード（タグ）とタイトルにする
* [ADR 0075](0075-video-design-system-progrust-video.md) - 動画の見た目を、スライドのデザインシステムを改めた「Progrust Video」に定める
* [ADR 0103](0103-keep-console-output-tail.md) - 演習画面のコンソールに残す出力を末尾の 64K 文字までとし、超えた分は古いほうから消す
* [ADR 0116](0116-bundle-upstream-variable-fonts-unmodified.md) - 書体のファイルを、配布元が配っている可変フォントのまま、変換もサブセット化もせずにアプリに同梱する
* [ADR 0142](0142-remove-sentence-jump-from-video-controls.md) - 動画の再生操作から、前後の文への移動をなくす
* [ADR 0143](0143-no-lesson-list-button-in-video-only-controls.md) - 動画のみのレッスンの動画画面に「レッスン一覧へ」を置かず、「演習へ」の位置には「次のレッスンへ」だけを置く
* [ADR 0144](0144-top-page-shows-nine-chapters.md) - トップページの章ごとの完了率を、次にやるべきレッスンの章を含む 9 章ぶんだけ表示する
* [ADR 0150](0150-screen-layouts-follow-ui-design-until-implemented.md) - 各画面のレイアウトは画面デザイン「Progrust UIデザイン」に従い、画面を実装した後は実装を正とする
* [ADR 0152](0152-keep-design-system-files-in-repository.md) - デザインシステム「Progrust」「Progrust Video」のファイルをリポジトリの `design-system/` に置いてリポジトリのファイルを正とし、claude.ai のアーティファクトは読み取り専用の表示用として残し、変えるときはリポジトリを直してからアーティファクトへ送り、画面デザインの中の写しを取り直して、中身を確かめる（ADR 0181 で画面デザインの置き場所を、ADR 0196 で表示用のアーティファクトを残して送ることを変えた）
* [ADR 0153](0153-load-design-system-css-directly-from-repository.md) - アプリは、リポジトリの `design-system/` にあるデザインシステムの CSS とアイコンを、写しを作らずに直接読み込み、書体を外部から読み込む `@import` の行はビルドのときに取り除き、`design-system/` を lint と整形の対象から外す（ADR 0197 で `@import` の行をデザインシステムのファイルから消した）
* [ADR 0154](0154-check-spec-references-in-design-systems.md) - デザインシステムの検査で、README などからインラインコードのパスで書いた仕様とデザインシステムの文書への参照が、実在する文書と小見出しを指すことを確かめる
* [ADR 0155](0155-check-video-design-system-copied-tokens.md) - デザインシステムの検査で、「Progrust Video」の `video-` で始まらないトークンと書体の族が、「Progrust」と同じ名前・同じ値であることを確かめる
* [ADR 0171](0171-keep-caption-space-when-captions-are-hidden.md) - 字幕を表示しないときも、動画の下に字幕の場所を空けておき、字幕を切り替えてもプレーヤーの操作の位置を変えない
* [ADR 0172](0172-dim-rows-of-lessons-in-preparation.md) - レッスン一覧で、準備中のレッスンの行の番号と文字を淡い色にして「準備中」の印を付け、選んでもマウスを乗せても反応させない
* [ADR 0173](0173-show-video-loading-and-retry-inside-the-frame.md) - 動画を読み込んでいる間は動画の枠の中に回る印を出し、読み込みに失敗したら枠の中に失敗の文と「もう一度読み込む」ボタンを出す
* [ADR 0177](0177-load-both-design-systems-and-write-app-css-with-classes.md) - 「Progrust」と「Progrust Video」の CSS をどちらもアプリ全体に読み込み、アプリの画面の CSS は動画の枠の中の要素に当たらないように書き、デザインシステムの土台の規則はそのまま両方に効かせる
* [ADR 0179](0179-define-initial-settings-values.md) - `settings.json` が無いときの初期値を定め、テーマは「システムに合わせる」、エディタの文字サイズは 14px、動画は音量 1.0・ミュートなし・1 倍・字幕なしにする
* [ADR 0181](0181-keep-ui-design-files-in-repository.md) - 画面デザイン「Progrust UIデザイン」のファイルをリポジトリの `design-system/ui-design/` に置いてリポジトリのファイルを正とし、claude.ai のアーティファクトは表示用としてデザインシステムと同じやり方で送り、実装のループと計画と実装とレビューはリポジトリのファイルを読む（ADR 0196 で表示用のアーティファクトを残して送ることを変えた）
* [ADR 0182](0182-app-icon-p-in-chapter-colors-on-white.md) - アプリのアイコンを、白い地に第1〜4章の章カラーで塗り分けた「P」を置いた絵にし、元の絵を `src-tauri/app-icon.svg` に置いて、アプリに入れるファイルを Tauri CLI で作る
* [ADR 0196](0196-stop-display-artifacts-for-design-files.md) - デザインシステムと画面デザインの表示用のアーティファクトをやめてリポジトリのファイルだけを正とし、アーティファクトへ送る手順と道具とページのためだけの索引の項目を消し、見る仕組みは作らず、実装前の画面は引き続き画面デザインのファイルに従う
* [ADR 0197](0197-remove-font-import-from-design-system-css.md) - デザインシステムの `bundle.css` から書体を外部から読み込む `@import` の行を消し、ビルドと教材チェックツールの描画で取り除く仕組みをやめ、デザインシステムの CSS が外部から読み込まないことを検査で確かめる
* [ADR 0204](0204-exercise-back-link-goes-to-list.md) - 演習画面の左上のリンクを、レッスンの演習では「レッスン一覧」、生成問題の演習では「生成問題一覧」にし、それぞれの一覧に移る

# 非機能要件

* [ADR 0014](0014-run-learner-code-without-sandbox.md) - 学習者のコードをサンドボックスに入れずに実行する
* [ADR 0090](0090-support-only-authors-mac-until-completion.md) - アプリの完成までは、対応環境を作者の Mac だけとし、一般公開しない前提で開発する
* [ADR 0091](0091-no-distribution-or-auto-update-until-publication.md) - アプリの完成までは配布と更新の仕組みを作らず、作者の Mac でビルドして使い、教材はアプリに同梱する
* [ADR 0115](0115-minimum-rustc-1-100.md) - 学習者の環境に必要な rustc の最低バージョンを 1.100.0 にする
* [ADR 0140](0140-check-csp-in-production-build.md) - デスクトップの開発時の起動（tauri dev）では CSP が掛からないので、CSP の下で動くかどうかを本番ビルドで確かめ、本番ビルドで CSP の違反が出ないことを受け入れ条件にする
* [ADR 0141](0141-allow-data-uri-in-media-src.md) - Remotion の Player の共有の audio 要素が使う無音の data: の音声を拒否しないよう、CSP の media-src に data: を加える
* [ADR 0149](0149-testing-policy.md) - バックエンドのロジックを cargo test の自動テストで、AI の出力を受けた動作を決まった出力を返す偽の claude で確かめ、実物の claude を使う確認・画面の操作・本番ビルドでの確認は人が行い、E2E は行わず、CI ではコミット前の確認と教材チェックツールを ubuntu-latest で実行する
* [ADR 0168](0168-write-fake-claude-in-perl-and-place-by-symlink.md) - 偽の claude を、受け取った環境変数をそのまま書き残せるように sh ではなく perl で書き、偽のツールは原本を指すシンボリックリンクとしてテストごとの一時ディレクトリに置く

# ドキュメントの運用ルール

* [ADR 0079](0079-manage-documents-as-okf-bundle.md) - ドキュメントを `documents/` に集約し、OKF v0.3 のバンドルとして AI エージェントが管理する（ADR 0089 でバンドルのルートのディレクトリの数を変えた）
* [ADR 0080](0080-use-minimal-okf-frontmatter.md) - OKF の frontmatter のうち type・title・description・status・superseded_by だけを使い、type を6つに固定する（ADR 0088 で `Convention` の範囲を、ADR 0089 で type の数を、ADR 0160 で `Convention` の範囲を変えた）
* [ADR 0081](0081-split-specs-into-one-topic-per-file.md) - 仕様を1トピック1ファイルに分け、受け入れ条件と未決事項は機能ごとに1ファイルにまとめる
* [ADR 0082](0082-reference-by-links-without-section-numbers.md) - 文書間の参照を Markdown リンクで書き、節番号での参照を廃止する
* [ADR 0083](0083-express-adr-status-in-okf-frontmatter.md) - ADR の状態を OKF の status と superseded_by で表し、番号・ファイル名・本文は変えない
* [ADR 0085](0085-check-documents-with-own-checker.md) - ドキュメントを独自の検査スクリプトで検査して hook で自動実行し、OKF の公式 validator は手動で実行する
* [ADR 0086](0086-list-all-affected-specs-and-find-adrs-by-path-search.md) - ADR の関連仕様に、判断が内容を決めている仕様をすべて載せ、仕様に関係する ADR を関連仕様の行の検索で探す
* [ADR 0087](0087-note-carry-over-in-adr-description.md) - 後の ADR が別の対象に引き継いだ ADR の description の末尾に、引き継いだ ADR と対象を示す注記を足す
* [ADR 0088](0088-agents-file-found-problems-as-issues.md) - AI エージェントが作業中に見つけた問題を、原因の調査や改善案なしに、確認せずに GitHub の Issue に起票し、その決まりを運用ルールに置く（ADR 0122 でラベルの数を、ADR 0123 で閉じた Issue と同じ問題の扱いを、ADR 0130 で対応するかどうかを決める者を、ADR 0136 で起票する問題の範囲と、いましている作業と関係する問題を調査なしに起票することを、ADR 0160 で `Convention` の範囲を、ADR 0164 で起票した問題を直すときを変えた）
* [ADR 0089](0089-keep-verification-reports-in-documents.md) - 技術検証の結果を `documents/verification/` に1つの Issue につき1文書で残し、そのための type `Verification Report` を加える
* [ADR 0121](0121-no-tracking-issues-use-labels.md) - Issue の一覧のための Issue を作らず、Issue のまとまりをラベルで表して一覧する
* [ADR 0122](0122-hold-label-for-deferred-issues.md) - 対応するが今は対応しない Issue にラベル `保留` を付け、保留の理由と対応するきっかけを Issue のコメントに書く（ADR 0130 で `保留` を付けるかどうかを決める者を変えた）
* [ADR 0123](0123-refile-closed-issue-when-situation-changed.md) - 閉じた Issue と同じ問題を見つけたとき、閉じたときから状況が変わっていれば起票し、対応しない Issue を閉じるときに理由とまた問題になる条件をコメントに書く
* [ADR 0129](0129-note-partial-change-in-adr-description.md) - 後の ADR が採用した ADR の決定の一部だけを変えたときは、古い ADR を採用のままにし、description の末尾に、変えた ADR と変えた内容を示す注記を足す
* [ADR 0130](0130-triage-undecided-issues-by-skill.md) - ユーザーがスキル `triage-issues` を呼んだときは、開いている `未判断` の Issue を AI エージェントが仕分け、ユーザーの判断が要るもの以外は、対応するかどうかを決めて、直す・保留にする・閉じるところまで処理する（ADR 0133 で仕分ける Issue の範囲を、ADR 0136 で起票の決まりを、ADR 0164 で仕分けるときを変えた）
* [ADR 0133](0133-triage-also-picks-up-accepted-open-issues.md) - スキル `triage-issues` で、`未判断` の Issue に加えて、対応すると決めて開いたままの Issue も仕分け、解消しているか、その場で直せるようになっていれば処理する（ADR 0164 で仕分ける Issue の範囲を変えた）
* [ADR 0136](0136-fix-related-problems-before-filing-issues.md) - AI エージェントが見つけた問題がいましている作業と関係するときは、起票する前にその作業の中で直せるかを確かめ、直せるものは起票せずに直し、関係しない問題と、大きな変更やユーザーの判断が要るなどしてその作業の中で直せない問題を起票する
* [ADR 0160](0160-plan-implementation-as-task-issues-on-github.md) - 実装の進め方の決まりを運用ルールに置き、実装を 1 タスク 1 Issue 1 pull request に分けて、計画と進み具合を GitHub の Issue・ラベル・マイルストーンで持ち、タスクの Issue は次に進めるマイルストーンの分だけ作る
* [ADR 0161](0161-run-implementation-loop-locally-and-park-human-work.md) - 実装のループを、作者の Mac でスクリプトから、作業の単位ごとに新しい会話の claude を呼んで回し、人の判断や確認が要ることではループを止めずにラベルと前提で脇に置いて、マイルストーンの区切りにユーザーがまとめて見る（ADR 0189 で権限のモードを変えた）
* [ADR 0162](0162-pick-model-by-difficulty-and-review-in-separate-session.md) - タスクの難易度（小・中・大）で実装のモデルを Haiku・Sonnet・Opus に分け、機械のチェックの後に、実装したのとは別の会話のモデルにレビューさせ、直しを同じモデルで 2 回と引き上げたモデルで 1 回までに限る
* [ADR 0163](0163-loop-merges-pull-requests-by-itself.md) - 実装のループが、機械のチェック・レビュー・CI を通ったタスクの pull request を、ユーザーの承認を待たずに自分で squash マージし、ユーザーは後から pull request を見て、取り消したいものを取り消す
* [ADR 0164](0164-loop-triages-issues-between-tasks.md) - 実装のループが、ユーザーがループを始めたことを仕分けの依頼として、タスクの切れ目に `AI起票` の Issue を仕分け、直すと決めたものを `実装タスク` を付けてループのタスクとして直し、`実装タスク` の付いた Issue をスキル `triage-issues` の対象から外す（ADR 0193 でユーザーの判断が要るとして残す Issue の扱いを、ADR 0194 で同じ実行の中で仕分け直さないことを変えた）
* [ADR 0165](0165-implementation-plan-excludes-content.md) - 実装計画と実装のループの対象を、アプリ本体・開発用ツール・CI とし、教材（レッスン・動画・音声）を作ることは対象にしない
* [ADR 0183](0183-stop-loop-when-blocked-by-permission-denials.md) - 実装のループで、実装する側が判断を求める Issue を挙げずに進められないと答え、その会話で権限に拒否された操作があったときは、タスクを行き詰まりにせず、仕組みの失敗としてラベルを付けずに止める
* [ADR 0184](0184-allow-loop-to-delete-files-with-git-clean.md) - 実装のループの実装する側に、作ったファイルを消す操作として、`rm` ではなく、リポジトリの中の git に入っていないファイルだけを消す `git clean -f -- （パス）` を許可する（ADR 0189 で `rm` を許可しないことを変えた）
* [ADR 0189](0189-run-loop-claude-with-bypass-permissions.md) - 実装のループが呼ぶ claude を、実装・レビュー・仕分けのすべてで権限のモード bypassPermissions で動かし、コミット・push・ブランチの切り替え・pull request の操作・保護するパスの書き換えなどだけを deny で禁止し、claude を呼んだ前後で作業ツリーの HEAD が変わったらループを止める
* [ADR 0193](0193-loop-does-not-retriage-issues-left-for-user.md) - 実装のループの仕分けで、ユーザーの判断が要るとして `未判断` のまま残す Issue には、そのことを示すコメントを書き、そのコメントがあって `未判断` が付いたままの Issue は、次の実行からもループでは仕分け直さない
* [ADR 0194](0194-retry-loop-triage-once-after-permission-denials.md) - 実装のループで、Issue の仕分けの会話に権限で拒否された操作があったときは、その回の Issue を仕分け済みにせず、同じ実行の次のタスクの切れ目で、もう一度だけ仕分ける
* [ADR 0195](0195-loop-checks-node-and-npm-series-before-start.md) - 実装のループは、始めるときに、始めた端末で見つかる node と npm の版が `package.json` の `engines` の系列に合うかを確かめ、合わないときは始めない

# 置き換え済み・廃止

* [ADR 0005](0005-slides-in-restricted-html.md) - スライドを、使える部品を決めた HTML で書く
* [ADR 0037](0037-bundle-content-and-auto-update.md) - 教材をアプリに同梱し、GitHub Releases と Tauri の自動更新でアプリごと配布する
* [ADR 0038](0038-no-os-code-signing-for-now.md) - OS のコード署名を当面行わない
* [ADR 0047](0047-separate-slide-design-system.md) - スライドの見た目を、画面とは別のデザインシステム「Progrust Slides」に定める
* [ADR 0151](0151-copy-design-system-css-into-repository.md) - デザインシステムの CSS とアイコンをリポジトリに写して使い、写しは編集せず、デザインシステムを変えてから取り直す
* [ADR 0175](0175-strip-font-import-with-vite-plugin.md) - デザインシステムの `bundle.css` の、書体を外部から読み込む `@import` の行を、Vite のプラグインで取り除いて読み込み、教材チェックツールの描画でも同じように取り除く
