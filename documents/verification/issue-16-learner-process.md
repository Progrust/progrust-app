---
type: Verification Report
title: "学習者のプログラムの子プロセスとしての扱いの技術検証"
description: "学習者のプログラムを子プロセスとして扱えるかを Issue #16 で検証した結果（標準出力・標準エラー出力の受け取り、標準入力の送信と閉じ方、プロセスグループへの killpg による停止、1MB の出力量の上限、run_output イベントの送り方と表示量の上限、グループから抜けた子が残ること）を記す。"
---

- 日付: 2026-11-21
- Issue: [#16](https://github.com/Progrust/progrust-app/issues/16)
- 確かめた OS: macOS 27.2.1（Apple Silicon、arm64）。Windows は確かめていない（当面 macOS に集中する方針のため。Job Object は未確認）
- 関連仕様: [コードの実行（run）](../spec/exercise-runner/run.md)、[実行・判定の制限値](../spec/exercise-runner/limits.md)、[フロントエンドとバックエンドの役割分担](../spec/architecture/responsibilities.md)、[演習画面](../spec/screens/exercise-screen.md)、[ADR 0013](../decisions/0013-interactive-stdin.md)、[ADR 0014](../decisions/0014-run-learner-code-without-sandbox.md)、[ADR 0045](../decisions/0045-judge-result-above-problem-collapsed.md)

## 環境

| 項目 | 値 |
|---|---|
| OS | macOS 27.2.1、Apple Silicon（arm64） |
| rustc / cargo | 1.99.1 / 1.99.1 |
| Tauri | tauri 2.12.7（crate）、tauri-cli 2.12.6、@tauri-apps/api 2 |
| Node.js | v25.9.0 |
| 画面 | WKWebView（待機中の requestAnimationFrame の間隔は 17ms、60Hz） |

## 方法

二つの試作を作った。

1. 子プロセスの試作（プレーンな Rust の CLI。以下、ランナー）。学習者のプログラムを起動し、出力を受け取り、標準入力を送り、止める。学習者のプログラムに見立てた 25 個のプログラム（`print!` の flush の有無、`BufWriter`、スレッド、`Command` や `fork()` で作る子、`setsid()` や `process_group(0)` で抜ける子、二重 fork、無限出力、`loop {}`、panic、abort など）を動かした。
2. 表示の試作（最小の Tauri v2 アプリ、素の TypeScript）。1 のランナーで 1MB を出力するプログラムを動かし、`run_output` イベントで WKWebView に送り、ページ自身が表示し終わるまでの時間と、画面が描かれなかった最長の時間（`requestAnimationFrame` の最大間隔）を測った。送り方 4 通り × 表示 6 通り × 出力 4 種類を測った（`.app` にして前面に出し、各 5 回前後。描画が止まった回は除いた）。

仕様どおり、`cargo build` でできた実行ファイルを直接起動した。`cargo run` 経由とビルド中の停止も比べた。ランナーの決め手になる部分は次のとおり。

```rust
// 起動: 新しいプロセスグループを作る（pgid = 子の pid）
cmd.process_group(0).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
let mut child = cmd.spawn()?;

// 停止: グループ全体に SIGKILL。自分のグループを殺さない
fn kill_group(pgid: i32) -> i32 {
    let me = unsafe { libc::getpgrp() };
    assert!(pgid > 1 && pgid != me);
    unsafe { libc::killpg(pgid, libc::SIGKILL) }
}

// 出力量: stdout・stderr それぞれの読み取りスレッドで read()（8KB）し、合計を数える。
// 1MB を超えた時点で、その場で killpg
let before = total.fetch_add(n, Ordering::SeqCst);
if before + n > cap { ctl.request_kill(StopReason::OutputLimit); }

// 終了の検出: 刈り取らずに終了を知る（pgid が再利用されない）。
// その間にグループの残りを killpg で止め、残りの出力を最大 300ms 待ってから child.wait()
libc::waitid(libc::P_PID, pid as _, &mut info, libc::WEXITED | libc::WNOWAIT);

// 制限時間は専用のスレッドで見る
thread::spawn(move || { thread::sleep(timeout); ctl.request_kill(StopReason::Timeout); });
// 監督のスレッドでは、停止・タイムアウトの後にキューに残った出力を捨てる
Msg::Data(s, b) => {
    if matches!(ctl.reason(), Some(StopReason::User) | Some(StopReason::Timeout)) { dropped_after_stop += b.len(); continue; }
    ...
}
```

- 標準入力は専用のスレッドで書く（相手が読まないと書き込みがブロックするため）。
- 読み取りスレッドと監督のスレッドの間は有限のチャネル（`sync_channel(64)`）にした。画面への送信が遅いとパイプが詰まり、子の `write` が待つ（端末と同じ流量制御）。
- UTF-8 の文字がチャンクの境目で切れたら、次のチャンクまで持ち越す。1MB の上限で切れた文字は捨てる。
- 比較用に、親子関係をたどって子孫を SIGSTOP で凍らせてから SIGKILL する `kill_tree` も作った。

## 結果

### 標準出力・標準エラー出力を少しずつ受け取れるか: 確かめた（OK）

`println!` の行は書かれた時点で届く（300ms 間隔で書くと 315ms、619ms、…に届いた）。改行のない `eprint!` もすぐ届く。

- Rust の `stdout` は、パイプでも端末でも行バッファ。`println!` は行ごとに届く。
- `print!("名前を入力: ")` の後に `flush()` しないと、プロンプトは届かない。`stdin().read_line` は stdout を flush しない。端末（pty）でも同じで、プロンプトは入力の後に出る。アプリが端末と違う動きをしているわけではない。`BufWriter` は終了時（または 8KB たまったとき）にまとめて届く。


- `eprintln!` は 1 行が複数回の `write` に分かれることがある。受け取る側は、行の途中で切れたチャンクを前提にする。改行まで待つ作りにすると、flush したプロンプトや改行のない出力が届かなくなる。
- stdout と stderr の前後関係は保てない。別々のパイプなので、out 0, err 0, out 1, err 1, … の順に書いても、`out 0〜2` → `err 0〜3` → `out 3〜4` → `err 4` のように届いた（端末では 1 つの tty なので順序どおりに見える）。
- panic は終了コード 101、`abort()` はシグナル 6 で終わり、どちらも `runtime_error` と判定できた。

### 対話的な標準入力を 1 行ずつ送れるか、閉じられるか: 確かめた（OK）

`p16_echo`（300/600/900ms に 1 行ずつ送り、1500ms に閉じる）では、各行が 1ms 以内に返り、閉じた 0.2ms 後に `read_line` が 0（EOF）を返した。

- 送った 1 行は 1ms 以内に受け取られた。日本語も問題ない。
- 終了した子に書くと `write` が `BrokenPipe` を返すだけで、アプリは落ちない（Rust のプロセスは起動時に SIGPIPE を無視する）。
- 読まない子に 1MB を書くと `write` がブロックするが、専用のスレッドで書くので停止ボタンなどは止まらない。停止するとブロックしていた `write` も `BrokenPipe` で戻った。
- 標準入力を待つ間も実行時間に数えられ、制限時間で `timeout` になった（ADR 0013 のとおり）。

### 停止・タイムアウト・出力量の上限で、子プロセスまでまとめて止められるか: 条件付き OK

`process_group(0)` で起動して `killpg(pgid, SIGKILL)` で止め、`ps -axo pid,ppid,pgid,sess,stat,command` で 0ms・200ms・1200ms 後に残りを見た。

| プログラム | 結果 | 停止後に残ったもの |
|---|---|---|
| スレッド 4 本、`Command` の子 3 つ、`fork()` 3 回 | 停止・タイムアウト（1 秒）・出力の上限のどれでも、子もすべて終了 | なし（ゾンビも残らない） |
| 子を残して終了コード 0 で終わる | `success`。親の終了時にグループの残りを killpg で止めた | なし |
| `loop {}`（制限 60 秒） | 60001.6ms で `timeout` | なし |
| `cargo run` 経由 | `stopped`。学習者のプログラムとその子まで止まった | なし |
| `cargo build` 中（rustc が動いている） | `stopped`。cargo と rustc がまとめて止まり、再ビルドも成功 | なし |
| `process_group(0)`・`setsid()` で抜けた子（パイプを持ったまま、または `/dev/null` のデーモン風） | 親は `stopped`。抜けた子は止まらない | 抜けた子が残る（親が PID 1 に変わる） |

```
== stop: p16_escape_pg
    94427 94426 94427  S  .../p16_escape_pg
    94428 94427 94428  S  .../p16_sleeper 60        ← pgid が親と違う
  [ps after finish +1200ms] 1 proc(s)
    94428     1 94428  S  .../p16_sleeper 60        ← 残った
```

- macOS の `cargo run` は、ビルドの後に cargo 自身を学習者のプログラムに置き換える（exec）。起動した PID がそのまま実行ファイルになっていて、cargo のプロセスは残らない。
- グループから抜けた子は、端末で Ctrl+C を押した場合も残った（端末の Ctrl+C は前面のプロセスグループにだけ SIGINT を送る。pty で確認）。killpg だけでも「学習者が自分のターミナルで `cargo run` するのと同等」（ADR 0014）は保てている。
- 抜けた子が stdout のパイプを持ったままだと、読み取り側に EOF が来ない。そのためランナーは EOF を待たず、「親の終了 + 最大 300ms」で実行を終える（停止から 310ms で `run_finished`）。ただし、その読み取りスレッドは抜けた子が終わるまで残る。
- 親子関係をたどる `kill_tree` を足すと、親が生きている間に抜けた子は止められた。二重 fork（子が孫を作ってすぐ終わり、孫が `setsid()`）は、孫が止める前に PID 1 の子になっているので止められなかった（`kill_tree` が見つけたのは終わった中間の子だけ）。macOS には Linux の `PR_SET_CHILD_SUBREAPER` に当たる仕組みが見当たらない（ドキュメントの範囲で調べた。未確認）。

#### 画面への送信が遅いとき

出力 1 回ごとに 100ms 眠らせて画面への送信を遅くし、無限に出力するプログラムを止めた。

- 最初の版は、制限時間を `recv_timeout` で見て、キューに残った出力も送り切っていた。停止から `run_finished` まで 7.3 秒遅れ、さらにキューが空にならないのでタイムアウトが効かず、120 秒たっても終わらなかった。
- 制限時間を専用のスレッドで見て、停止・タイムアウトの後にキューに残った出力を捨てる版（上の抜粋）では、どちらも送信中だった 1 回分（100ms）以内に終わった。

### 出力量の上限（1MB）で止まるか: 確かめた（OK）

stdout と stderr の合計で数え、1,048,576 バイトを超えた時点で止めた。無限に出力する 5 種類（`"a\n"`、約 40 バイト/行、改行なし、日本語、stdout と stderr を交互）のすべてが `runtime_error`（理由 `OutputLimit`）になり、子は残らなかった。上限に達するまでは 2〜332ms。

- 読んだバイト数は 1,048,631〜1,114,112 で、画面に送ったのは常に 1,048,576。超えた分（最大 64KB 程度）は画面に送らない。
- 最初の試作は、判定を画面に送る側で行い、チャネルを無制限にしていた。送る側が遅いと子が書き続け、1MB の上限なのに 4.5MB 読んでいた。判定を読み取りスレッドに移し、チャネルを有限にして、超過が 64KB 程度に収まった。
- 日本語の上限をずらして文字の途中で切れるようにしたが、切れた 1〜2 バイトは捨てられ、置換文字（U+FFFD）は 1 つも出なかった。

### 1MB を一気に出力しても、run_output イベントで画面が固まらないか: 条件付き OK（行ごとのイベントは NG）

バックエンドの送り方（`app.emit("run_output", { run, seq, items: [{ stream, text }] })`）は 4 通り試した。

- `line`: 1 行ごとに 1 イベント
- `chunk`: `read()` 1 回（最大 8KB）ごとに 1 イベント
- `batch:16` / `batch:50`: 16ms / 50ms ごとに、たまった分を 1 イベントにまとめる（同じストリームが続く分は 1 項目にまとめ、stdout と stderr の順序は保つ）

```rust
// batch: 出力は pending にためる。同じストリームが続く間は 1 項目にまとめる
if let Some(last) = g.pending.last_mut() { if last.stream == s { last.text.push_str(&text); return; } }
g.pending.push(Item { stream: s, text });
// 別のスレッドが interval_ms ごとに、たまった分を 1 つの run_output にして送る
std::thread::spawn(move || loop {
    std::thread::sleep(Duration::from_millis(interval_ms));
    let mut g = st.lock().unwrap();
    if !g.pending.is_empty() { let items = std::mem::take(&mut g.pending); emit(items, &mut g); }
    if done.load(Ordering::SeqCst) { break; }
});
```

フロントエンドの表示は、イベントごとに `<span>` を追加する `append`、1 フレームに 1 回まとめて追加する `raf`、末尾の 64K 文字だけを描く `tail`（1 つのテキストノード。色分けなし）、`tail` と同じ範囲をストリームごとの `<span>` で色分けして描く `tailspan` などを比べた。決め手になった `tailspan` は次のとおり。

```ts
// 受け取ったら kept にためるだけ。1 フレームに 1 回、末尾の 64K 文字だけを span で描き直す
const flushTailSpan = () => {
  scheduled = false;
  while (kept.length > 1 && keptLen - kept[0].text.length >= TAIL) { keptLen -= kept[0].text.length; kept.shift(); }
  const frag = document.createDocumentFragment(); let skip = Math.max(0, keptLen - TAIL);
  for (const it of kept) {
    let t = it.text; if (skip > 0) { const k = Math.min(skip, t.length); t = t.slice(k); skip -= k; }
    if (!t) continue;
    const sp = document.createElement("span"); if (it.stream !== "stdout") sp.className = it.stream; sp.textContent = t; frag.appendChild(sp);
  }
  con.replaceChildren(frag); con.scrollTop = con.scrollHeight;
};
```

「止まった」の目安は、`requestAnimationFrame` の最大間隔が 100ms を超えたかどうかにした。主な結果（rAF の最大間隔の中央値 / 最悪、ms）は次のとおり。

| 出力 | 送り方 + 表示 | イベント数 | rAF 最大間隔 |
|---|---|---|---|
| `"a\n"`（52 万行） | `line` + 受け取るだけ | 524,289 | 192 / 1037（実行〜表示に 13.5 秒） |
| `"a\n"` | `line` + `raf` | 524,289 | 877 / 1077（13 秒間カクつく） |
| `"a\n"` | `line` + `append` | - | 130 秒以上たっても終わらず（強制終了） |
| `"a\n"` | `batch:16` + `raf` | 15 | 193 / 212 |
| `"a\n"` | `batch:16` + `tailspan` | 15 | 40 / 48 |
| 40 バイト/行（2.6 万行） | `batch:16` + `raf` | 2 | 62 / 73 |
| 40 バイト/行 | `batch:16` + `tailspan` | 2 | 21 / 24 |
| 改行なし（1 行 1MB） | `batch:16` + `raf` | 1 | 38 / 47 |
| 改行なし | `chunk` + `raf`（細かいイベントをつなげて追加） | 133 | 1077 / 2679 |
| 改行なし | `batch:16` + 16K 文字ごとの `<span>` に分ける | 1 | 11741 / 11755 |
| 改行なし | `batch:16` + `tailspan` | 1 | 20 / 26 |
| stdout と stderr を交互 | `batch:16` + `raf` | 6 | 1165 / 4531 |
| stdout と stderr を交互 | `batch:16` + `tailspan` | 7 | 29 / 42 |

分かったこと。

1. 1 行ごとに 1 イベントで送ると、行数が多いときに使えない。この試作の emit は 1 秒に約 4 万イベントまでで、`"a\n"` では 1MB に達するまで 13 秒かかった。追いつかない分はパイプを通して学習者のプログラムが待たされる。受け取るだけでも画面が 0.2〜1 秒止まった。
2. 時間でまとめて送ると IPC の費用はほぼなくなる。1MB が 1〜16 イベントになり、受け取るだけなら画面は止まらなかった。16ms と 50ms で大きな差はない。`read()` ごと（`chunk`）は、1 行ずつ書くプログラムで 1.6〜2 万イベントになる。
3. まとめて送っても、1MB を DOM に描く費用で止まる。`"a\n"` の 52 万行を 1 つの `<pre>` に足すと 0.2〜0.7 秒止まった（受け取るだけなら止まらないので、原因は DOM のレイアウトと描画）。
4. 改行のない 1MB は、DOM の組み立て方しだいで 0 秒〜12 秒止まった。原因は特定していない。
5. 表示する量に上限を付けると、どの場合も目安の内側に収まった。`tailspan` + `batch:16` は、4 種類のどのプログラムも、最後のイベントから 0.1 秒以内に表示し終わり、最悪の回でも rAF の最大間隔は 48ms で、色分けも保てた。上限なしでは、stdout と stderr が入れ替わるたびに `<span>` ができ（1.6〜3 万個）、1.2 秒（最悪 4.5 秒）止まった。

上限より前の出力をどう見せるか（捨てる、または仮想スクロールで全部見せる）は確かめていない。

### 仕様の制限値のとおりに動くか: 確かめた（OK）

`loop {}` は 60001.6ms で `timeout` になった。結果の状態は `success`・`runtime_error`（終了コード 101、シグナル 6、出力量の上限）・`timeout`・`stopped` を区別できた。

## 判定

基準は「両 OS で、実行・提出の仕様と制限値のとおりに動き、画面が固まらない」。macOS では条件付きで満たした。

| 確かめること | 判定 |
|---|---|
| 標準出力・標準エラー出力を少しずつ送れるか | OK（ただし、子が flush しない `print!` と `BufWriter` は子の側でたまる。端末でも同じ） |
| 標準入力を 1 行ずつ送れるか、閉じられるか | OK |
| 停止・タイムアウト・出力量の上限で、子までまとめて止められるか | 条件付き OK（グループから抜けた子は残る） |
| 1MB を一気に出力しても画面が固まらないか | 条件付き OK（一定時間ごとにまとめて送り、表示量に上限を付ければ固まらない。1 行ごとのイベントは NG） |
| 制限値（60 秒、1MB、結果の状態） | OK |

子プロセスの扱いの方式（プロセスグループ + killpg、パイプ + 読み取りスレッド、標準入力の専用スレッド）は変えずに実装できる。Issue の「うまくいかなかった場合」にある出力の送り方の見直し（まとめて送る）は必要になった。Windows は確かめていない。

## 残った事項

確かめられなかったこと。

- Windows（Job Object による停止、パイプの扱い）。
- macOS で二重 fork した孫を止める方法（`PR_SET_CHILD_SUBREAPER` に当たる仕組みの有無はドキュメントの範囲でしか調べていない）。
- 上限より前の出力を仮想スクロールで全部見せる場合の表示の費用。
- 改行のない 1MB の描画が、DOM の組み立て方で 0 秒〜12 秒と大きく違った原因。

ユーザーの判断が要る点。

1. グループから抜けた子を止めるか。案 A: killpg だけにする（端末と同じ。ADR 0014 の考え方に合う。実装が単純）。案 B: killpg に、親子関係をたどる停止を足す（抜けた子の多くを止められるが、二重 fork は止められず、完全ではない）。
2. 親が終わった後に同じグループに残った子を止めるか。試作では止めた。端末では残り続ける。止めないと、次の実行と並行して動き、パイプも持ち続ける。
3. stdout と stderr の順序。案 A: 別々のパイプのまま（`run_output` の `stdout` / `stderr` の区別を保てるが、順序は崩れうる）。案 B: 1 つのパイプにまとめる（順序は保てるが、区別できなくなる）。
4. 停止・タイムアウトの後、まだ画面に送っていなかった出力を捨てるか。試作では捨てた。送り切ると、画面への送信が遅いとき `run_finished` が数秒遅れる。
5. 画面に残す出力の量の上限と見せ方。ADR 0045 は、コンソールに実行の出力と提出の経過を続けて表示し、提出してもそれまでの出力を消さないとしている。表示量に上限を付けると古い出力が画面から消えるので、この判断との兼ね合いを決める。上限を付ける（末尾の 64K 文字や数千行）か、仮想スクロールにするか。
6. `print!` の後の flush。Rust の標準ライブラリの性質で、アプリ側では直せない。教材（動画・問題文）で `io::stdout().flush()` を教えるか。

仕様の変更案。

- [コードの実行（run）](../spec/exercise-runner/run.md) の手順 4 の「逐次送る」を、「一定時間（16〜50ms）ごとにまとめて送る。改行で終わらない出力もためずに送る。1 つのイベントに複数の stdout / stderr の断片を順に入れる」と書く。[フロントエンドとバックエンドの役割分担](../spec/architecture/responsibilities.md) の `run_output` の内容も、1 イベントに複数の断片が入る形にする。
- [演習画面](../spec/screens/exercise-screen.md) のコンソールに、表示する量の上限と、改行のない長い出力の表示のしかたを書く。
- [実行・判定の制限値](../spec/exercise-runner/limits.md) の出力量に、超えた後にパイプに残っていた分（最大 64KB 程度）は表示しないこと、上限で切れた UTF-8 の文字は捨てることを書く。
- run.md に、停止の方法（新しいプロセスグループで起動し、停止・タイムアウト・出力量の上限でグループ全体に SIGKILL）と、実行の終わりの判定（学習者のプログラムの終了を待ち、その後の出力は 300ms だけ待つ。EOF を待たない）を書く。
- ビルド中の停止ボタンの扱い。ビルド中も cargo と rustc をまとめて止められ、その後の再ビルドも成功したので、ビルド中も停止ボタンを有効にして結果を `stopped` とするかを決める。

そのほかの気づき。

- 描画の停止: 他のアプリのウィンドウなどのために、WKWebView の `requestAnimationFrame` が止まることがあった（`document.visibilityState` は `visible` のまま）。最前面にしても測定 447 回中 39 回で起き、その回は除いた。同じ時間帯に WKWebView の時間を測った他の検証結果は、この影響を受けていないか確かめ直したほうがよい。
- ビルド直後の実行ファイルは、最初の起動だけ約 200ms 遅い（2 回目以降は 4ms 前後）。学習者は実行のたびにビルドし直すので毎回遅れるが、制限値には影響しない。
