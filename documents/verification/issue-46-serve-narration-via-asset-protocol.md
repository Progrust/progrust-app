---
type: Verification Report
title: "音声をアプリ同梱データから asset:// で読む再生の技術検証"
description: "動画の音声をアプリ同梱データに置き、アセットプロトコル（asset://）で読んで再生できるかを Issue #46 で検証した結果（仕様の CSP での再生と media-src の対照、Remotion の共有の audio の data: が拒否されること、音声ファイルだけに絞ったスコープと外のファイルの 403、配信の回数と量の tauri:// との比較、get_lesson のパスでのレッスンの切り替え、tauri dev で CSP が掛からないこと、音声と絵のずれ、人が見て聞いた確認）を記す。"
---

- 日付: 2026-11-22
- Issue: [#46](https://github.com/Progrust/progrust-app/issues/46)
- 確かめた OS: macOS 27.2.1（Apple M1）
- 関連仕様: [フロントエンドとバックエンドの役割分担](../spec/architecture/responsibilities.md)、[Tauri の権限](../spec/architecture/tauri-permissions.md)、[セキュリティとプライバシー](../spec/non-functional/security-privacy.md)、[動画の音声](../spec/content-format/video/voice.md)、[ADR 0128](../decisions/0128-serve-narration-from-resources-via-asset-protocol.md)、[ADR 0092](../decisions/0092-play-narration-with-html5audio.md)、[ADR 0111](../decisions/0111-bundle-monaco-and-set-csp.md)

## 環境

- macOS 27.2.1（26B131）、Apple M1。WebKit.framework 22625.2.8.11.9。rAF 58.8Hz。
- rustc 1.100.0、node v25.9.0、Tauri CLI 2.12.6、tauri 2.12.7（feature `protocol-asset`）、wry 0.57.3、@tauri-apps/api 2.12.5。
- Vite 8.3.8、React 19.3.1、remotion・@remotion/player 4.0.545。
- 計測は、開発時の起動（「結果」の「開発時の起動（`tauri dev`）でも読めるか」）を除き、release ビルド（`npx tauri build --bundles app`）の .app の実行ファイルを、環境変数を付けて起動して行った。ページが自分で計測し、Tauri のコマンドで JSON に書いて終了する（[Issue #4 の検証結果](issue-4-lazy-load-lesson-video.md)と同じ仕組み）。ビルドの時間を短くするため、release の LTO を切り、codegen-units を 16 にした。
- 全回で、ウィンドウは表示・フォーカス済みで、rAF の間隔の中央値は 17ms だった。

## 方法

### 作ったもの

[Issue #4 の検証結果](issue-4-lazy-load-lesson-video.md)の試作（レッスン 3 つ: ch04-move 21 文、ch02-move-b 21 文、ch07-short 5 文）を写し、次のように直した。

- 音声をフロントエンドのビルドから外した。レッスンの塊を作る Vite のプラグインで、音声の `?url` の import をやめた。`dist/` の中の `.m4a` は 0 個になった。
- `tauri.conf.json` の `app.security` を次のようにした。CSP は [Issue #21 の検証結果](issue-21-monaco-in-tauri.md)の試作の CSP（[セキュリティとプライバシー](../spec/non-functional/security-privacy.md)「アプリ自体」の CSP）に、`media-src` を足したもの。スコープは、仕様（[Tauri の権限](../spec/architecture/tauri-permissions.md)）の音声ファイルだけにした。

  ```json
  "csp": "default-src 'self'; script-src 'self'; worker-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self' data:; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost; media-src 'self' asset:",
  "assetProtocol": { "enable": true, "scope": { "allow": ["$RESOURCE/content/lessons/*/video/voice/*.m4a"] } }
  ```

- `bundle.resources` で `content/` をまるごとリソースに入れた。スコープの外のファイル（`script.json`・`lesson.json` など）もリソースの中にある状態で試すためである。試験用に、スコープの外のファイルを足した（「結果」の「スコープ」の表）。
- バックエンドに `get_lesson` の代わりのコマンドを作った。台本と、音声のディレクトリの絶対パスを返す。

  ```rust
  let video = app.path().resource_dir()?.join("content/lessons").join(&lesson_id).join("video");
  Ok(LessonData { script: fs::read_to_string(video.join("script.json"))?, voice_dir: video.join("voice").display().to_string() })
  ```

- フロントエンドは、そのパスと文の ID から URL を作り、`Html5Audio` に渡す。`Html5Audio` は文ごとに `Sequence` の中に置く（[ADR 0092](../decisions/0092-play-narration-with-html5audio.md)）。フロントエンドは Tauri のパスの API を呼ばない。

  ```ts
  const { voiceDir } = await invoke("get_lesson", { lessonId });
  const src = convertFileSrc(`${voiceDir}/${beatId}.m4a`);
  // → asset://localhost/%2FUsers%2F…%2FContents%2FResources%2Fcontent%2Flessons%2Fch04-move%2Fvideo%2Fvoice%2Fh1.m4a
  ```

### 計測の仕組み

- asset:// の要求は、Tauri の `on_web_resource_request` に出ない。数えるため、tauri 2.12.7 のソースを写して Cargo の `[patch.crates-io]` で差し替え、アセットプロトコルの処理（`src/protocol/asset.rs` の `get`）に記録を 1 か所足した。環境変数があるときだけ、要求ごとに、メソッド、パス、`Range`、返したステータス、本体のバイト数、`Content-Range` をファイルに追記する。スコープの判定と応答は変えていない。人が聞いた確認のビルドは、差し替えを外して作った。
- tauri:// の配信は、[Issue #4 の検証結果](issue-4-lazy-load-lesson-video.md)と同じく `on_web_resource_request` で数えた。
- CSP の違反は、最初に読み込むモジュールで `securitypolicyviolation` を受けて記録した。ページに実際に付いた CSP は、`fetch(location.href)` の応答ヘッダーで読んだ。
- 次の 5 つのビルドを作った。CSP は `--config` で変えた。

  | ビルド | 音声の置き方 | `media-src` | tauri |
  |---|---|---|---|
  | 本体 | asset:// | `'self' asset:` | 記録を足したもの |
  | 対照（asset なし） | asset:// | `'self'` | 記録を足したもの |
  | 対照（data: あり） | asset:// | `'self' asset: data:` | 記録を足したもの |
  | 比較（tauri://） | Vite のアセット（`?url`。47 個） | `'self' asset:` | 記録を足したもの |
  | 人が聞く確認 | asset:// | `'self' asset:` | 元のまま |

### 計測の手順

- A. Issue #4 と同じ手順: ch04 の Player を出し、文 h1 の途中へシークして 2.5 秒再生する（約 3 秒）。続けて 5 秒再生する。5 回繰り返す。
- B. 再生中の操作: 次の文・前の文・5 秒先・5 秒前・シークバー（5 か所）・速度（2・0.75・1.5・1 倍）・一時停止と再開・同じ文の中で 1 秒戻る（2 回）。操作ごとに 2 秒待ち、その間の配信を数える。同じ文の中のシークは、あとから足し、本体と同じ設定で作り直したビルドで 1 回測った。
- C. レッスンの切り替え: ch04 → ch02 → ch07 → ch04 の順に、`get_lesson` を呼び、Player を出して 2 文目から 2.5 秒再生する。
- D. 音の中身: `crossOrigin = "anonymous"` の `<audio>` を `createMediaElementSource` で AnalyserNode に通して RMS を測る（出力は音量 0）。
- E. スコープ: スコープの中と外のファイルの URL を `<audio>` に入れ、読めたか（`loadedmetadata`）とバックエンドの応答を記録する。`fetch` は `connect-src`、`<img>` は `img-src` で止まるので、`<audio>` で試した。
- F. 音声と絵のずれ: [Issue #2 の検証結果](issue-2-m4a-audio-sync.md)の Html5Audio と同じ考え方で、rAF ごとに `<audio>` の `currentTime` と絵の位置を記録した。絵の位置は、各フレームが最初に現れた rAF の時刻に ±700ms の窓で直線を当てはめて求めた。ずれは「(絵の位置 − 文の開始フレーム) ÷ 30 − currentTime」で、正の値は音が絵より遅いこと。ch04 を 1 倍で通しで、0.75・1.5・2 倍で 2〜7 文目を再生し、再生中に文の途中へ 7 回シークした。本体と比較（tauri://）のビルドで 2 回ずつ測った。

## 結果

### 仕様の CSP の下で再生できるか・`media-src` に何が要るか

再生できた。`media-src` には `asset:` が要る。

- ページに付いた CSP は次のとおり。Tauri がビルド時に `script-src` にハッシュを足すが、`media-src` は書いたとおりだった。

  ```
  worker-src 'self'; connect-src 'self' ipc: http://ipc.localhost; default-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self' data:; media-src 'self' asset:; script-src 'self' 'sha256-…'（6 個）; img-src 'self' data:
  ```

- 本体のビルド: 全段階で音声が鳴った（`currentTime` が 2.5 秒の再生で 2.43〜2.47 秒進んだ）。`<audio>` の `currentSrc` はすべて `asset://localhost/…`。tauri:// で配信した `.m4a` は 0 件。
- 対照（`media-src 'self'`）: asset:// の音声はすべて `media-src` の違反で拒否され、バックエンドに要求が 1 件も届かず、どの段階でも鳴らなかった。`'self'` は asset:// を含まない。
- `asset:` だけで足りた。macOS の `convertFileSrc` の URL は `asset://localhost/…` である。Tauri の文書が Windows・Android 向けに挙げる `http://asset.localhost` は、確かめていない。

`<audio>` 以外の読み方は、CSP で止まった。

| 読み方 | 結果 |
|---|---|
| `fetch(asset://…)` | `connect-src` の違反。`TypeError: Load failed`。バックエンドに要求は届かない |
| `<img src="asset://…">` | `img-src` の違反。バックエンドに要求は届かない |

- `@remotion/media` の `Audio` は音声を `fetch` で読む（[Issue #2 の検証結果](issue-2-m4a-audio-sync.md)）ので、この CSP では鳴らない見込み（試していない）。仕様の部品は `Html5Audio` なので影響はない。

### Remotion の共有の `<audio>` の `data:` が拒否される

- 本体のビルドで、`media-src` の違反が `data:` に対して出続けた（計測 1 回の間に 74 件）。
- Remotion の Player は、`Html5Audio` のために共有の `<audio>` を作る（既定で 5 個）。使っていない `<audio>` には、無音の音声 `EMPTY_AUDIO`（`data:audio/mp3;base64,…`）を `src` に入れておく（remotion 4.0.545 の `SharedAudioContextProvider`）。これが CSP で拒否される。
- 文の音声を鳴らすときは、Remotion が `src` を asset:// の URL に替えるので、再生には影響しなかった。
- 対照（`media-src 'self' asset: data:`）では、`media-src` の違反は 0 件になった。配信の回数・量、再生、レッスンの切り替えの結果は、本体のビルドと同じだった。

### スコープを音声ファイルだけに絞っても再生できるか・外のファイルが読めないか

再生でき、外のファイルは読めなかった。スコープは `$RESOURCE/content/lessons/*/video/voice/*.m4a`。Tauri のスコープの照合では `*` が `/` をまたがない（`require_literal_separator: true`）。

| 試したパス | `<audio>` | バックエンドの応答 |
|---|---|---|
| ch04 の `voice/h1.m4a`（対照） | 読めた | 206 |
| ch02 の `voice/h1.m4a`（対照） | 読めた | 206 |
| 同じレッスンの `video/script.json` | エラー（code 4） | 403 |
| 同じレッスンの `lesson.json` | エラー | 403 |
| `voice/` の外の `.m4a`（`video/outside.m4a`） | エラー | 403 |
| リソースの直下の `.m4a`（`content/outside.m4a`） | エラー | 403 |
| `voice/` の中の `.m4a` 以外（`notes.txt`） | エラー | 403 |
| `voice/` の中の `.m4a` 以外（`h1.m4a.bak`。中身は音声） | エラー | 403 |
| `voice/` の下のサブディレクトリの `.m4a`（`voice/sub/h1.m4a`） | エラー | 403 |
| `voice/` の中のシンボリックリンク（`link.m4a` → `/System/Library/Sounds/Ping.aiff`） | エラー | 403 |
| `voice/` の中のシンボリックリンク（`inlink.m4a` → 同じ `voice/` の `h2.m4a`。相対パス） | エラー | 403 |
| リソースの外の音声（`/System/Library/Sounds/Ping.aiff`） | エラー | 403 |
| リソースの外のファイル（`/etc/hosts`） | エラー | 403 |
| `..` で `voice/` の外へ（`voice/../outside.m4a`） | エラー | 403 |
| `..` でスコープの中へ（`voice/../voice/h1.m4a`） | エラー | 403 |
| 存在しない `.m4a`（`voice/nothing.m4a`） | エラー | 404 |

- `..` を含むパスは、スコープの照合の前に、Tauri の `SafePathBuf` が拒否する（中身がスコープの中でも 403）。
- 相対パスのシンボリックリンクは、行き先がスコープの中でも 403 だった。Tauri はリンクの行き先を `read_link` で読み、相対パスのまま存在を確かめるためとみられる（tauri 2.12.7 の `scope/fs.rs`）。教材の音声はシンボリックリンクを使わないので、影響はない。

### 読み込みとシークのときの配信の回数と量

回数は tauri:// と同じだった。1 回に配る量が小さくなり、合計は音声ファイル 1 つ分ほどになった。

手順 A（Player を出し、文 h1（52,755 バイト）の途中へシークして 2.5 秒再生する。約 3 秒）:

| | tauri://（比較のビルド） | asset://（本体のビルド） |
|---|---|---|
| 要求の回数 | 81 回（5 回とも） | 81 回（10 回とも。2 回の計測 × 5） |
| 応答 | すべて 200・52,755 バイト（全体） | すべて 206 |
| 配った量の合計 | 4,273,155 バイト | 51,366 バイト |
| 要求が続いた時間 | | 320〜333ms |

- asset:// の 81 回の内訳: 77 回は 2〜100 バイトの範囲（ファイルの構造を読む）で、4 回が 1,137〜24,040 バイトの範囲（音声のデータ）だった。

  ```
  GET bytes=0-1 → 206 2B bytes 0-1/52755
  GET bytes=0-7 → 206 8B bytes 0-7/52755
  GET bytes=28-35 → 206 8B bytes 28-35/52755
  …
  GET bytes=1655-25694 → 206 24040B bytes 1655-25694/52755
  GET bytes=25695-49671 → 206 23977B bytes 25695-49671/52755
  ```

- 続けて 5 秒再生した間に、次の文 h2（35,202 バイト）に移り、h2 を 80 回要求した。tauri:// は 80 回（記録では 1 回ごとに全体を返していたので、計算すると 80 × 35,202 = 2,816,160 バイト）、asset:// は 80 回で 35,424 バイトだった。
- 手順 B の操作ごとの配信（asset://。2 回の計測で同じ傾向）:

  | 操作 | 要求 | 配った量 |
  |---|---|---|
  | 操作なしで再生を続ける | 0 回 | 0 |
  | 次の文・前の文（5 回） | 各 80 回（その文のファイル） | 31〜34KB |
  | 5 秒先・5 秒前（4 回） | 79〜158 回（文が 1 つか 2 つ替わった） | 18〜30KB |
  | シークバー（5 回） | 80〜159 回 | 32〜51KB |
  | 速度を変える（同じ文の中） | 0〜2 回 | 0〜33KB |
  | 速度を 0.75 倍に（直後に文が替わった） | 80 回 | 44KB |
  | 一時停止 | 0 回 | 0 |
  | 同じ文の中で 1 秒戻る（文 h1。2 回） | 各 2 回 | 各 26,661 バイト |

  tauri:// のビルドでも、操作ごとの要求の回数は同じだった（各 79〜81 回、文が 2 つ替わると 158〜159 回）。
- このことから、約 80 回の要求は、WebKit の `<audio>` が 1 つのファイルを読み込むたびに出すもので、tauri:// が 200 で全体を返すことによるものではない。Remotion は文が替わるたびに共有の `<audio>` の `src` を替えるので、文が替わるたびに 1 回の読み込みが起きる。同じ文の中で戻るシークと、同じ文の中での速度の変更では、要求は 0〜2 回だった。同じ文の中で進むシークは、試したときに次の文に移ってしまい、測れなかった。
- Tauri のアセットプロトコルは、要求のたびにファイルを開き、種類を調べるため先頭の最大 8,192 バイトを読む（tauri 2.12.7 の `protocol/asset.rs`）。1 回の読み込みで約 80 回開くことになる。CPU や待ち時間への影響は測っていない。どちらの置き方でも、再生に問題は見えなかった。

### `get_lesson` のパスと `convertFileSrc` で、レッスンを切り替えても、そのレッスンの音声が鳴るか

鳴った（本体のビルドで 2 回、開発時の起動で 2 回、いずれも同じ）。

| 順 | `get_lesson` の音声のディレクトリ（末尾） | 要求したファイル | 2.5 秒の再生での `currentTime` の進み |
|---|---|---|---|
| 1. ch04-move | `…/Contents/Resources/content/lessons/ch04-move/video/voice` | `ch04-move/video/voice/h1.m4a` だけ | 2.46〜2.47 秒 |
| 2. ch02-move-b | `…/content/lessons/ch02-move-b/video/voice` | `ch02-move-b/video/voice/h1.m4a` だけ | 2.44〜2.46 秒 |
| 3. ch07-short | `…/content/lessons/ch07-short/video/voice` | `ch07-short/video/voice/s1.m4a` だけ | 2.46 秒 |
| 4. ch04-move | `…/content/lessons/ch04-move/video/voice` | `ch04-move/video/voice/h1.m4a` だけ | 2.46 秒 |

- 各段階で要求したファイルは、そのレッスンの `voice/` のものだけだった。
- 手順 D で、ch04 と ch02 の h1 の RMS の最大は 0.194〜0.198 で、音の中身が出ていた。asset:// の応答の `Access-Control-Allow-Origin` はウィンドウの origin（`tauri://localhost`）で、`crossOrigin` を付けた `<audio>` を Web Audio に通せた。

### 開発時の起動（`tauri dev`）でも読めるか

読めた。ただし、開発時の起動では CSP が掛からなかった。

- ページの origin は `http://localhost:1420`（Vite の開発サーバー）。リソースのディレクトリは `src-tauri/target/debug/` で、`get_lesson` はその中の `content/lessons/{id}/video/voice` を返した。
- 配信（手順 A: 81 回・51,366 バイト）、レッスンの切り替え、スコープの表の結果は、本体のビルドと同じだった。
- CSP の違反は 0 件で、ページの応答に CSP のヘッダーは無かった。`fetch(asset://…)` も 206 で読めた（release ビルドでは `connect-src` で拒否される）。スコープの外のファイルは、開発時も 403 だった。
- Tauri は CSP を、tauri:// でページを配るときの応答ヘッダーとして付ける（tauri 2.12.7 の `protocol/tauri.rs`）。デスクトップの開発時は、WebView が開発サーバーの URL を直接開く（開発サーバーを Tauri の中継で配るのはモバイルだけ。`manager/webview.rs` の `PROXY_DEV_SERVER`）。開発時用の CSP（`app.security.devCsp`）も同じ経路で付くので、デスクトップの開発時には掛からないとみられる（試していない）。

### 音声と絵のずれ（自動の計測。参考）

本体（asset://）と比較（tauri://）のビルドで、同じ手順で 2 回ずつ測った。単位は ms。中央値（範囲）。

| | asset:// 1 回目 | asset:// 2 回目 | tauri:// 1 回目 | tauri:// 2 回目 |
|---|---|---|---|---|
| 1 倍の通し（21 文） | +119（+81〜+138） | +120（+84〜+169） | +117（+17〜+135） | +118（+82〜+139） |
| 1 倍の文の頭 | +93（+82〜+108） | +94（+84〜+142） | +89（+39〜+105） | +92（+83〜+109） |
| 0.75 倍 | +124（+71〜+139） | +125（+72〜+129） | +117（+66〜+131） | +119（+67〜+129） |
| 1.5 倍 | +211（+150〜+234） | +208（+142〜+232） | +218（+137〜+230） | +210（+141〜+217） |
| 2 倍 | +280（+180〜+292） | +280（+185〜+295） | +268（+185〜+306） | +289（+181〜+300） |
| シークの直後（7 か所） | +94（+86〜+103） | +94（+83〜+108） | +93（+82〜+141） | +90（+78〜+118） |

- asset:// と tauri:// の差は、どの項目でも 15ms 以内で、2 回の計測の差と同じくらいだった。音声の置き方は、ずれに影響していない。
- 1 倍のずれ（約 +120ms）は、[Issue #2 の検証結果](issue-2-m4a-audio-sync.md)の Html5Audio（中央値 +110ms）とほぼ同じ。1.5・2 倍は Issue #2（最大 +210ms）より大きく出た（+210〜+290ms）。Issue #2 と計測の当てはめ方が少し違い、試作も違う（Issue #2 は 1 レッスンの試作）。差の原因は調べていない。今回の tauri:// のビルドでも同じ値なので、asset:// による差ではない。

### 人が見て聞いた確認

自動の計測のあと、ユーザーが、人が聞く確認のビルド（仕様の CSP と、音声ファイルだけに絞ったスコープ。tauri は元のまま）を、人が触る画面で起動して聞いた。画面では、レッスンの切り替え、再生と一時停止、前の文・次の文、±5 秒、シークバー、速度（0.75・1・1.25・1.5・2 倍）を操作できる。

- 1 倍、速度を変えたとき、シークの直後の音声と絵のずれ、音質、レッスンを切り替えたときの音声のどれも、特に気にならなかった。
- 最初に渡した画面には、レッスンを切り替えると操作のボタンが Player を見失う誤り（試作の画面の誤りで、asset:// とは関係ない）があった。直したビルドで聞き直した。

## 判定

基準「本番ビルドで、仕様の CSP と、音声ファイルだけに絞ったスコープの下で、音声が再生でき、人が見て聞いて気にならない」は、満たした。

- 仕様の CSP（`media-src` は `'self'` と `asset:`）とスコープ（`$RESOURCE/content/lessons/*/video/voice/*.m4a`）の下で、音声が鳴り、スコープの外のファイルは読めなかった。
- 人が見て聞いて、気にならなかった。

仕様（[Tauri の権限](../spec/architecture/tauri-permissions.md)、[セキュリティとプライバシー](../spec/non-functional/security-privacy.md)）と音声の置き方（[ADR 0128](../decisions/0128-serve-narration-from-resources-via-asset-protocol.md)）を見直す必要はない。

## 残った事項

判断が要る点: なし。

確かめられなかった項目

- Windows・Android 向けの `http://asset.localhost` の形の URL と CSP。対応環境の外なので確かめていない（[ADR 0090](../decisions/0090-support-only-authors-mac-until-completion.md)）。
- 開発時用の CSP（`app.security.devCsp`）を設定したときの開発時の動き。
- 1 回の読み込みで約 80 回ファイルを開くことの、CPU と待ち時間への影響。再生に問題は見えなかった。

検証中に見つけた別の問題

- デスクトップの開発時の起動（`tauri dev`）では CSP が掛からない。開発時に動いても、本番ビルドで CSP に拒否されることがある。開発時の CSP の扱いが仕様と運用ルールに無いことを、Issue [#58](https://github.com/Progrust/progrust-app/issues/58) に起票した。
- Remotion の共有の `<audio>` の無音の `data:` が、`media-src` の違反として出続ける。再生には影響しない。`media-src` に `data:` を足せば出なくなる。違反を許すのか、`data:` を足すのか（読み込み先を最小限にする方針、[ADR 0111](../decisions/0111-bundle-monaco-and-set-csp.md)）の定めが仕様に無いことを、Issue [#59](https://github.com/Progrust/progrust-app/issues/59) に起票した。
- 仕様の CSP の `connect-src` では、`fetch` で asset:// を読めない。音声を `fetch` で読む部品（`@remotion/media` の `Audio`）に替えるときは、CSP を見直す必要がある。
