# 教材データ形式 仕様書

アプリに同梱する教材（章・レッスン・スライド・演習・要素マスタ）のファイル形式を定める。
教材の書き方のルール（内容面）は `content-guidelines.md` に定める。
AI が生成する演習もこの形式に変換して保存する（`exercise-generation.md`）。

## 1. ディレクトリ構成

リポジトリの `content/` に置き、アプリのリソースとして同梱する。

```
content/
├── chapters.json                   # 章とレッスンの並び（§2）
├── elements.json                   # 要素マスタ（§3）
├── assets/                         # 全レッスン共通の画像
└── lessons/
    └── {lesson_id}/
        ├── lesson.json             # レッスン定義（§4）
        ├── slides.html             # スライド（§5）。章末復習レッスンにはない
        ├── assets/                 # このレッスン専用の画像
        └── exercise/               # 演習（§6）
            ├── problem.md
            ├── explanation.md
            ├── starter/            # 初期コード
            ├── solution/           # 模範解答
            ├── cases.json          # judge.mode = stdout の場合
            └── judge_tests.rs      # judge.mode = test の場合
```

## 2. 章とレッスンの並び（chapters.json）

カリキュラム（章立てと学習順）の唯一の定義とする。

```json
[
  {
    "id": "ch04",
    "title": "所有権",
    "description": "Rust のメモリ管理の中心となる所有権の仕組みを学ぶ",
    "lessons": ["ch04-01", "ch04-02", "ch04-03", "ch04-review"]
  }
]
```

- 配列の順序が章の学習順、`lessons` の順序が章内の学習順である。
- 全レッスンをこの順に一列に並べたものを「学習順」と呼ぶ。「過去のレッスン」は学習順で前にあるレッスンを指す。
- 要素の「導入レッスン」は、学習順で最初にその要素を `primary_elements` に含むレッスンとする。
- あるレッスンにとっての「既習の要素」は、導入レッスンが学習順でそのレッスンより前にある要素とする。
- 各章の最後のレッスンは章末復習レッスン（`ch{NN}-review`）とする。

## 3. 要素マスタ（elements.json）

```json
[
  {
    "id": "ownership.move",
    "chapter": "ch04",
    "name": "所有権のムーブ",
    "description": "値の代入・関数呼び出しによる所有権の移動",
    "related_errors": ["E0382", "E0505"]
  }
]
```

| フィールド | 型 | 説明 |
|---|---|---|
| `id` | string | `{分類}.{要素}` 形式の snake_case。一度公開したら変更しない |
| `chapter` | string | その要素を主に扱う章の ID |
| `name` | string | 表示名 |
| `description` | string | 要素の説明（AI への入力・生成設定画面の説明に使用） |
| `related_errors` | string[] | その要素の誤解で発生しやすいコンパイルエラーコード |

- `related_errors` は、試行ログ中のエラーコードから躓いた要素の候補を出すために使う（`learning-record-and-analysis.md` §6.1）。
- 要素 ID を廃止する場合は、削除せず `"deprecated": true` を付ける。

## 4. レッスン定義（lesson.json）

```json
{
  "id": "ch04-03",
  "chapter": "ch04",
  "kind": "lesson",
  "title": "所有権のムーブ",
  "summary": "代入や関数呼び出しで所有権が移動する仕組みを学ぶ",
  "primary_elements": ["ownership.move"],
  "secondary_elements": ["fn.basic", "string.basic"],
  "estimated_minutes": 20,
  "exercise": {
    "layout": "single",
    "judge": { "mode": "stdout", "target": "src/main.rs" },
    "dependencies": [],
    "readonly_files": [],
    "case_labels": {}
  }
}
```

| フィールド | 説明 |
|---|---|
| `id` / `chapter` | レッスン ID と所属する章の ID。`chapters.json` と一致すること |
| `kind` | `lesson` / `chapter_review` / `generated`（生成問題のみ） |
| `title` / `summary` | 表示名と、レッスン一覧に表示する 1 行の説明 |
| `primary_elements` | 主要素の ID（1〜2 件） |
| `secondary_elements` | 副要素の ID |
| `estimated_minutes` | 目安の所要時間（分） |
| `exercise.layout` | `single`（`src/main.rs` の 1 ファイルのみ）/ `multi`（複数ファイル） |
| `exercise.judge.mode` | `stdout` / `test`（§6.4） |
| `exercise.judge.target` | `test` 方式で隠しテストを組み込むファイル（`src/main.rs` または `src/lib.rs`）。`stdout` 方式では `src/main.rs` 固定 |
| `exercise.dependencies` | 使用する許可クレートの名前（`exercise-runner.md` §2.1 の一覧にあるもの） |
| `exercise.readonly_files` | 学習者が編集できないファイルのパス（`multi` のみ） |
| `exercise.case_labels` | `test` 方式でテスト関数名に付ける表示名（例: `{"empty_input": "空の入力"}`）。省略時は関数名を表示する |

