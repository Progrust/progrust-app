---
type: Verification Report
title: "検証と学習者の実行の並行ビルドの技術検証"
description: "問題生成の検証と学習者の実行が互いを待たせずに並行できるかを Issue #13 で検証した結果（同じビルド先でのロックの待ち、同じパッケージ名で相手の実行ファイルを使ってしまう誤り、初回準備中のパッケージキャッシュのロック、APFS のクローンなどでビルド先を分ける方法の比較）を記す。"
---

- 日付: 2026-11-20
- Issue: [#13](https://github.com/Progrust/progrust-app/issues/13)
- 確かめた OS: macOS 27.2.1（Apple Silicon、APFS）。Windows は確かめていない
- 関連仕様: [実行基盤の概要](../spec/exercise-runner/overview.md)、[許可クレートと作業領域](../spec/exercise-runner/crates-and-workspace.md)、[出力と検証](../spec/exercise-generation/output-and-validation.md)、[ADR 0012](../decisions/0012-separate-build-dir-without-crates.md)、[ADR 0034](../decisions/0034-validate-generated-exercises.md)

## 環境

- macOS 27.2.1（Build 26B131）、Apple Silicon（aarch64）、8 コア、メモリ 16 GB、APFS
- rustc 1.99.1（58cf8f9c3 2026-10-13）、cargo 1.99.1（5f94df478 2026-08-27）、stable チャネル
- `~/.cargo/config.toml` なし。環境変数 `CARGO_*`・`RUSTFLAGS` の設定なし。sccache なし
- `CARGO_HOME` は、他のエージェントとパッケージキャッシュのロックを取り合わないように検証用の専用ディレクトリにした（初回準備中の待ちを測るときは空のディレクトリ）
- 他のエージェントが同時にビルドしていたため、load average は 4〜19 で変動した。時間は 3〜5 回測った。各測定に、そのときの load average を書く

## 方法

許可クレートは、仕様の `rand` に加え、10 クレートの想定の一覧（`rand`、`serde`、`serde_json`、`regex`、`itertools`、`thiserror`、`anyhow`、`tokio`、`futures`、`tokio-stream`。詳細は[許可クレートのビルド結果の共有の技術検証](issue-12-reuse-prepared-crates.md)）を使った。warmup（初回準備）で `target/` に全許可クレートをビルドしてから、次を試した。

- 2 つの cargo を同じビルド先に向け、A（検証）を起動してから 0.05〜0.5 秒後に B（学習者）を起動した。B の経過時間と、標準エラー出力の `Blocking waiting for file lock ...` の行を記録した。
- 待ち時間を決まった長さにするため、A に重い定数評価を入れた。毎回ループの回数を変えて、インクリメンタルキャッシュに当たらないようにした。待たされる側の B は、普通の小さな演習にした。

```rust
#![allow(long_running_const_eval)]
const X: u64 = { let mut i = 0u64; let mut s = 0u64; while i < 2_500_000 { s = s.wrapping_add(i * i); i += 1; } s };
fn main() { println!("validate (slow) {}", X); }
```

- `project/`（学習者）はパッケージ名 `progrust_exercise`、`validate/`（検証）も仕様どおり同じ `progrust_exercise` で組み立てた。
- ビルド先の複製は、warmup の直後の `target/` を複製して、B の依存が Fresh になるかを見た。

## 結果

### 同じビルド先を使うと待たされる

単独で測った基準は、学習者の `cargo build`（依存は Fresh）が 0.13〜0.14 s、`cargo test --no-run` が 0.13〜0.14 s（各 5 回、load 8.4）。

A が重い検証のビルド（約 6 s）のとき、0.5 s 後に B を起動すると、B は A が終わるまで待たされた。

```
load=8.43 {"A": {"elapsed": 6.0}, "B": {"elapsed": 5.47, "blocking": ["Blocking waiting for file lock on build directory"]}}
load=7.84 {"A": {"elapsed": 6.06}, "B": {"elapsed": 5.53, "blocking": ["Blocking waiting for file lock on build directory"]}}
load=6.86 {"A": {"elapsed": 6.02}, "B": {"elapsed": 5.49, "blocking": ["Blocking waiting for file lock on build directory"]}}
```

A が普通の小さな検証のビルド（約 0.26 s、依存は Fresh）のとき、0.05 s 後に B を起動すると、B は毎回待たされた。B の経過時間は 0.17〜0.24 s で、中央値は build と test --no-run のどちらも 0.18〜0.19 s（各 5 回、load 4.1〜4.3）。単独より 0.05〜0.1 s 遅い。

- 待ち時間は A の cargo の残りの時間で決まる。A の検証が、その feature の組み合わせを初めて使う（1〜8 s。[許可クレートのビルド結果の共有の技術検証](issue-12-reuse-prepared-crates.md)）、warmup の規模のビルドをする、のような処理を含むと、その間 B は待たされ続ける。
- ロックを持つのは cargo の実行中だけである。検証が実行ファイルを動かしている間（テストケースの実行）は、ロックを持たない。
- ロックの待ちは、cargo の標準エラー出力の `Blocking waiting for file lock on build directory`（パッケージキャッシュなら `on package cache`）で分かる。仕様では cargo の進捗のメッセージを表示しないので、学習者には何もせずに止まっているように見える。

### 同じパッケージ名で同じビルド先を使うと、相手の実行ファイルを使ってしまう（重大）

これはロックの待ちとは別の、正しくない結果になる問題である。2 つの形がある。(1) の方が、起きる範囲が広い。

(1) 実行ファイルの置き場所が同じなので、後のビルドが前のビルドの実行ファイルを置き換える。

- 実行ファイルの置き場所は `target/debug/<パッケージ名>` で、ハッシュが付かない。依存が違っていても、両方が正しくコンパイルされても、後からビルドした方で置き換わる。
- stdout 方式の判定は、テストケースごとに同じパスの実行ファイルを起動する。判定の途中で相手のビルドが入ると、途中から相手のプログラムで判定することになる。

```
== 依存が同じ（rand）。両方とも Dirty → Compiling で正しくコンパイルされる
学習者がビルドして 1 回目の実行: LEARNER
（validate を組み立ててビルド: Dirty → Compiling progrust_exercise (validate)）
学習者が同じパスで 2 回目の実行: VALIDATE
== 依存が違う（project は rand、validate は itertools）
1 回目: LEARNER-rand → validate のビルドの後の 2 回目: VALIDATE-itertools
== target-std/（依存なし）
1 回目: LEARNER-std → validate のビルドの後: VALIDATE-std
```

(2) 鮮度の判定を誤り、Fresh と判定されて相手の実行ファイルがそのまま使われる（依存が同じ場合）。

- `project/` と `validate/` は、どちらもパッケージ名が `progrust_exercise` で、依存も同じになりうる。cargo 1.99 では、この 2 つのビルド単位のハッシュが同じになる。パッケージのディレクトリが違っても同じで、`.fingerprint/progrust_exercise-ce84ccedc20d63f2` が 1 つだけできる。実行ファイルの置き場所も同じになる（`target/debug/progrust_exercise`、test は `target/debug/deps/progrust_exercise-<hash>`）。
- cargo の鮮度の判定は、今のパッケージの `src/**` の mtime と、最後にこの単位をビルドしたとき（相手のビルド）を比べる。そのため、自分のソースを書いた後に相手がビルドすると、自分のビルドは Fresh と判定され、相手のバイナリがそのまま使われる。

順に実行した場合の再現は次のとおり。学習者の project/ をビルドし、1 秒後に validate/ を組み立ててビルドする。

```
   Compiling progrust_exercise v0.1.0 (.../workspace/project)
learner
       Fresh progrust_exercise v0.1.0 (.../workspace/validate)
validate's binary prints: learner          ← 検証が学習者のコードを実行している
```

`cargo test --no-run` でも同じで、validate を test でビルドした後に、project の test が Fresh になった。

`target-std/`（依存なし）での最小の再現コマンドは次のとおり。`asm` は project を組み立てる関数で、第 2 引数がパッケージ名、第 3 引数が許可クレート名（`NONE` は依存なし）、第 4 引数が `main.rs` の中身である。

```sh
asm(){ rm -rf $1; mkdir -p $1/src; printf '[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\n' $2 > $1/Cargo.toml; grep -E "^$3 = " $D/runner/allowed-crates.toml >> $1/Cargo.toml; cp $D/runner/Cargo.lock $1/; printf '%s\n' "$4" > $1/src/main.rs; }
asm $W/project  progrust_exercise NONE 'fn main() { println!("LEARNER"); }'
asm $W/validate progrust_exercise NONE 'fn main() { println!("VALIDATE"); }'
(cd $W/project  && CARGO_TARGET_DIR=$W/target-std cargo build --offline -q)
(cd $W/validate && CARGO_TARGET_DIR=$W/target-std cargo build --offline -v 2>&1 | grep progrust_exercise)   # → Fresh
$W/target-std/debug/progrust_exercise   # → LEARNER（validate のビルドなのに学習者のコード）
```

並行に実行した場合（学習者が project/ を組み立てた直後に検証のビルドが始まる場合）も、3 回とも学習者の実行ファイルが検証のものになった。

- 1 回目は、学習者の cargo がロックを 5.3 s 待ったあと、Fresh と判定した。
- 2・3 回目は、A がインクリメンタルキャッシュで 0.37 s で終わり、ロックの待ちはなかったが、学習者の cargo は Fresh と判定した。

```
load=4.94 {"A": {"elapsed": 5.91}, "B": {"elapsed": 5.59, "root_fresh": [true], "blocking": ["Blocking waiting for file lock on build directory"], "post": "validate (slow) 5208330208333750000"}}
```

`post` は、学習者が実行するはずの `target/debug/progrust_exercise` の出力である。学習者のコードではなく、検証のコード（模範解答や初期コード）が実行される。提出なら、学習者のコードと違うもので判定される。

- (2) は、自分のソースを書いてから cargo がビルドするまでの間に相手の rustc が始まったときだけ起きる。それ以外では Dirty と判定されて正しくコンパイルされる。ただし、その場合も (1) の置き換えは起きる。
- 同じビルド先で学習者の実行・提出と検証が重なると、(1) によって、依存の組み合わせに関係なく結果が誤りうる。std だけの演習（今の一覧ではほとんどの演習）の `target-std/` でも起きる。
- 回避は、validate/ のパッケージ名を変えること（例: `progrust_validate`）である。ビルド単位のハッシュが変わって (2) が起きなくなり、実行ファイルの置き場所も `target/debug/progrust_validate` に変わって (1) も起きなくなる。ただし、ロックの待ちはなくならない。パッケージ名は cargo のエラーの表示（``could not compile `progrust_validate` ``）にも出る。ビルド先を分けても (1)・(2) は起きなくなる。

### 初回準備中に、std だけの演習は待たずにビルドできるか

ADR 0012 は、初回準備中でも、許可クレートを使わない演習は `target-std/` で待たずにビルドできるとしている。これは一部だけ成り立つ。

空の `CARGO_HOME` で warmup を始め、0.8 s 後に std だけの演習（`--offline`、`target-std/`）をビルドした。2 回とも、パッケージキャッシュのロックで待たされた（0.95 s と 0.74 s。単独なら約 0.13〜0.27 s）。

```
{"std_only_into_target-std": {"elapsed": 0.95, "blocking": ["Blocking waiting for file lock on package cache"]}, "warmup": {"elapsed": 8.73, "downloaded": 52}}
```

- `target/` のロックは避けられるが、ダウンロード中は `CARGO_HOME` のパッケージキャッシュのロックで待たされる。今回は回線が速く、待ちは 1 秒未満だった。回線が遅いとダウンロードが終わるまで待たされると考えられるが、遅い回線は再現していない。
- std だけの演習を、アプリ専用の空の `CARGO_HOME` でビルドすると、待たなかった（0.27 s、Blocking なし）。ただし、学習者の `~/.cargo/config.toml` は読まれなくなる。

### ビルド先を分けて、許可クレートのビルド結果を再利用する方法

| 方法 | 待ち | 正しさ | 依存の再利用 | 備考 |
| --- | --- | --- | --- | --- |
| APFS のクローンで `target/` を `target-validate/` に複製する（`cp -cRp target target-validate`） | なし（3 回とも Blocking なし。B 0.16〜1.32 s。遅い回は A の rustc と CPU を取り合った） | 正しい（学習者の実行ファイルは LEARNER を出力した） | 全クレートで Fresh（build・test とも）。`-p` を付けず、mtime を保たずに複製しても Fresh | 複製は 0.13 s（3 回とも）。ディスクの増加は約 0.3〜0.4 MB（254 MB の `target/` に対して）。普通の `cp -Rp` だと 0.74 s で容量は 2 倍。初回準備の後と、クレートの準備のやり直しの後に作り直す必要がある。Windows（ReFS 以外）にはクローンがない |
| ハードリンクで複製する（`mkdir hl-tmp && pax -rwl target hl-tmp`） | 試していない（分けた後なので待たないはず） | 正しいはず（validate の名前は変えた） | 全クレートで Fresh（build・test とも、rand と serde+serde_json の組み合わせで確かめた） | 複製は 0.25 s。新しい出力は別のファイルとして作られ、リンク数 1 になる。ただし 1 回目の確認で、`target/` 側のビルドの後に複製先の中身が変わったことがあった（`.rustc_info.json` などをその場で書き換えていると考えられる。2 回目は変わらなかった）。共有されたファイルをその場で書き換えると両方に及ぶので、書き込み時コピーのクローンの方が安全である。Windows（NTFS）でも使えるのはこの方法である（Windows 未確認） |
| `CARGO_BUILD_BUILD_DIR` を共有し、`CARGO_TARGET_DIR`（成果物の置き場）だけ分ける（stable） | 待つ（`Blocking waiting for file lock on build directory`、B 6.73 s） | 誤り（パッケージ名が同じなので、学習者の実行ファイルが validate の出力をした） | 共有できる | build-dir の分離は stable だが、ロックは build-dir 全体にかかる |
| `-Z fine-grain-locking`（nightly のみ。`RUSTC_BOOTSTRAP=1` で試した）で `target/` を共有する | 待つ（`Blocking waiting for file lock on artifact directory`、B 8.2〜9.6 s） | 名前を変えれば正しい | 共有できる | 成果物の置き場のロックが残る |
| `-Z fine-grain-locking` で build-dir を共有し、成果物の置き場を分ける。validate の名前も変える | なし（B 0.14〜0.38 s） | 正しい | 共有できる | stable では使えない（1.99.1 でも unstable）。名前が同じままだと、単位ごとのロック（`Blocking waiting for file lock on progrust_exercise (ce84…)`）で待ち、結果も誤る |
| `--artifact-dir`（成果物のコピー先） | 試していない | — | — | 1.99.1 でも unstable |
| sccache（`RUSTC_WRAPPER`） | 試していない | — | — | ドキュメントによると、リンカーを呼ぶクレート（bin、dylib、cdylib、proc-macro）とインクリメンタルビルドはキャッシュできない。serde_derive・thiserror-impl・tokio-macros などの proc-macro は再利用できない。別のツールを学習者の環境に同梱する必要もある。そのため採らない |

クローンでの複製を再現するコマンドは次のとおり。

```sh
time cp -cRp $W/target $W/target-validate   # 0.1 秒ほど。依存は Fresh のまま使える
```

ロックの待ちの再現コマンドは次のとおり（validate はパッケージ名を `progrust_validate` にして、重い定数評価を入れる）。

```sh
(cd $W/validate && CARGO_TARGET_DIR=$W/target cargo build --offline -q &) ; sleep 0.5
(cd $W/project && time CARGO_TARGET_DIR=$W/target cargo build --offline 2>&1 | grep Blocking)
# → Blocking waiting for file lock on build directory
```

複製（クローン・ハードリンク）には注意がある。

- 複製は、複製した時点のスナップショットである。仕様どおりの組み立てだと、組み合わせの初回の作り直しが、`target/` と `target-validate/` のそれぞれで別々に起きる。rand の演習で試すと、`target/` 側で 6 個、`target-validate/` 側で 7 個の作り直しが起き、それぞれ 1.27 s と 1.26 s かかった。
- 複製で作り直しがなくなるのは、[許可クレートのビルド結果の共有の技術検証](issue-12-reuse-prepared-crates.md)の案 b（全許可クレートに依存する空のライブラリへの path 依存）と組み合わせたときだけである。

## 判定

基準は「検証中でも、学習者の実行・提出が待たされずに始まる」である。

- 仕様どおりの構成（同じ `target/` を使う）では満たさない。B は A の cargo が終わるまで待つ。小さな検証では +0.05〜0.1 s、重い検証では検証のビルドの時間全部（測定では約 5.4 s）である。
- さらに、パッケージ名が同じなので、相手の実行ファイルで実行・判定してしまう誤りが起きる（重大）。実行ファイルの置き場所が同じなので、依存の組み合わせに関係なく、`target-std/` でも起きる。
- validate 用のビルド先を APFS のクローンで分けると、待たずに正しく動き、依存も再利用できる（複製 0.13 s、追加の容量はほぼ 0）。
- std だけの演習も、初回準備のダウンロード中は、パッケージキャッシュのロックで待たされる（測定では 1 秒未満）。ビルド先を分けるだけでは避けられない。
- Windows は確かめていない。

## 残った事項

確かめられなかった項目:

- Windows は確かめていない（当面 macOS に集中する方針のため）。Windows（ReFS 以外）にはクローンがなく、普通のコピーでは容量が 2 倍になる。ハードリンクは試したが、その場で書き換えられるファイルがあるため、安全とは言えない。
- 遅い回線での、初回準備中のパッケージキャッシュのロックの待ちは再現していない。
- ハードリンクでの複製後に、学習者と検証を同時に走らせる並行の測定はしていない。
- `--artifact-dir` と sccache は試していない。

ユーザーの判断が要る点（Issue は開いたままにする）:

1. 検証用のビルド先を、学習者のものと分けるかどうか。分けるなら、次のどれを使うか。
   - 初回準備の後に `target/` を `target-validate/` に複製する（APFS のクローン。Windows ではクローンが使えず、普通のコピーで容量が 2 倍）。
   - 検証専用に、もう一度依存をビルドする（初回準備が 2 倍になる）。
   - 分けずに、検証と学習者の実行を直列にする（待ちを受け入れる。その場合も、パッケージ名は分ける必要がある）。
2. 同じビルド先で、小さな検証と重なったときの +0.05〜0.1 s を「待たされない」とみなすか。
3. 初回準備のダウンロード中に、std だけの演習が待つのを許すか。許さないなら、std だけの演習のビルドにアプリ専用の空の `CARGO_HOME` を使う（待たない。ただし、学習者の `~/.cargo/config.toml` は読まれなくなる）。

仕様の変更が要りそうな点（提案）:

- [実行基盤の概要](../spec/exercise-runner/overview.md)の「生成問題の検証は学習者の実行と並行して行える」と ADR 0012 は、同じビルド先では成り立たない。validate/ のパッケージ名を project/ と変える（例: `progrust_validate`）ことと、検証用のビルド先を分ける（`target-validate/`、`target-std-validate/` など）か並行できるという記述を改めることを、仕様に書く必要がある。
- ADR 0012 の「初回準備中でも許可クレートを使わない演習をすぐに実行できる」は、ダウンロード中のパッケージキャッシュのロックで成り立たない。
- 未記載のクレートを `use` したときのコンパイラのヒントが、学習者がアプリの中ではできない操作（`cargo add`）を案内する。
