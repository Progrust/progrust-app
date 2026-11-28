## 考え方

この問題では、コードを 2 つのモジュールに分け、片方のモジュールからもう片方を使います。

まず `jitter` モジュールの `delay_ms` を作ります。`rand::random_range` に範囲を渡すと、その範囲の乱数が 1 つ返ります。待ち時間は 0 以上 `max_ms` 以下なので、`0..=max_ms` を渡します。`rand` は外部クレートなので、`rand::random_range` のように、クレートの名前から始まるパスで呼びます。

次に `retry` モジュールの `plan_delays` を作ります。空の `Vec` を作り、`for` で `count` 回、`jitter::delay_ms(max_ms)` の結果を `push` します。`jitter` は `retry` の外にあるモジュールなので、`retry` の中の `use crate::jitter;` で、ルートから始まる絶対パスで取り込みます。

最後に、`delay_ms` と `plan_delays` に `pub` を付けます。モジュールの中の関数は、既定では、そのモジュールの中でしか使えません。`jitter::delay_ms` は `retry` から、`retry::plan_delays` は `main` と隠しテストから呼ぶので、どちらも `pub` が必要です。

乱数の値そのものは毎回変わるので、隠しテストは、値の範囲と個数だけを確かめます。

## 模範解答

```rust
mod jitter {
    pub fn delay_ms(max_ms: u32) -> u32 {
        rand::random_range(0..=max_ms)
    }
}

mod retry {
    use crate::jitter;

    pub fn plan_delays(max_ms: u32, count: usize) -> Vec<u32> {
        let mut delays = Vec::new();
        for _ in 0..count {
            delays.push(jitter::delay_ms(max_ms));
        }
        delays
    }
}

fn main() {
    let delays = retry::plan_delays(500, 5);
    println!("{delays:?}");
}
```

## ポイント

- **モジュールの定義**: `mod jitter { ... }` で、関数をまとめたモジュールを作ります。モジュールの中の関数は、`jitter::delay_ms` のように、モジュール名と `::` をつけて呼びます。
- **公開範囲**: `pub` を付けた関数だけが、モジュールの外から呼べます。付けないと、同じモジュールの中でしか使えません。
- **use とパス**: `use crate::jitter;` の `crate` は、ファイル全体のルートを指します。`retry` モジュールの中で `jitter` を使うために、ルートから `jitter` までのパスを書いて取り込みます。取り込んだ `jitter` は、`jitter::delay_ms(max_ms)` のように使います。
- **外部クレート**: `rand` のような外部クレートの関数は、`rand::random_range(...)` のように、クレートの名前から始まるパスで呼びます。外部クレートを使うには `Cargo.toml` の `[dependencies]` に書く必要があります。この演習では、アプリが書いてくれます。
- **実務での使われ方**: 乱数のような横断的な処理は、専用のモジュールにまとめ、使う側のモジュールからは `use` で取り込んで呼ぶ、という分け方が、実務のコードでよく使われます。そうすると、乱数の作り方を変えたいときに、モジュール 1 つを直せば済みます。また、乱数のように結果が毎回変わる処理のテストは、決まった値と比べるのではなく、この演習の隠しテストのように、値の範囲と個数を確かめる書き方にします。外部クレートは、実務のプログラムでは、乱数以外にも日時・通信・設定の読み込みなど、多くの場面で使います。

## よくある間違い

**pub を付け忘れる**

`jitter::delay_ms` に `pub` を付けないと、`retry` から呼んだところで、非公開の関数を使ったエラー（E0603）になります。

```text
error[E0603]: function `delay_ms` is private
  --> src/main.rs:13:33
   |
13 |             delays.push(jitter::delay_ms(max_ms));
   |                                 ^^^^^^^^ private function
   |
note: the function `delay_ms` is defined here
  --> src/main.rs:2:5
   |
 2 |     fn delay_ms(max_ms: u32) -> u32 {
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

`note` の行が、`delay_ms` を定義した場所を示しています。そこに `pub` を付けます。`retry::plan_delays` に付け忘れたときは、`main` や隠しテストから呼ぶところで、同じエラーになります。

**use を書かずに jitter を使う**

`retry` モジュールの中で `use crate::jitter;` を書かずに `jitter::delay_ms` を呼ぶと、`jitter` が見つからないエラー（E0433）になります。モジュールの中では、外にあるモジュールの名前は、そのままでは見えないからです。

```text
error[E0433]: cannot find module or crate `jitter` in this scope
  --> src/main.rs:12:25
   |
12 |             delays.push(jitter::delay_ms(max_ms));
   |                         ^^^^^^ use of unresolved module or unlinked crate `jitter`
   |
   = help: you might be missing a crate named `jitter`
help: consider importing this module
   |
 9 +     use crate::jitter;
   |
```

`help` に、足すべき `use` の行が出ています。

**モジュール名を付けずに delay_ms を呼ぶ**

`use crate::jitter;` で取り込んだのは `jitter` というモジュールの名前だけです。`delay_ms(max_ms)` と書くと、関数が見つからないエラー（E0425）になります。`jitter::delay_ms(max_ms)` と書きます。

```text
error[E0425]: cannot find function `delay_ms` in this scope
  --> src/main.rs:13:25
   |
13 |             delays.push(delay_ms(max_ms));
   |                         ^^^^^^^^ not found in this scope
   |
help: a local variable with a similar name exists
   |
13 -             delays.push(delay_ms(max_ms));
13 +             delays.push(delays(max_ms));
   |
help: consider importing this function
   |
 8 +     use crate::jitter::delay_ms;
   |
```

**use のパスを間違える**

`use crate::jitters;` のように、存在しない名前を書くと、取り込めないエラー（E0432）になります。

```text
error[E0432]: unresolved import `crate::jitters`
 --> src/main.rs:8:9
  |
8 |     use crate::jitters;
  |         ^^^^^^^-------
  |                |
  |                no `jitters` in the root
  |
help: a similar name exists in the module
  |
8 -     use crate::jitters;
8 +     use crate::jitter;
  |
```

`no `jitters` in the root` は、ルートに `jitters` という名前が無いという意味です。`help` が、似た名前の `jitter` を示しています。
