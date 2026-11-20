---
type: Verification Report
title: "ビルドの時間と初回準備のやり直しの技術検証"
description: "実行のたびに project/ を作り直すときのアプリ側の処理時間、--offline のビルドが失敗する状態ごとの終了コードと stderr の文、cargo のキャッシュ自動削除（gc）が既定で有効なこと、許可クレートを増やしたときの初回準備の時間とディスク使用量を Issue #18 で検証した結果を記す。"
---

- 日付: 2026-11-21
- Issue: [#18](https://github.com/Progrust/progrust-app/issues/18)
- 確かめた OS: macOS 27.2.1（Apple Silicon、APFS）
- 関連仕様: [プロジェクトの組み立て](../spec/exercise-runner/project-assembly.md)、[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)、[性能](../spec/non-functional/performance.md)、[ADR 0011](../decisions/0011-prepare-crates-on-first-launch.md)

## 環境

- macOS 27.2.1、Apple Silicon、8 コア、APFS
- rustc 1.99.1（58cf8f9c3 2026-10-13）、cargo 1.99.1（5f94df478 2026-08-27）。toolchain は stable の 1 つだけ。`rustup update` は実行していない
- 専用の CARGO_HOME を作って使い、`~/.cargo` は変えていない。`cargo` の実行時は `CARGO_*`・`RUSTFLAGS` を取り除いた
- 許可クレートの一覧は 2 つ。「今の一覧」は rand 1 つ（`=0.9.1`、8 パッケージ）。「想定の一覧」は rand、serde(derive)、serde_json、regex、itertools、thiserror、anyhow、tokio(full)、futures、tokio-stream（63 パッケージ。版と Cargo.lock は [許可クレートのビルド結果の共有の技術検証](issue-12-reuse-prepared-crates.md)と同じ）
- 測定中の load average は 1.8〜17。Spotlight の `mds_stores`（CPU 約 330%）が作業ディレクトリのファイルを索引していたため。時間には余裕を持たせて読むこと

## 方法

- 初回準備（warmup）: `fn main() {}` の `src/main.rs` を置き、`cargo build` → `cargo test --no-run` を、空の CARGO_HOME・空の target から 3 回測った。
- 組み立ての試作（Rust、std のみ）: 仕様どおり、実行のたびに `project/` を削除し、`Cargo.toml`（`allowed-crates.toml` の指定クレートの行）を書き、`Cargo.lock` をコピーし、エディタの全ファイルを `src/` 以下に書く。パスは `src/` 以下の `.rs` に限り、`..` と絶対パスを拒否する。`Instant` で組み立てだけを測る。オプションで、続けて `cargo build --offline --message-format=json-diagnostic-rendered-ansi` と生成バイナリの実行も測る。

```rust
if project.exists() { fs::remove_dir_all(&project)?; }
fs::create_dir_all(project.join("src"))?;
fs::write(project.join("Cargo.toml"), toml)?;
fs::copy(runner.join("Cargo.lock"), project.join("Cargo.lock"))?;
for (p, body) in files { validate_rel(p)?; fs::write(project.join(p), body)?; }
```

- 演習は、std 1 ファイル、std 5 ファイル、rand、tokio+serde_json+serde、想定の一覧の全 10 クレートの 5 種類。
- 失敗の再現: 初回準備の直後の状態を APFS のクローンで保存し、壊し方を 1 つずつ適用して、`cargo build --offline` の終了コードと stderr を記録した。

## 結果

### project/ の作り直しとビルドの時間

組み立て（`project/` の削除とファイルの書き出し）だけの時間。200 回 × 3 セットの中央値 / p95 / 最大（ms）。load は 6.8〜7.2。

| 演習 | release 版の試作 | debug 版の試作 |
|---|---|---|
| std、1 ファイル | 0.53〜0.61 / 0.71〜0.79 / 最大 1.11 | 0.54〜0.55 / 0.73〜0.77 / 最大 0.95 |
| std、5 ファイル | 0.85〜0.89 / 1.15〜1.32 / 最大 1.63 | 0.86 / 1.16〜1.25 / 最大 3.74 |
| rand を使う | 0.52〜0.55 / 0.69〜0.84 / 最大 0.99 | 0.53〜0.54 / 0.72〜0.73 / 最大 1.17 |
| tokio+serde_json+serde を使う | 0.53〜0.65 / 0.70〜0.91 / 最大 2.04 | 0.53〜0.54 / 0.70〜0.72 / 最大 0.98 |

- 組み立ては 1ms 未満（最大でも 4ms 弱）で、基準の 500ms の 1/100 以下。許可クレートの有無や数の影響はほぼない（63 パッケージの `Cargo.lock` でも約 18KB のコピー）。
- 試作に含めなかった処理（cargo の起動、JSON 出力の解析、画面への通知、`allowed-crates.toml` の構文解析）は測っていない。

cargo のビルドの時間（依存はすべて Fresh）と、生成したバイナリの実行時間。load は 4〜13。

| 演習 | 2 回目以降の cargo build（ms） | 最初の 1 回（ms） | バイナリの実行（ms） |
|---|---|---|---|
| std | 133〜167（5 ファイルでも 150〜161） | 310 | 197〜221 |
| rand（今の一覧） | 167〜177 | 241 | 214〜251 |
| rand（想定の一覧） | 172〜179 | 1496（依存 7 個を作り直し） | 210〜230 |
| tokio+serde_json+serde（想定の一覧） | 198〜208 | 901（依存 3 個を作り直し） | 256〜277 |
| 想定の一覧の全 10 クレート | 213〜221 | 268 | 257〜272（外れ値 760 が 1 回） |

- 最初の 1 回が遅いのは、feature の違いで依存を作り直したため（[許可クレートのビルド結果の共有の技術検証](issue-12-reuse-prepared-crates.md)で見つけたもの）。同じ演習の 2 回目以降は Fresh。
- `Hello` 程度のバイナリの実行にも 200ms 強かかった。毎回ビルドし直した実行ファイルの初回起動の時間と見られる（macOS が起動前に検査していると推測。切り分けていない）。判定がテストケースごとに実行ファイルを起動するなら、ケース数 × この時間が掛かりうる。

### --offline のビルドが失敗する状態と、検出に使える文

今の一覧（rand）の初回準備の直後を元に、状態を壊して `cargo build --offline` を実行した。

| 状態の作り方 | exit | 時間 | stderr の要点 |
|---|---|---|---|
| 何もしない | 0 | 0.06s | `Finished` |
| `registry/cache` と `registry/src` を削除（target は残す） | 101 | 0.06s | `error: failed to download \`cfg-if v1.0.5\`` / `Caused by: attempting to make an HTTP request, but --offline was specified` |
| `registry/src` だけ削除 | 0 | 0.37s | 成功（`.crate` から展開し直す） |
| `registry/cache` だけ削除 | 101 | 0.06s | 上と同じ `failed to download` |
| `registry/` 全体、CARGO_HOME 全体、`registry/index` のどれかを削除 | 101 | 0.05s | `error: no matching package named \`rand\` found` / `location searched: crates.io index` / `required by package \`progrust_exercise v0.1.0 (…)\`` / `note: offline mode (via \`--offline\`) can sometimes cause surprising resolution failures` |
| target 全体、`target/debug/.fingerprint`、`target/debug/build` のどれかを削除 | 0 | 2.5〜2.8s | 成功。`Compiling libc` … と全依存を作り直す |
| `target/debug/deps` の依存の rlib・rmeta を削除（`.fingerprint` は残す） | 0 | 1.08s | 成功。消した依存だけ作り直す |
| `librand-*.rlib` を 0 バイトにする | 101 | 0.12s | `error[E0786]: found invalid metadata files for crate \`rand\`` / ` --> src/main.rs:1:5` / `= note: failed to mmap file '…/target/debug/deps/librand-….rlib': memory map must have a non-zero length` |
| `rustc -vV` だけ 1.100.0 に見せかけるラッパーを `RUSTC=` に指定（rustc の版の変更の簡易な再現） | 0 | 2.87s | 成功。全依存を作り直す |

初回準備のやり直しが要る状態の判定は、終了コード 101 のときの cargo 自身の stderr（`--message-format=json` の `compiler-message` ではないもの）に、次の文があるかで行える。

1. `failed to download \`` と `attempting to make an HTTP request, but --offline was specified`（`.crate` の欠落）
2. `no matching package named \`` と `location searched: crates.io index`（レジストリの索引の欠落）
3. `found invalid metadata files for crate`（E0786。target の成果物の破損）

- 1 と 2 は `--offline` を付けているため必ずこの形になる。cargo の文言は版で変わりうるので、「終了コード 101 かつ `--offline` に触れる文」を判定に使い、文言の一致は 2 つ以上の語で見るのがよい。
- 3 は学習者のコードのコンパイルエラーと同じ形（`error[E0786]` と `--> src/main.rs`）で出る。取り違えないよう、`note:` が target ディレクトリ内のパスを指していることを条件にする。別の版の rustc で作った成果物の E0514 は、cargo が先に作り直すため再現できず、未確認。
- 1 と 2 は `compiler-message` が出る前に cargo が失敗するので、学習者のコードの誤りとは区別できる。

失敗せず、静かに遅くなる状態がある。target の欠落、target の中の一部の削除（壊れていないもの）、rustc の版の変更は、`--offline` が成功して全依存を作り直すだけである（今の一覧で 2.5〜2.9s、想定の一覧で約 8s）。学習者には、実行が数秒〜十数秒止まったように見える。検出するには、JSON 出力の `compiler-artifact` で、project 自身以外のパッケージの `fresh` が `false` のものを数える（または stderr の `Compiling <依存>`）。

回復: 失敗した状態のあと、初回準備をオンラインでもう一度（`cargo build` → `cargo test --no-run`）実行すると、target が残っていれば 0.4〜0.6s で依存をダウンロードし直すだけで終わり、そのあとの project の `--offline` のビルドは成功する（依存はすべて Fresh）。target も壊れているときは、今の一覧で約 3s、想定の一覧で約 8s。初回準備のやり直しは可能。

### cargo のキャッシュの自動削除（gc）

- この版（stable 1.99.1）では、既定で有効。確かめ方は次のとおり。
  - `cargo clean gc` は nightly 専用（`error: the \`cargo clean gc\` command is unstable, and only available on the nightly channel`）で、stable では手動実行できない。設定は `cache.auto-clean-frequency`（既定 1 日）。
  - cargo の実行ファイルの文字列に `gc.auto.max-src-age`・`gc.auto.max-crate-age`・`1 month` がある。src は約 1 か月、`.crate` は約 3 か月使われないと消えると思われる（値ごとの検証はしていない）。
  - 実際の動作の確認: グローバルキャッシュの DB（`CARGO_HOME/.global-cache`、SQLite）の最終使用時刻と `last_auto_gc` を 120 日前に書き換えてから実行した（想定の一覧の CARGO_HOME。project は rand だけに依存）。
- オンラインの `cargo build` では、`registry/cache` の `.crate` と `registry/src` が各 52 から 8 に減った（今回使った rand の依存 8 個は最終使用時刻が更新されて残った）。`last_auto_gc` も更新された。
- `cargo build --offline` では、自動削除が動かなかった（52 のまま、`last_auto_gc` も古いまま）。`CARGO_CACHE_AUTO_CLEAN_FREQUENCY=always` を付けても同じ。
- 削除のあとに tokio を使う演習を `--offline` でビルドすると、`error: failed to download \`bytes v1.12.2\`` / `attempting to make an HTTP request, but --offline was specified`（exit 101）で失敗した。上の 1 の文で検出できる。
- 含意: 演習のビルドは `--offline` なので、gc は動かさない。動くのは、初回準備のオンラインの `cargo build` と、学習者が既定の CARGO_HOME（仕様では `~/.cargo`）で行う他の cargo の操作。数か月使っていない許可クレート（例: tokio）の `.crate` は他の操作で消され、その演習だけが `--offline` で失敗しうる。アプリ専用の CARGO_HOME なら、アプリの初回準備以外では動かない。

### rustc の版が変わったとき

- 実機の `rustup update` と、別の toolchain は確かめていない（toolchain が 1 つだけだったため）。代わりに上の表のラッパーで、cargo が版の変更を検出した場合を再現した。
- 結果は、全依存の作り直しのあと成功し、2 回目以降は Fresh。fingerprint に rustc の版が入るため、別の版の成果物を誤って使うことはなく、E0514 も出なかった。target は新しい版の成果物が増えて 47MB から 98MB になった（古い版の成果物は消えない）。
- `rustup update` 自体は CARGO_HOME のレジストリを変えない（変えるのは `~/.rustup`）ので、レジストリが消える原因にはならないと思われる。
- 影響: 仕様の `crates_hash` には rustc の版が入っていない。`rustup update` の後も `prepared.json` は `status: ready` のままで、初回準備のやり直しは起動しない。最初の演習のビルドで全依存を作り直し（今の一覧で約 3s、想定の一覧で約 8s。tokio などが増えれば更に長い）、その間は画面に進捗が出ない。作り直しは 1 回だけ。版を上げるたびに target が 1 つ分増える（想定の一覧で約 250MB、今の一覧で約 50MB）。仕様に target を掃除する手順はない。

### 初回準備の時間とディスク使用量

warmup（空の CARGO_HOME・空の target から `cargo build` → `cargo test --no-run`）を 3 回測った。ディスクは `du -sk` を MB に換算した（1MB = 1024kB）。

| 一覧 | パッケージ数 | cargo build（秒） | 追加の test --no-run（秒） | target（build 後 / test 後） | CARGO_HOME |
|---|---|---|---|---|---|
| 今の一覧（rand） | 8 | 3.4, 3.4, 3.4 | 0.1 | 45.4MB / 46.5MB | 12.9MB |
| 想定の一覧（tokio full を含む 10 クレート） | 63 | 8.7, 8.3, 8.4 | 0.2 | 251.6MB / 252.7MB | 48.3MB |

- 各回とも大きさは同じで、時間は load が違ってもほぼ同じ。ネットワークのダウンロード（1〜2s 程度と見られる）を含み、この開発機の回線での値。
- `cargo fetch` → `cargo build --offline` に分けると、今の一覧は fetch 0.6〜0.8s + build 2.6〜2.8s、想定の一覧は fetch 1.3s + build 7.1〜7.4s。ただし `cargo fetch` は全プラットフォーム（Windows 向けなど）の依存を取得するため、CARGO_HOME が 21.5MB / 77.0MB と大きくなる。仕様の warmup は `cargo build` のみなので、表の値になる。
- `cargo test --no-run` が増やすのは、空のテストの実行ファイル 1 つ（約 1MB）だけ。依存は dev プロファイルを共有する。release ビルドは作っていない（仕様にない）。
- 実行のたびに project を作り直しても target は増えない。30 通りの異なるソースを続けてビルドしても、std の target は 1.3MB のまま、tokio を使う target も増減なし（339.6MB から 338.8MB）。ただし、feature の違いで作り直した依存の成果物は target に加わる（上の 339MB と 251.6MB の差 約 80MB は、別の組み合わせの検証で作った分）。別の組み合わせの演習が増えるほど target は大きくなる。

## 判定

- アプリ側の処理時間 500ms 以内: 満たした。組み立ては 1ms 未満。cargo のビルドとバイナリの実行は基準の対象外。他の処理（cargo の起動、JSON の解析、通知）は試作に含めておらず、実装で確かめる。
- 初回準備をやり直す条件を検出できる: 一部だけ確かめた。レジストリの欠落（`.crate`・索引）と成果物の破損（E0786）は、`--offline` のビルドの失敗の文で検出できた。target の欠落と rustc の版の変更は、失敗せず静かに全部作り直す。実機の `rustup update` と、別の版の rustc で作った target は未確認。
- 初回準備の時間は、今の一覧で約 3.5s、想定の一覧で約 8.5s。ディスクは、想定の一覧で約 300MB（target 253MB + CARGO_HOME 48MB）、今の一覧で約 60MB。仕様に数値の基準はなく、いずれも実用的な範囲と見る。

## 残った事項

ユーザーの判断が要る点（Issue は開いたままにする）:

- 初回準備のやり直しの条件に rustc の版を加えるか。加えるなら、`prepared.json` に `rustc -vV` の出力（または `commit-hash`）を持たせ、起動時に今の rustc と比べて違えばやり直す（warmup を先に実行すれば、最初の演習で待たされない）。加えない場合の影響は上のとおり（最初の演習が数秒遅くなる、target が増える）。
- cargo の gc の影響をどう扱うか。数か月使っていない許可クレートで `--offline` のビルドが失敗する。案は、(a) 失敗の文（上の 1・2）を検出して自動で初回準備を再実行する（ネットワークがあるときだけ）、(b) アプリ専用の CARGO_HOME にする（学習者の他の cargo の gc の影響を受けない。仕様は学習者の既定の CARGO_HOME を使うので、ADR 0011 の判断と変わる）、(c) 起動時に `registry/cache` に全依存の `.crate` があるかを確かめる。

確かめられなかった項目:

- 実機の `rustup update` と、別の版の rustc で作った target。E0514 の実際の出方。
- cargo の gc の期間（src 1 か月、`.crate` 3 か月）の個別の検証（120 日で両方が消えることだけ確認した）。
- release プロファイルのビルドの大きさ・時間。
- アプリ側の他の処理（cargo の起動、JSON の解析、イベント通知）を含めた実測。

仕様の変更案:

- 「依存が見つからないエラーになった場合は初回準備をやり直す」に、上の実際の文を明記し、検出の対象に E0786 を加える。target の欠落と rustc の版の変更は失敗にならないので、この方法では検出できない。
- `prepared.json` に rustc の版を持たせる案（上のとおり）。
- クレートのダウンロード先を学習者の既定の CARGO_HOME とする仕様に、gc で `.crate` が消えたときの復旧（自動の再実行）を書く。
- 初回準備の `cargo test --no-run` は、空の `main.rs` では約 0.1〜0.2s・約 1MB しか増やさない。省略しても依存のビルドは変わらない（空の `fn main() {}` が必要な点は変わらない）。
- `Cargo.lock` のルートのパッケージ名が違うと、`--locked` 付きの `cargo fetch` や build は失敗する。仕様は `--locked` を使っていないので問題ないが、付けるならルートのパッケージ名を `progrust_exercise` にそろえる。

検証中に見つけた別の問題:

- 作り直したバイナリの初回起動が 200ms 強かかる。stdout 方式で複数のケースを実行するときの 2 回目以降の起動の時間は未測定。
- Spotlight が作業ディレクトリのビルド成果物を索引して CPU を大量に使った。学習者の `{app_data_dir}/workspace` も対象になりうる（学習者の Mac のビルドを遅くしうる）。`.metadata_never_index` を置くなどの対策の余地がある（未確認）。