## 5. スライド（slides.html）

### 5.1 構造

1 レッスンのスライドを 1 つの HTML 断片に書く。`<section class="slide">` 1 つが 1 枚のスライドである。

```html
<section class="slide" data-layout="two-column">
  <h1>ムーブした値は、もう使えない</h1>
  <div class="col">
    <pre data-label="src/main.rs"><code class="language-rust">fn main() {
    let s1 = String::from("hi");
    let s2 = <span class="mark">s1</span>;
    println!("{<span class="error">s1</span>}");
}</code></pre>
  </div>
  <div class="col">
    <div class="callout ng">4 行目で s1 を使うと、ムーブ済みの値を使ったエラー（E0382）になります。</div>
    <figure class="diagram"><svg viewBox="0 0 544 240" role="img" aria-label="s1 の所有権が s2 に移る図">...</svg></figure>
  </div>
  <aside class="ai-note">学習者には表示しない、AI への補足説明</aside>
</section>
```

- スライドは 16:9 の固定比率で表示する。作者は 1280×720 の基準サイズで内容を作り、アプリが表示領域に合わせて拡大・縮小する。
- 見た目（書体・色・余白など）は、スライドのデザインシステム（`screens.md` §10.1）に従うアプリ共通のテーマ CSS で決まる。スライド側では見た目を指定しない。

### 5.2 レイアウト（`data-layout`）

| 値 | 用途 | 子要素 |
|---|---|---|
| `title` | レッスンの表紙 | `h1`、`p`（サブタイトル） |
| `content` | 文章・箇条書き中心（既定値） | 自由 |
| `two-column` | 左右 2 列 | `h1` と `.col` 2 つ |
| `code` | コード中心 | `h1`、`pre` 1 つ、`p` / `.callout` |
| `diagram` | 図中心 | `h1`、`figure.diagram` 1 つ、`p` / `.callout`。`pre` は置かない（図とコードを並べるときは `two-column` を使う） |
| `summary` | まとめ | `h1`、`ul` |
| `statement` | 要点を 1 文で大きく見せる | `h1`、`p` |

### 5.3 使用できる要素

| 要素 | 用途・制約 |
|---|---|
| `h1` | スライドのタイトル。各スライドに 1 つ必須 |
| `h2`、`p`、`ul`、`ol`、`li`、`strong`、`em`、`br` | 文章 |
| `code` | インラインコード |
| `pre > code.language-rust` | Rust のコードブロック（§5.4） |
| `pre > code.language-{言語名}` | Rust 以外の言語のコードブロック（例: `language-python`、`language-toml`）。シンタックスハイライトはしない |
| `pre > code.language-plaintext` | ターミナルの出力ではないテキスト（入力ファイルの中身など） |
| `pre > code.language-text` | ターミナルの出力（コンパイラ・cargo の出力、プログラムの実行結果、標準入力とのやりとり）。行番号を表示しない |
| `pre[data-label]` | コードブロックのラベル（ファイル名・言語名） |
| `table`、`thead`、`tbody`、`tr`、`th`、`td` | 表 |
| `div.col` | `two-column` の列 |
| `div.callout` | 強調ボックス。種類を `info`（補足の情報）/ `tip`（ヒント）/ `warn`（注意）/ `ng`（誤り）/ `ok`（正しい）のクラスで指定する |
| `div.points` | ポイント欄。`h2` と `ul` を入れる |
| `span.mark` | 強調（マーカー）。文中でもコードブロックの中でも使う |
| `span.error` | コードブロックの中のエラーの箇所 |
| `figure.diagram > svg` | 図。色は §5.5 のクラスで指定する |
| `figcaption` | 図の説明。`figure.diagram` の中に置く |
| `img` | 画像。`src` は `assets/` または `../../assets/` 内のファイルに限る |
| `aside.ai-note` | 画面に表示しない補足。質疑応答で AI に渡す |

