# 用語集

仕様書・実装・UI で使う用語を定める。仕様書間で用語の意味がずれないよう、新しい用語を導入する場合はここに追加する。

## 教材

| 用語 | 英語表記（コード上の名前） | 定義 |
|---|---|---|
| 教材 | content | アプリに同梱する章・レッスン・要素マスタの総称。形式は `content-format.md` |
| 章 | chapter | レッスンをまとめる単位。ID は `ch{NN}`（例: `ch04`） |
| レッスン | lesson | 「動画 1 本 + 演習 1 問」の学習単位。ID は `ch{NN}-{NN}`（例: `ch04-03`） |
| 章末復習レッスン | chapter review | 各章の最後に置く、演習のみのレッスン。ID は `ch{NN}-review` |
| 動画 | video | レッスンの知識をインプットするための、ナレーションつきの 16:9 の教材。動画ファイルではなく、アプリの中で描画する（`content-format.md` §5） |
| シーン | scene | 動画を構成する場面の単位（表紙、まとめ など） |
| 台本 | script | 動画のナレーションを文ごとに並べたデータ（`script.json`。`content-format.md` §5.2） |
| 文 | sentence | 台本の 1 件。ID を持ち、音声 1 ファイルに対応する |
| ナレーション | narration | 台本を読み上げた音声 |
| 合図 | cue | 絵を動かす位置の指定。「文の開始 + 秒」で表す（`content-format.md` §5.4） |
| 声の設定 | voice settings | 音声を作るときの VOICEPEAK のナレーターと設定値。全レッスン共通（`content/voice.json`。`content-format.md` §5.3） |
| 読みの対応表 | readings | 読み間違えやすい語と、`say` に書く読みの対応。全レッスン共通（`content/readings.json`。`content-format.md` §5.3） |
| 再生中の文 | current sentence | 再生位置にあたる文（決め方は `screens.md` §5） |
| 文の頭 | sentence start | 文の開始位置（`screens.md` §5） |
| 検索ワード | tags | レッスン検索の対象にする語（`lesson.json` の `tags`） |
| 演習 | exercise | コードを書いて提出し、正誤判定を受ける問題。レッスンの演習と生成問題がある |
| 生成問題 | generated exercise | AI が生成した演習。ID は `gen-{生成 ID}` |
| 要素 | element | Rust の文法・概念の単位（例: 所有権のムーブ）。要素 ID（例: `ownership.move`）で参照する |
| 要素マスタ | element master | 全要素の定義（`elements.json`） |
| 主要素 | primary elements | そのレッスン・演習の学習対象の要素 |
| 副要素 | secondary elements | 学習対象ではないが演習に登場する既習の要素 |
| 学習順 | learning order | 全レッスンを章・レッスンの順に一列に並べた順序（`content-format.md` §2） |
| 導入レッスン | introducing lesson | 学習順で最初にその要素を主要素とするレッスン（`content-format.md` §2） |
| 既習の要素 | learned elements | 導入レッスンが、あるレッスンより学習順で前にある要素（`content-format.md` §2） |
| 問題文 | problem | 演習の課題を説明する文章 |
| 初期コード | starter code | 演習を開いたときにエディタに表示されるコード |
| 模範解答 | solution | 判定を通過する作者の解答コード |
| 解説 | explanation | 正解後に表示する、考え方と模範解答の説明 |
| 判定方式 | judge mode | `stdout`（標準出力の比較）または `test`（隠しテスト） |
| テストケース | case | 判定の 1 単位。`stdout` 方式では入力と期待出力の組、`test` 方式では 1 つのテスト関数 |
| 許可クレート | allowed crates | 演習で使ってよい外部クレートの一覧 |

## 学習の進行

| 用語 | 英語表記 | 定義 |
|---|---|---|
| 実行 | run | エディタのコードをビルドして実行すること。判定は行わない |
| 提出 | submit | エディタのコードをビルドし、正誤判定を行うこと |
| 正解 | pass | 提出ですべてのテストケースを通過すること |
| 完了 | completed | レッスンの演習に一度以上正解した状態 |
| 進捗 | progress | レッスンごとの完了状態などの記録（`progress.json`） |
| 下書き | draft | 編集中のコードの自動保存データ |
| 試行セッション | attempt session | 1 つのレッスン（または生成問題）に取り組み始めてから終了するまでの単位。試行ログ 1 ファイルに対応する |
| 試行ログ | attempt log | 試行セッション中の行動をアプリが記録した JSON |

仕様書では「セッション」と単独で書かず、「試行セッション」または「AI セッション」と書く。ただし、データ上のフィールド名は `session_id` のままとする。

## AI 連携

| 用語 | 英語表記 | 定義 |
|---|---|---|
| AI セッション | AI session | `claude -p` の 1 回の呼び出し |
| 記録セッション | record session | 試行ログから学習記録を作る AI セッション |
| 分析セッション | analysis session | 学習記録から分析ファイルを更新する AI セッション |
| 学習記録 | learning record | 試行セッションごとに AI が作る Markdown ファイル |
| 分析ファイル | analysis file | 学習者全体の傾向を表す、常に最新に保たれる Markdown ファイル |
| 習熟レベル | level | 要素ごとの理解度（0〜4） |
| ジョブ | job | バックグラウンドで実行する AI 処理の単位（記録・分析・再構築・問題生成） |
| 質疑応答 | chat | 動画や解説について AI に質問する機能 |

## 画面

| 用語 | 英語表記（コード上の名前） | 定義 |
|---|---|---|
| デザインシステム | design system | 画面の見た目（配色・書体・余白・角丸・アイコン・部品）の定義と使い方をまとめたもの（`screens.md` §10.1） |
| 動画のデザインシステム | video design system | 動画の中身（`content-format.md` §5）の見た目の定義と使い方をまとめたもの「Progrust Video」。画面のデザインシステムとは別に持つ（`screens.md` §10.1） |
| 字幕 | captions | 再生中の文を、動画の枠の外に文字で表示する機能（`screens.md` §5） |
| レッスン検索 | lesson search | 検索ワードとレッスンのタイトルから、レッスンを探す機能（`screens.md` §9） |
| 章カラー | chapter color | 章ごとに割り当てた色。開いている章の画面のアクセントになる。動画では、再生するときに指定された章の章カラーが主役の色になる（`screens.md` §10.1、`content-format.md` §5.7） |
| 墨色 | ink | 本文の文字色。章に属さない画面ではアクセントにもなる（`screens.md` §10.1） |
