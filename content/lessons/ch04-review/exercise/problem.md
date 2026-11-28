## 問題

相対パスを組み立てる 3 つの関数 `go_up`・`path_and_up`・`diff_args` を完成させてください。相対パスの末尾に `/..` を付けると、1 つ上のディレクトリを指すパスになります。設定ファイルやログの置き場所を、いまのディレクトリからの相対パスで指すときに使う書き方です。

初期コードの 3 つの関数は、仮の値を返すだけです。「出力」に書いた仕様どおりの値を返すように書き換えてください。`main` は、3 つの関数を呼んで結果を表示します。

## 出力

次の 3 つの関数を実装してください。関数の名前・引数の型・戻り値の型は、このとおりにします。

```rust
fn go_up(path: String, levels: u32) -> String
```

- `path` の後ろに、`/..` を `levels` 個付けた文字列を返します。
- `levels` が 0 のときは、`path` と同じ文字列を返します。

```rust
fn path_and_up(path: String, levels: u32) -> (String, String)
```

- 2 つの文字列のタプルを返します。
- 1 つ目は、`path` と同じ文字列です。
- 2 つ目は、`path` の後ろに、`/..` を `levels` 個付けた文字列です。

```rust
fn diff_args(path: String, levels: u32) -> String
```

- `path` と、`path` の後ろに `/..` を `levels` 個付けた文字列を、この順に、間に半角の空白を 1 つ挟んでつないだ文字列を返します。2 つのパスを引数に取るコマンドに渡す並びを作る処理です。
- 前後には、空白を付けません。

## 例

| 呼び出し | 戻り値 |
|---|---|
| `go_up(String::from("logs/app"), 2)` | `"logs/app/../.."` |
| `go_up(String::from("logs/app"), 0)` | `"logs/app"` |
| `path_and_up(String::from("data"), 2)` | `("data", "data/../..")` |
| `diff_args(String::from("config"), 1)` | `"config config/.."` |
| `diff_args(String::from("config"), 0)` | `"config config"` |

## 制約

- `path` は、1 文字以上 20 文字以下の文字列です。
- `levels` は、0 以上 10 以下です。
- 3 つの関数の名前・引数の型・戻り値の型を変えないでください。