- 図（`svg`）の中で使える要素は `rect`、`path`、`text`、`tspan` とする。属性は `viewBox`、`role`、`aria-label`、`class`、`x`、`y`、`width`、`height`、`rx`、`d`、`text-anchor`、`dominant-baseline`、`font-size`、`font-weight`、`stroke-width`、`stroke-dasharray`、`fill`、`stroke` とする。`fill` と `stroke` の値は `none` か `currentColor` に限る（§5.6）。
- 図には `role="img"` と、図の要点を表す `aria-label` を付ける。`img` には `alt` を付ける。

### 5.4 コードブロック

- `language-text` 以外のコードブロックには、アプリが左に行番号を表示する。行番号はファイルの行番号である。
- コードブロックの `code` には次の属性を書ける。

| 属性 | 内容 |
|---|---|
| `data-start-line` | 載せた 1 行目の、ファイルでの行番号（1 以上の整数。例: `"118"`）。省略時は 1。`data-diff` と併用するときは、変更後のファイルで最初の行の番号を書く |
| `data-diff` | Diff として表示する。各行の先頭の 1 文字に `+`（追加）・`-`（削除）・半角空白（変わらない行）を書き、その後ろにコードを書く。空の行は変わらない行として扱う。削除行には行番号を表示せず、行番号は変更後のファイルの行番号になる |
| `data-scopes` | 変数が生きている範囲を枠で示す。`ラベル:開始行-終了行` を半角空白で区切って並べる（例: `"r:2-7 x:4-6"`）。ラベルには空白と `:` を使えない |

- 作者が属性に書く行番号（`data-start-line`・`data-scopes`）は、表示される行番号で書く。
- `data-scopes` の範囲は 1 つのコードブロックに 3 つまで、入れ子は 2 段までとする。範囲の一部だけが重なる指定はできない。範囲は表示される行番号の中に収める。
- `data-diff` と `data-scopes` は、同じコードブロックで使わない。
- コードブロックの中の `span.mark` と `span.error` は、1 行の中に収める。`language-text` の中では使わない。
- 表示する最後の行番号は 999 以下とする。

### 5.5 色の指定

スライドの HTML・SVG では色を直接指定せず、以下のクラスを使う。各クラスの実際の色は、テーマ CSS がライト・ダークモードそれぞれで定める。

| クラス | 意味 | 色 |
|---|---|---|
| `c-primary` | 主役（注目させたいもの） | 開いている章の章カラー |
| `c-accent` | 補助的な強調 | 開いている章の章カラー（淡い段） |
| `c-ok` | 正しい・成功 | 章によらない固定の色 |
| `c-ng` | 誤り・エラー | 章によらない固定の色 |
| `c-muted` | 背景的な情報・無効になったもの | 章によらない固定の色 |

- HTML では、文中の `strong` に付ける（例: `<strong class="c-ng">できない</strong>`）。
- SVG では `fill-*` / `stroke-*` の接頭辞を付けて使う（例: `class="fill-primary stroke-muted"`）。
- SVG では、さらに次のクラスを使える。
  - `fill-on` / `stroke-on`: 主役などの塗りの上に置く文字・線
  - `fill-surface`: スライドの地の色で塗る
  - `code`（`text` / `tspan` に付ける）: 変数名・型名・値などを等幅の書体で書く
- `callout` の 5 種類と `span.mark` の色も、章によらない固定の色とする。
- 章カラーを使うときの実際の色と、章に属さずに表示するときの色は、スライドのデザインシステム（`screens.md` §10.1）に従う。

### 5.6 禁止事項

以下を含むスライドは教材チェックツール（`content-guidelines.md` §6）でエラーとする。

- §5.3・§5.4 にない要素・属性
- `style` 属性、`<style>` 要素、`<script>` 要素、`on*` 属性
- SVG の `fill` / `stroke` 属性での色の直接指定（`none` と `currentColor` を除く）
- 外部 URL の参照
- §5.4 の制約に違反するコードブロック（`data-start-line` が 1 以上の整数でない、`data-diff` の行の先頭が `+`・`-`・半角空白のどれでもない、`data-scopes` の範囲の重なり・入れ子・個数の違反、表示される行番号の外の範囲、複数行にまたがる `span.mark` / `span.error`、`language-text` の中の `span.mark` / `span.error`、999 を超える行番号）

### 5.7 表示時の処理

