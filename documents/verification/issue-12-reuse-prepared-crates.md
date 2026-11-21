---
type: Verification Report
title: "許可クレートのビルド結果の共有の技術検証"
description: "初回準備でビルドした許可クレートを、各演習が作り直さずに使えるかを Issue #12 で検証した結果（rand だけの今の一覧では作り直しなし、10 クレートの想定の一覧では feature の違いによる作り直し、Cargo.lock の不一致がエラーにならないこと、空のライブラリへの path 依存で作り直しがなくなること、空の main.rs で warmup が失敗すること、ユーザーが Claude の推奨に任せて案 b にした判断）を記す。"
---

- 日付: 2026-11-20
- Issue: [#12](https://github.com/Progrust/progrust-app/issues/12)
- 確かめた OS: macOS 27.2.1（Apple Silicon、APFS）
- 関連仕様: [許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)、[project/ の組み立て](../spec/exercise-runner/project-assembly.md)、[ADR 0010](../decisions/0010-allow-listed-crates.md)、[ADR 0011](../decisions/0011-prepare-crates-on-first-launch.md)、[ADR 0012](../decisions/0012-separate-build-dir-without-crates.md)

## 環境

- macOS 27.2.1（Build 26B131）、Apple Silicon（aarch64）、8 コア、メモリ 16 GB、APFS
- rustc 1.99.1（58cf8f9c3 2026-10-13）、cargo 1.99.1（5f94df478 2026-08-27）、stable チャネル
- `~/.cargo/config.toml` なし。環境変数 `CARGO_*`・`RUSTFLAGS` の設定なし。sccache なし
- `CARGO_HOME` は、他のエージェントとパッケージキャッシュのロックを取り合わないように検証用の専用ディレクトリにした（ダウンロードから測るときは空のディレクトリ）
- 他のエージェントが同時にビルドしていたため、load average は 4〜19 で変動した。時間は 3〜5 回測った

## 方法

許可クレートの一覧を 2 つ用意した。

1. 今の一覧: 仕様にある `rand = "=0.9.1"` だけ。
2. 想定の一覧（仮定）: 今の一覧だけでは検証にならないので、カリキュラムの未決事項にあるクレートと、feature の差が出やすいクレートを加えた。バージョンは検証時点の最新版。

```toml
[dependencies]
rand = "=0.9.1"
serde = { version = "=1.0.229", features = ["derive"] }
serde_json = "=1.0.154"
regex = "=1.13.2"
itertools = "=0.15.0"
thiserror = "=2.0.21"
anyhow = "=1.0.104"
tokio = { version = "=1.54.1", features = ["full"] }
futures = "=0.3.34"
tokio-stream = "=0.1.19"
```

`Cargo.lock` は、この一覧を `[dependencies]` に持つパッケージ `progrust_allowed_crates` で `cargo generate-lockfile` して作った（63 パッケージ）。

仕様どおりに組み立てた。

- `warmup/`: パッケージ名 `progrust_warmup`、edition 2024。`[dependencies]` は `allowed-crates.toml` そのまま。同梱の `Cargo.lock` をコピーし、`CARGO_TARGET_DIR=workspace/target` で `cargo build` と `cargo test --no-run` を実行する。
- `project/`: パッケージ名 `progrust_exercise`、edition 2024。`[dependencies]` には演習のクレートだけを `allowed-crates.toml` の行のまま書く。同梱の `Cargo.lock` をコピーし、毎回作り直す。
- ビルド: `cargo build --offline --message-format=json-diagnostic-rendered-ansi` と `cargo test --offline --no-run --message-format=json-diagnostic-rendered-ansi`。いずれも `CARGO_TARGET_DIR=workspace/target`、`RUST_BACKTRACE=0` 付き。

作り直しの判定は、JSON の `compiler-artifact` の `fresh` が `false` の依存とした。補助として `cargo build -v`（`Fresh`/`Compiling`/`Dirty`）と、`CARGO_LOG=debug` の fingerprint のログ（この版の target 名は `cargo::compiler::fingerprint`）を見た。組み合わせを試すたびに、warmup 直後の `target/` のスナップショット（`cp -cRp` による APFS のクローン）に戻した。先に試した組み合わせのビルド結果が残ると、後の作り直しが隠れるためである。

## 結果

### 今の一覧（rand だけ）では作り直しが起きない

warmup の後に rand に依存する `project/` を組み立てた。`cargo build` と `cargo test --no-run` のどちらでも、依存はすべて Fresh だった。

```
{"cmd": "cargo build --offline ...", "exit": 0, "elapsed": 0.2, "fresh_units": 11, "rebuilt_deps": []}
{"cmd": "cargo test --offline --no-run ...", "exit": 0, "elapsed": 0.14, "fresh_units": 11, "rebuilt_deps": []}
       Fresh libc v0.2.191
       Fresh getrandom v0.3.4
       Fresh rand_core v0.9.5
       Fresh rand_chacha v0.9.0
       Fresh rand v0.9.1
```

許可クレートが 1 つだけなので、一部だけに依存する演習はありえない。今の一覧では問題が起きない。

### 想定の一覧（10 クレート）で仕様どおりに組み立てると、依存の一部を作り直す

依存を 1 つだけにした演習では、10 個のうち 7 個で依存の一部を作り直した。`cargo build` と `cargo test --no-run` の結果は同じだった（load average は 6〜13）。

| 演習の依存 | 作り直した依存 | 時間（中央値、3 回） |
| --- | --- | --- |
| 全 10 クレート | なし | 0.6 s |
| rand | libc（とビルドスクリプト）、getrandom、rand_core、rand_chacha、rand | 1.29 s |
| serde | proc-macro2、quote、syn、serde_derive、serde（とビルドスクリプト 2 つ） | 2.86 s |
| serde_json | memchr、serde_core、serde_json（とビルドスクリプト） | 1.9〜2.0 s（各 1 回） |
| regex | memchr、aho_corasick、regex、regex_automata | 2.1 s（各 1 回） |
| itertools / anyhow / tokio | なし | 0.1〜0.5 s |
| thiserror | syn、thiserror_impl、thiserror | 1.7 s（各 1 回） |
| futures | futures_core、futures_channel、futures_util、futures_executor、futures | 1.62 s |
| tokio-stream | tokio、tokio_stream | 0.88 s |
| tokio + futures + tokio-stream | なし | 0.38 s |

原因は feature の集合（feature unification）の違いである。`cargo tree -e features` で warmup と project を比べた。

```
rand        → libc         warmup: [default,std]                        project: []
serde       → syn          warmup: [...,derive,full,parsing,...]        project: [...,derive,parsing,...]（full なし）
              proc-macro2  warmup: [default,proc-macro]                 project: [proc-macro]
serde_json  → memchr       warmup: [alloc,default,std]                  project: [alloc,std]
              serde_core   warmup: [result,std]                         project: [std]
futures     → futures-core warmup: [alloc,default,std]                  project: [alloc,std]
tokio-stream→ tokio        warmup: [full ほか全部]                       project: [default,sync,time]
```

- コンパイルされる内容が同じでも、`default` という feature の名前が集合にあるかどうかだけで別のビルド単位になる（例: memchr）。
- fingerprint のログでは、変更を検知して作り直す（Dirty）のではない。feature の集合が違うのでビルド単位のハッシュが変わり、新しい単位として扱われる。

```
libc v0.2.191 target="libc"}: cargo::compiler::fingerprint:     err: failed to read `.fingerprint/libc-1aff22acc777551d/lib-libc`
```

作り直しは組み合わせごとに 1 回だけである。feature の組み合わせが違うビルド結果は上書きされず、別のハッシュで `target/` に並ぶ（例: `liblibc-1aff….rlib` と `liblibc-2141….rlib`）。同じ組み合わせの 2 回目からは Fresh になる。

```
[rand] build: 3.12s 作り直し 6   → [rand] test: 0.33s 作り直しなし → [rand] build: 0.31s なし
[serde] build: 8.21s 作り直し 7  → [serde_json] build: 5.76s 作り直し 4 → 以後 serde・serde_json・rand・全部を交互に: すべて 0.3s 前後、作り直しなし
```

- 作り直しに要るソースは `CARGO_HOME` にあるので、`--offline` でも失敗しない。かかるのは時間（1〜8 秒、負荷による）とディスクだけである。
- 学習者から見ると、その組み合わせを初めて使う演習の初回の実行だけが数秒遅い。

`cargo test --no-run` は、依存を `cargo build` と同じ単位で使う。warmup の `cargo test --no-run` でも、コンパイルされたのはルートのパッケージだけだった。仕様の手順 2 の `cargo test --no-run` は、依存のビルドを増やさない。

```
   Compiling progrust_warmup v0.0.0 (...)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.13s
```

warmup は普通の出力、project は `--message-format=json-...` でビルドした。全クレートに依存する project は Fresh だったので、この違いは作り直しの原因にならない。`[dependencies]` に regex、`[dev-dependencies]` に serde_json を持つ演習（仕様にはない）では、`cargo build` で regex 系を、`cargo test --no-run` でさらに serde_json 系を作り直した。下の案 b では、どちらも Fresh だった。

### Cargo.lock の不一致はエラーにならない

同梱の `Cargo.lock` には、演習が使わない entry と、別の名前のルート（`progrust_allowed_crates`）が入っている。`--offline` でビルドすると、cargo はエラーにも警告にもせず、`project/Cargo.lock` から使わない entry を削り（63 → 17 パッケージ）、ルートの名前を書き換える。

- 残った entry のバージョンは、同梱の lock とすべて同じだった（同梱の lock にないバージョンは 0 件）。
- warmup でも、ルートの名前の 1 行だけが書き換わる。
- `--locked` を付けると、全クレートに依存する演習でも失敗する。仕様は `--locked` を付けないので問題ないが、`--locked` や `--frozen` を足すと壊れる。

```
error: cannot update the lock file .../project/Cargo.lock because --locked was passed to prevent this
```

### 作り直しを避ける方法

| 案 | 結果 | 備考 |
| --- | --- | --- |
| a. project の `[dependencies]` に毎回全許可クレートを書く | 成り立つ（全組み合わせで Fresh） | 演習の `dependencies` にないクレートも `use` できてしまい、仕様の「演習の dependencies に挙げたクレートだけを書く」と食い違う。使わないクレートはバイナリにリンクされない（474,240 B と 474,272 B） |
| b. 全許可クレートに依存する空のライブラリ `progrust_allowed_crates` を固定の場所（例: `workspace/allcrates/`）に作り、warmup と project の両方が path 依存で使う。project には演習のクレートだけを直接書く | 成り立つ（10 個の単独と 2 つの組み合わせ、build と test の両方、dev-dependencies ありでも、作り直しは 0。時間 0.2〜0.5 s） | feature の集合が warmup と同じになる。直接書いていないクレートは `use` できない（E0432 ``use of unresolved module or unlinked crate `regex` ``）。空のライブラリ自体は名前で参照できるが、中身がないので何も使えない。バイナリの大きさも変わらない（474,160 B）。warmup の時間とディスクは仕様どおりの場合と同じ（`target/` 254 MB） |
| c. `[workspace]` で warmup と project をメンバーにし、`-p progrust_exercise` でビルドする | 成り立たない | stable の feature unification は「選んだパッケージだけ」なので、仕様どおりの場合と同じものを作り直す。全メンバーでそろえる `resolver.feature-unification = "workspace"` は `-Z feature-unification` が要る（1.99.1 でも nightly のみ）。`RUSTC_BOOTSTRAP=1` で試すと、rand と regex は Fresh だったが serde は作り直した |
| d. 作り直しを受け入れる | 動作上は問題ない | 組み合わせの初回だけ 1〜8 秒遅い。オフラインでも動く。組み合わせが増えるほど `target/` が大きくなる |
| e. warmup でレッスンの組み合わせごとにもビルドする | 試していない | 生成問題の組み合わせは事前に分からない |

案 b の `project/Cargo.toml` は次の形になる。末尾の 1 行が追加である。ライブラリの `src/lib.rs` は空で、`Cargo.toml` の `[dependencies]` は `allowed-crates.toml` そのままである。

```toml
[package]
name = "progrust_exercise"
version = "0.1.0"
edition = "2024"

[dependencies]
tokio = { version = "=1.54.1", features = ["full"] }
futures = "=0.3.34"
tokio-stream = "=0.1.19"
progrust_allowed_crates = { path = "../allcrates" }
```

案 b を再現するコマンドは次のとおり。warmup も同じライブラリに依存させ、project に 1 行足すと、`Compiling` が出るのは `progrust_exercise` だけになる。

```sh
# warmup の Cargo.toml: [dependencies] は path 依存 1 行だけ
printf '[package]\nname = "progrust_warmup"\nversion = "0.0.0"\nedition = "2024"\n\n[dependencies]\nprogrust_allowed_crates = { path = "../allcrates" }\n' > $W/warmup/Cargo.toml
(cd $W/warmup && CARGO_TARGET_DIR=$W/target cargo build -q)
# project（rand だけの演習）に同じ 1 行を足す
echo 'progrust_allowed_crates = { path = "../allcrates" }' >> $W/project/Cargo.toml
(cd $W/project && CARGO_TARGET_DIR=$W/target cargo build --offline -v 2>&1 | grep -E '^ +(Compiling|Fresh|Dirty)')
```

案 b の `Cargo.lock` の注意点がある。

- 今回は、同梱の `Cargo.lock` のルートのパッケージ名（`progrust_allowed_crates`）が、path 依存のライブラリと同じ名前だった。そのため lock にもともとその entry があった。
- 名前が違う場合は、cargo が `--offline` のまま lock を書き換えると考えられる（上の「Cargo.lock の不一致」と同じ動作）。この場合は確かめていない。
- 案 b を採るなら、lock を作るときのルートのパッケージを、そのライブラリそのものにしておくと一致する。

### 仕様どおりの組み立ての再現

決め手になった、仕様どおりの組み立てで作り直しが起きることの再現を書く。`$W` は作業領域、`$D/runner/` に `allowed-crates.toml` と同梱の `Cargo.lock` がある。

```sh
# project を組み立てる関数: asm <ディレクトリ> <パッケージ名> <許可クレート名> <main.rs の中身>
asm(){ rm -rf $1; mkdir -p $1/src; printf '[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\n' $2 > $1/Cargo.toml; grep -E "^$3 = " $D/runner/allowed-crates.toml >> $1/Cargo.toml; cp $D/runner/Cargo.lock $1/; printf '%s\n' "$4" > $1/src/main.rs; }
# warmup の後に、rand だけの演習をビルドする
asm $W/project progrust_exercise rand 'fn main() { println!("LEARNER"); }'
(cd $W/project && CARGO_TARGET_DIR=$W/target cargo build --offline -v 2>&1 | grep -E '^ +(Compiling|Fresh|Dirty)')   # Compiling libc などが出る
(cd $W/project && CARGO_TARGET_DIR=$W/target cargo tree --offline -e features -i libc | head -3)
```

### その他に分かったこと

- 仕様の「空の `src/main.rs`」では warmup が失敗する。ファイルが空だと `main` がないので、依存はすべてビルドできても、最後に `cargo build` がエラーで終わる（終了コードが 0 でない）。仕様どおりだと `prepared.json` は毎回 `status: failed` になる。`fn main() {}` が必要である。

  ```
  error[E0601]: `main` function not found in crate `progrust_warmup`
  error: could not compile `progrust_warmup` (bin "progrust_warmup") due to 1 previous error
  ```

- warmup の時間は、想定の一覧で、依存のコンパイルが 7.3〜12.5 秒（load 10〜16）。ダウンロード（52 クレート、`.crate` で 5.7 MB、index で 5.3 MB）を含めると 8.5〜9.2 秒。macOS では windows-sys など Windows 用のクレート（10 個）はダウンロードされない。
- 想定の一覧の lock に含まれるクレートの `rust-version` は、最大で 1.85（tokio）だった。最低バージョン（rustc 1.85）を超えるものはない。
- 未記載のクレートを `use` したときのコンパイラのヒントは ``use `cargo add regex` to add it to your `Cargo.toml` `` で、学習者がアプリの中ではできない操作を案内する（仕様どおりの組み立てでも同じ）。

### 検証のあとの判断

検証のあと、ユーザーは、残った判断を Claude の推奨に任せた。Claude は次のように決め、仕様に書いた。

- 方式: 案 b にした。全許可クレートに依存する空のライブラリ `allcrates/`（`progrust_allowed_crates`）に、warmup・project・validate のどれもが path 依存する。同梱の `Cargo.lock` は、このライブラリをルートにして生成する（[ADR 0097](../decisions/0097-share-crate-builds-via-allcrates-library.md)）。
- warmup の `src/main.rs` は `fn main() {}` にした（Issue #33）。`--locked` と `--frozen` を付けないことを仕様に書いた（ADR 0097）。
- `crates_hash` に rustc の版を含めるかは、`prepared.json` に rustc の版を別の項目として持たせ、変わったら初回準備をやり直すことにした（[ADR 0100](../decisions/0100-redo-preparation-when-rustc-changes.md)）。
- cargo の自動削除でソースが消えた場合は、`--offline` のビルドの失敗の文から判定して、初回準備を自動でやり直すことにした（[ADR 0105](../decisions/0105-detect-lost-crates-from-offline-build-errors.md)）。

## 判定

基準は「許可クレートの一部だけに依存する演習で、依存を作り直さずにビルドが終わる」である。

- 今の一覧（rand だけ）では満たす。
- 想定の一覧では、仕様どおりの組み立てだと満たさない。feature の組み合わせごとの初回だけ、1〜8 秒の作り直しが起きる。オフラインでも失敗はしない。
- 案 b（全許可クレートに依存する空のライブラリへの path 依存を足す）なら満たす。全組み合わせで、build と test の両方の作り直しが 0 だった。
- `Cargo.lock` の不一致はエラーにならない。

ADR 0011 の「ビルド結果を全演習で共有する」は、一覧が 2 つ以上になると、仕様どおりの組み立てでは組み合わせごとの初回を除いて成り立つ。

## 残った事項

確かめられなかった項目:

- 案 b で、同梱の `Cargo.lock` のルートの名前とライブラリの名前が違う場合の動作は確かめていない。
- 案 e（レッスンの組み合わせごとに warmup でもビルドする）は試していない。
- 学習者が `rustup update` した後は、`crates_hash` が `allowed-crates.toml` と `Cargo.lock` だけのハッシュで rustc のバージョンを含まないため、初回準備がやり直されないと考えられる。rustc のバージョンはビルド単位のハッシュに入るので、`target/` の依存は全部作り直しになり、最初の実行が長くかかる。ソースは `CARGO_HOME` にあるので、オフラインでも失敗しない。この Mac に別の toolchain を入れられないので未確認（cargo の仕組みからの推定）。`crates_hash` に `rustc -vV` を含めるかどうかを検討する余地がある。
- cargo の全体キャッシュの自動削除（1.88 以降の stable。ネットワークから取ったファイルは 3 か月使わないと削除される。`--offline` の実行中は削除しない）で、許可クレートを使う演習を長く使わない間に、ソースが消えることがありうる。仕様の「依存が見つからないエラーになったら初回準備をやり直す」で回復できるが、回復にはネットワークが要る。確認していない。

ユーザーの判断が要るとした点は、検証のあとに決まった（「検証のあとの判断」）。決める前の推奨は次のとおり。

1. 方式を選ぶ。
   - 案 b: 空のライブラリ `progrust_allowed_crates` への path 依存を project と validate に足す（推奨。作り直しが 0 で、使えるクレートは演習の `dependencies` のまま）。
   - 案 d: 組み合わせの初回だけ数秒遅いのを受け入れる。
   - 案 a: 毎回全クレートを書く（演習の `dependencies` の意味が変わる）。

仕様の変更が要りそうな点（提案）。手順 1 の `fn main() {}`、案 b の組み立て、`--locked`・`--frozen` を付けないことは、ADR 0097 で仕様に書いた。`cargo test --no-run` は残した。

- [許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)「初回準備」の手順 1: 空の `src/main.rs` では `cargo build` が E0601 で失敗する。`fn main() {}` を置くと書く必要がある。
- 案 b を採るなら、[project/ の組み立て](../spec/exercise-runner/project-assembly.md)の `Cargo.toml` に、全許可クレートに依存するライブラリへの path 依存の行を足す。作業領域の構成にそのライブラリ（例: `allcrates/`）を加え、warmup もそのライブラリに依存させる。
- 案 b を採らないなら、ADR 0011 の「ビルド結果を全演習で共有する」に、組み合わせごとの初回は作り直しが起きることを書く。
- `--locked` と `--frozen` を付けないことを仕様に明記するとよい。同梱の lock はルートの名前も entry も演習と一致しないので、付けると必ず失敗する。
- 仕様の手順 2 の warmup の `cargo test --no-run` は、依存のビルドを増やさない。消しても困らないが、ルートの test のビルドが通ることを確かめる意味はある。
