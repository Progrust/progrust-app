---
type: Specification
title: "要素の分け方と要素 ID の付け方"
description: "要素マスタ（`content/elements.json`）の要素をレッスンを作る前にすべて定めておくこと、1 レッスン 1 要素を基本とする分け方と 2 つに分ける条件、要素にしないもの、要素 ID の形式と分類の一覧、`name`・`description`・`related_errors` の決め方、要素を足すときと分け方を変えるときの決まりを定める。"
---

- 要素マスタ（[要素マスタの形式（elements.json）](../content-format/elements-json.md)）の要素は、レッスンを作る前に、すべて `content/elements.json` に定めておく。
- レッスンを作るときは、[章とレッスンの一覧](../curriculum/chapters/index.md) の「主要素」の列に定めた要素を使う。その場で新しい要素を作らない。

## 要素の分け方

- Rust の章の、演習のあるレッスン 1 つにつき、要素を 1 つ置く。そのレッスンが主に扱う内容の全体を、1 つの要素にする。
- Rust の章のどのレッスン（動画のみのレッスンと章末復習レッスンを除く）も、新しい要素を 1 つ以上導入する。1 つの要素を、複数のレッスンにまたがらせない。
- 次の両方に当てはまるレッスンだけ、要素を 2 つに分ける。
  - レッスンが、別々に使える 2 つの内容を扱っている。
  - そのどちらも、後のレッスンや演習で単独でよく使うか、単独で躓きやすい。
- どのレッスンにどの要素を置くかは、[章とレッスンの一覧](../curriculum/chapters/index.md) の「主要素」の列に定める（[章とレッスンの一覧の読み方](../curriculum/chapters/how-to-read.md)）。
- 次のものは、要素にしない。
  - 周辺知識（[教材作成におけるカリキュラムの決まり](curriculum.md)）。
  - C の章で扱う内容。C のレッスンは要素を持たない（[教材作成におけるカリキュラムの決まり](curriculum.md)）。
  - 個々のメソッド・マクロ・構文の細部。それを扱う要素の `description` に含める。
  - [カリキュラムで扱わない内容](../curriculum/out-of-scope.md)。

## 要素 ID

- 要素 ID は `{分類}.{要素}` の形で書く。分類と要素は、どちらも英小文字で始め、英小文字・数字・`_` だけで書く（snake_case）。
- 分類は、Rust の機能のまとまりを表す。下の一覧から選ぶ。章の番号、レッスンの番号、学習順を表す語は入れない。1 つの分類の要素が、複数の章にあってよい。
- 要素の部分には、その要素が扱うキーワード・型・メソッド・概念の英語の名前を書く。
  - 2 つの内容を並べるときは `_` でつなぐ（例: `ownership.clone_copy`）。
  - その分類の入口になる要素（定義と基本の使い方を扱う要素）は `basic` とする（例: `vec.basic`）。

| 分類 | 表すもの |
| --- | --- |
| `program` | プログラムの構成 |
| `var` | 変数・定数 |
| `fmt` | 出力の書式 |
| `scalar` | スカラー型（整数・浮動小数点数・論理型・文字型）と、その型の変換 |
| `tuple` | タプル |
| `array` | 配列 |
| `fn` | 関数 |
| `expr` | 文と式 |
| `control` | 制御構文（`if`・`loop`・`while`・`for`） |
| `ownership` | 所有権 |
| `string` | 文字列（`String`） |
| `borrow` | 参照と借用 |
| `slice` | スライス |
| `struct` | 構造体 |
| `enum` | 列挙型 |
| `pattern` | パターンマッチ（`match`・`if let`・`while let` を含む） |
| `option` | `Option` |
| `vec` | `Vec` |
| `hashmap` | `HashMap` |
| `collections` | `Vec`・`HashMap` 以外のコレクション |
| `error` | エラー処理（panic・`Result`・エラー型） |
| `io` | 入出力 |
| `module` | モジュール |
| `crate` | クレート |
| `test` | テスト |
| `generic` | ジェネリクス |
| `trait` | トレイト |
| `std_trait` | 標準ライブラリのトレイトの実装と derive |
| `closure` | クロージャ |
| `iter` | イテレータ |
| `lifetime` | ライフタイム |
| `smart_ptr` | スマートポインタ |
| `trait_object` | トレイトオブジェクト |
| `types` | 型を使った設計と型システム |
| `thread` | スレッド |
| `sync` | スレッド間の共有と通信 |
| `async` | 非同期処理 |
| `macro` | マクロ |
| `unsafe` | unsafe |

- 分類を足すのは、どの分類にも入らない要素を足すときだけとし、この一覧にも足す。

## name・description・related_errors

- `name` には、要素そのものの名前を名詞で書く。レッスンのタイトルと同じでなくてよい。要素マスタの中で重複させない。
- `description` には、その要素が扱う範囲（構文、型、メソッド、概念）を具体的に並べる。[章とレッスンの一覧](../curriculum/chapters/index.md) の「主に扱う内容」と食い違わせない。
- `related_errors` には、その要素の誤解から直接出るコンパイルエラーのコードだけを書く。無ければ空の配列にする。
  - 必要な最低バージョンの rustc（[外部ツール（cargo・claude）への依存](../architecture/external-tools.md)）で、その誤解のコードをコンパイルして、実際に出ることを確かめたコードを書く。エラーコードは、rustc の版によって変わることがある。
  - E0308・E0277・E0599・E0425 のように多くの場面で出るコードは、その誤解が要素の中心である場合だけ書く。
  - エラーコードの付かないエラー（マクロの展開のエラーなど）は書けない。
- `name`・`description`・`related_errors` は、後から変えてよい。変えられないのは要素 ID だけである（[要素マスタの形式（elements.json）](../content-format/elements-json.md)）。

## 要素を足すとき・分け方を変えるとき

- カリキュラムにレッスンを足したときなど、要素を足すときは、上の決まりで要素 ID を決め、要素マスタと、[章とレッスンの一覧](../curriculum/chapters/index.md) の「主要素」の列の両方に足す。
- 要素の分け方を変えるときは、新しい要素を足し、古い要素に `deprecated` を付ける（[要素マスタの形式（elements.json）](../content-format/elements-json.md)）。[章とレッスンの一覧](../curriculum/chapters/index.md) の「主要素」の列は、新しい要素に書き換える。