- アプリはスライドを表示する前に、§5.3・§5.4 にない要素と属性をサニタイズで除去する。
- サニタイズのあと、アプリはスライドのデザインシステムの README「アプリが行う描画処理」に従って HTML を整える。整えるのは、章カラーとテーマの指定、柱（レッスン名・章名）、コードの字句と行の分割（`span.mark` / `span.error` は残す）、行番号、Diff の行の印、生きている範囲の枠、rustc の出力の色付けである。
- サニタイズのあとにアプリが加える属性（範囲の枠の位置を渡す `style` 属性など）は、§5.6 の禁止事項の対象外とする。

## 6. 演習

### 6.1 ファイル

| ファイル | 内容 |
|---|---|
| `problem.md` | 問題文（Markdown） |
| `explanation.md` | 解説（Markdown） |
| `starter/` | 初期コード。cargo パッケージのルートからの相対パスで配置する（例: `starter/src/main.rs`）。`Cargo.toml` は置かない（アプリが生成する）。`src/main.rs` を必ず含める |
| `solution/` | 模範解答。構成は `starter/` と同じ |
| `cases.json` | `stdout` 方式のテストケース（§6.4） |
| `judge_tests.rs` | `test` 方式の隠しテスト（§6.4） |

### 6.2 問題文（problem.md）

以下の見出しを持つ。

| 見出し | 内容 |
|---|---|
| `## 問題` | 何を作るか |
| `## 入力` | 標準入力の形式（`stdout` 方式で入力がある場合） |
| `## 出力` | 出力の形式（`stdout` 方式）、または実装する関数のシグネチャと仕様（`test` 方式） |
| `## 例` | 入出力例、または呼び出し例 |
| `## 制約` | 使ってはいけないもの、守るべき条件など（任意） |

### 6.3 解説（explanation.md）

以下の見出しを持つ。

| 見出し | 内容 |
|---|---|
| `## 考え方` | 解き方の方針 |
| `## 模範解答` | 模範解答のコード（`solution/` と同じ内容） |
| `## ポイント` | このレッスンの主要素がどう使われているか |
| `## よくある間違い` | 想定される誤りと、そのときに出るエラー |

### 6.4 判定方式

**`stdout` 方式**: プログラムに標準入力を与えて実行し、標準出力を期待値と比較する。

```json
[
  { "name": "基本の例", "stdin": "3\n1 2 3\n", "expected_stdout": "6\n" },
  { "name": "空の入力", "stdin": "0\n\n", "expected_stdout": "0\n" }
]
```

| フィールド | 説明 |
|---|---|
| `name` | テストケースの表示名 |
| `stdin` | 標準入力として与える文字列 |
| `expected_stdout` | 期待する標準出力 |

**`test` 方式**: 隠しテストを学習者のコードに組み込んで `cargo test` で実行する。

```rust
// judge_tests.rs
use super::*;

#[test]
fn empty_input() {
    assert_eq!(sum(&[]), 0);
}
```

- `judge.target` のファイルの子モジュールとして組み込まれるため、`use super::*;` で学習者のコードを参照する。
- テスト関数名は snake_case の英語とし、表示名は `lesson.json` の `case_labels` で与える。
- 比較方法・組み込み方法・制限時間は `exercise-runner.md` §5 に定める。

## 7. 受け入れ条件

- `chapters.json` に載っているすべてのレッスンについて、`lesson.json` と必要なファイルが揃っていることを教材チェックツールで確認できる。
- どのレッスンの `primary_elements` にも含まれない要素（`deprecated` を除く）は、教材チェックツールでエラーになる。
- `style` 属性や `<script>` を含むスライドはチェックでエラーになり、アプリで表示しても除去される。
- ダークモードに切り替えたとき、スライド内の文字・図の色がすべてテーマに合わせて変わる。
- 第4章のスライドでは `c-primary` が第4章の章カラーで表示され、第2章に移すと第2章の章カラーで表示される。`c-ok`・`c-ng`・`callout`・`span.mark` は、どの章でも同じ色で表示される。
- `data-start-line="118"` のコードブロックは、行番号が 118 から表示される。
- `data-diff` のコードブロックでは、削除行に行番号が表示されず、ほかの行は変更後のファイルの行番号で表示される。
- `language-text` のコードブロックには行番号が表示されない。
- 範囲の一部だけが重なる `data-scopes` を書いたスライドは、教材チェックツールでエラーになる。

## 8. 未決事項

- 同じファイルの離れた複数の箇所を、1 つのコードブロックで省略を挟んで載せる書き方（今はコードブロックを分け、それぞれに `data-start-line` を書く）。
