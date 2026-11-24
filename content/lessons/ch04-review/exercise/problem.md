## 問題

応援の掛け声を作る 3 つの関数 `exclaim`・`plain_and_loud`・`call_twice` を完成させてください。

初期コードの 3 つの関数は、仮の値を返すだけです。「出力」に書いた仕様どおりの値を返すように書き換えてください。`main` は、3 つの関数を呼んで結果を表示します。

## 出力

次の 3 つの関数を実装してください。関数の名前・引数の型・戻り値の型は、このとおりにします。

```rust
fn exclaim(text: String, count: u32) -> String
```

- `text` の後ろに、半角の `!` を `count` 個付けた文字列を返します。
- `count` が 0 のときは、`text` と同じ文字列を返します。

```rust
fn plain_and_loud(text: String, count: u32) -> (String, String)
```

- 2 つの文字列のタプルを返します。
- 1 つ目は、`text` と同じ文字列です。
- 2 つ目は、`text` の後ろに、半角の `!` を `count` 個付けた文字列です。

```rust
fn call_twice(text: String, count: u32) -> String
```

- `text` の後ろに半角の `!` を `count` 個付けた文字列を 2 つ、間に半角の空白を 1 つ挟んでつないだ文字列を返します。
- 前後には、空白を付けません。

## 例

| 呼び出し | 戻り値 |
|---|---|
| `exclaim(String::from("がんばれ"), 3)` | `"がんばれ!!!"` |
| `exclaim(String::from("がんばれ"), 0)` | `"がんばれ"` |
| `plain_and_loud(String::from("Alice"), 2)` | `("Alice", "Alice!!")` |
| `call_twice(String::from("ファイト"), 2)` | `"ファイト!! ファイト!!"` |
| `call_twice(String::from("ファイト"), 0)` | `"ファイト ファイト"` |

## 制約

- `text` は、0 文字以上 20 文字以下の文字列です。
- `count` は、0 以上 10 以下です。
- 3 つの関数の名前・引数の型・戻り値の型を変えないでください。
