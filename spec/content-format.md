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
  <h1>所有権のムーブ</h1>
  <div class="col">
    <pre><code class="language-rust" data-highlight-lines="2">let s1 = String::from("hi");
let s2 = s1;</code></pre>
  </div>
  <div class="col">
    <div class="callout warn">s1 はもう使えない</div>
    <figure class="diagram"><svg viewBox="0 0 400 200">...</svg></figure>
  </div>
  <aside class="ai-note">学習者には表示しない、AI への補足説明</aside>
</section>
```

- スライドは 16:9 の固定比率で表示する。作者は 1280×720 の基準サイズで内容を作り、アプリが表示領域に合わせて拡大・縮小する。
- 見た目（フォント・色・余白）はアプリ共通のテーマ CSS で決まる。スライド側では見た目を指定しない。

### 5.2 レイアウト（`data-layout`）

| 値 | 用途 | 子要素 |
|---|---|---|
| `title` | レッスンの表紙 | `h1`、`p`（サブタイトル） |
| `content` | 文章・箇条書き中心（既定値） | 自由 |
| `two-column` | 左右 2 列 | `h1` と `.col` 2 つ |
| `code` | コード中心 | `h1`、`pre` 1 つ、`p` / `.callout` |
| `diagram` | 図中心 | `h1`、`figure.diagram` 1 つ、`p` / `.callout` |
| `summary` | まとめ | `h1`、`ul` |

### 5.3 使用できる要素

| 要素 | 用途・制約 |
|---|---|
| `h1` | スライドのタイトル。各スライドに 1 つ必須 |
| `h2`、`p`、`ul`、`ol`、`li`、`strong`、`em`、`br` | 文章 |
| `code` | インラインコード |
| `pre > code.language-rust` | Rust のコードブロック。`data-highlight-lines`（例: `"2,4-5"`）で行を強調できる |
| `pre > code.language-text` | コンパイラの出力など |
| `table`、`thead`、`tbody`、`tr`、`th`、`td` | 表 |
| `div.col` | `two-column` の列 |
| `div.callout` | 強調ボックス。種類を `info` / `tip` / `warn` / `ng` / `ok` のクラスで指定する |
| `span.mark` | 文中の強調（マーカー） |
| `figure.diagram > svg` | 図。色は §5.4 のクラスで指定する |
| `img` | 画像。`src` は `assets/` または `../../assets/` 内のファイルに限る |
| `aside.ai-note` | 画面に表示しない補足。質疑応答で AI に渡す |

### 5.4 色の指定

スライドの HTML・SVG では色を直接指定せず、以下のクラスを使う。各クラスの実際の色はテーマ CSS がライト・ダークモードそれぞれで定める。

| クラス | 意味 |
|---|---|
| `c-primary` | 主役（注目させたいもの） |
| `c-accent` | 補助的な強調 |
| `c-ok` | 正しい・成功 |
| `c-ng` | 誤り・エラー |
| `c-muted` | 背景的な情報・無効になったもの |

SVG では `fill-*` / `stroke-*` の接頭辞を付けて使う（例: `class="fill-primary stroke-muted"`）。

### 5.5 禁止事項

以下を含むスライドは教材チェックツール（`content-guidelines.md` §6）でエラーとする。

- §5.3 にない要素
- `style` 属性、`<style>` 要素、`<script>` 要素、`on*` 属性
- SVG の `fill` / `stroke` / `color` 属性での色の直接指定（`none` と `currentColor` を除く）
- 外部 URL の参照

### 5.6 表示時の処理

アプリはスライドを表示する前に、§5.3 にない要素と属性をサニタイズで除去する。

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

## 8. 未決事項

- スライドの色クラス（§5.4）の色を、開いている章の章カラー（`screens.md` §10.1）に合わせるかどうか。
