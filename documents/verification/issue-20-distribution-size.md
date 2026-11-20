---
type: Verification Report
title: "配布物と更新のサイズの技術検証"
description: "音声・フォント・フロントエンド JS・Tauri の実行ファイルを含む配布物と更新のサイズを Issue #20 で検証した結果（音声の形式ごとのサイズ、142 レッスン分の見積もり、HE-AAC の扱い、Murecho のサイズ、Monaco と Remotion の JS、Tauri の updater が差分更新をしないこと、選択肢ごとのインストーラーのサイズ）を記す。"
---

- 日付: 2026-11-21
- Issue: [#20](https://github.com/Progrust/progrust-app/issues/20)
- 確かめた OS: macOS 27.2.1（Apple Silicon、aarch64）。Windows は確かめていない
- 関連仕様: [配布と更新](../spec/non-functional/distribution.md)、[ナレーション音声](../spec/content-format/video/voice.md)、[画面の未決事項](../spec/screens/open-questions.md)、[ADR 0037](../decisions/0037-bundle-content-and-auto-update.md)、[ADR 0063](../decisions/0063-generate-narration-with-voicepeak-and-bundle.md)、[ADR 0004](../decisions/0004-monaco-without-completion.md)

## 環境

- macOS 27.2.1、Apple Silicon（aarch64）。測定中は他のエージェントが動いていた（load average 11.6）が、サイズの測定なので影響しない。
- rustc 1.99.1、cargo 1.99.1、node v25.9.0、ffmpeg 9.0.2、tauri-cli 2.12.6（tauri crate 2.12.7）
- rustup のターゲットは `aarch64-apple-darwin` だけ。`x86_64-apple-darwin` は入れていない。ユニバーサルバイナリは見積もりである。
- ffmpeg に `libfdk_aac` は入っていない。

## 方法

- 音声: 試作の動画 1 本分のナレーション（21 文、モノラル 48kHz 16bit の WAV、合計 8,248,986 バイト）を 13 通りの設定で変換し、合計サイズを測った。音声の合計の長さは 85.909 秒（1 文平均 4.09 秒）。Issue の「1 分 44 秒」は動画全体の長さで、音声だけではない。1 文 1 ファイルなので、コンテナのオーバーヘッドが効く。
  - ffmpeg の標準 AAC エンコーダー: `ffmpeg -i x.wav -c:a aac -b:a 64k -ac 1 x.m4a`（48k、32k も）
  - macOS の afconvert の AAC-LC: `afconvert -f m4af -d aac -b 64000 x.wav x.m4a`
  - HE-AAC v1: `afconvert -f m4af -d aach -b 32000 x.wav x.m4a`（24000 も）
  - HE-AAC v2 の要求: `afconvert -f m4af -d aacp -b 24000 x.wav x.m4a`
  - Opus（参考）: `ffmpeg -i x.wav -c:a libopus -b:a 24k -ac 1 x.opus`
  - ffmpeg の `aac_at`（AudioToolbox）は `-profile aac_he` を受け付けない（`Unable to parse "profile" option value "aac_he"`）。HE-AAC は afconvert でだけ作れた。
- Tauri: `npm create tauri-app@latest -- empty-app --template react-ts` の空のアプリを、`bundle.targets = ["app","dmg"]`、`createUpdaterArtifacts = true` にして `npx tauri build`（release）で作った。テンプレートの `[profile.release]` は `codegen-units=1, lto=true, opt-level=3, panic="abort", strip=true`。更新用の署名鍵は使い捨てで作った。
- フロントエンド JS: Vite 7 で、React 19、`@remotion/player` 4.0.545、`monaco-editor` 0.57.0（エディター本体、Rust のシンタックス定義、エディターワーカーだけ。TypeScript などの言語サービスは含めない）を本番ビルドした。
- フォント: 試作の Murecho と Red Hat Mono の TTF を、fonttools（pyftsubset、instancer、brotli）で woff2 化・サブセット化した。

## 結果

### 音声のビットレートを下げられるか: サイズは下がる（音質は未確認）

試作の 21 文（85.9 秒）の合計サイズ。「実効 kbps」は、ヘッダーを含めたサイズからの逆算。

| 設定 | 21 ファイル合計 | 実効 kbps | 180 秒あたり |
|---|---:|---:|---:|
| WAV（元） | 8,248,986 B | 768 | 15.5 MB |
| AAC-LC 64k（ffmpeg `aac`）。仕様の形式 | 730,040 B | 68.0 | 1.53 MB |
| AAC-LC 48k（ffmpeg） | 557,313 B | 51.9 | 1.17 MB |
| AAC-LC 32k（ffmpeg） | 378,450 B | 35.2 | 0.79 MB |
| AAC-LC 64k（afconvert） | 802,225 B | 74.7 | 1.68 MB |
| AAC-LC 48k（afconvert） | 654,042 B | 60.9 | 1.37 MB |
| AAC-LC 32k（afconvert。32kHz に落とす） | 449,187 B | 41.8 | 0.94 MB |
| HE-AAC v1 32k（afconvert） | 496,141 B | 46.2 | 1.04 MB |
| HE-AAC v1 24k（afconvert） | 369,686 B | 34.4 | 0.77 MB |
| HE-AAC v2 を要求（afconvert、モノラル入力） | 32k: 496,254 B、24k: 369,707 B | 上と同じ | 上と同じ |
| Opus 32k（参考） | 352,105 B | 32.8 | 0.74 MB |
| Opus 24k（参考） | 268,489 B | 25.0 | 0.56 MB |
| Opus 16k（参考） | 181,304 B | 16.9 | 0.38 MB |

- afconvert の出力は 1 ファイルあたり 4,096 バイトのオーバーヘッドがある（`moov` 1.2KB と、`free` の空き領域 2.8KB）。`ffmpeg -c copy` で詰め直すと 1,579 バイトになる（HE-AAC 24k の 1 文で 16,459 バイトが 13,525 バイト）。詰め直したあとも、`afinfo` は 24000Hz・モノラル・HE-AAC を示し、ffprobe の profile も `HE-AAC` のままだった。上の表の afconvert の値は詰め直す前。
- 仕様は「AAC、モノラル、64kbps」だけで、エンコーダーを決めていない。同じ 64k でも、ffmpeg は 730KB、afconvert は 802KB と、1 割違う。
- AAC は圧縮済みなので、`.dmg` や `.tar.gz` にしてもほぼ縮まない（64k の 21 ファイルを tar.gz にして 730,040 B が 716,471 B）。
- モノラル入力に `-d aacp`（HE-AAC v2）を指定すると、エラーにならず HE-AAC v1 のファイルができ、サイズも v1 と一致する（24k: 369,707 B と 369,686 B）。HE-AAC v2 は Parametric Stereo（PS）でステレオを作る方式なので、モノラルでは使えない。モノラルで使えるのは HE-AAC v1 まで。
- HE-AAC の `AudioSpecificConfig` は、AOT 2（LC）+ 同期拡張 0x2b7 + AOT 5（SBR）、SBR 出力 48kHz の、明示的な後方互換のシグナリングだった。Apple の TN2236 が MP4 で推奨する方式と同じ。
- ffmpeg のデコーダーは、afconvert のモノラル HE-AAC を 48kHz・2 チャンネルで扱った（`ffprobe` が `channels=2`）。`afinfo` は 1ch・Mono と表示する。原因は確かめていない。

### WKWebView が HE-AAC の .m4a を再生できるか: 未確認

実際の再生は確かめていない。調べた範囲は次のとおり。

- Apple の TN2236 は「iPhone OS 2.2 から HE-AAC v1 に対応した」と書いている。Safari や macOS の対応版には触れていない。
- WebKit は macOS では CoreAudio と AVFoundation でデコードするので、CoreAudio が扱える AAC 系は再生できる、というのが一般的な理解だが、これを直接述べた一次情報は見つけられなかった。
- afconvert の HE-AAC は、この Mac の CoreAudio 自身が出力した形式で、`afinfo` で読める。WebView で再生できることの確認ではない。
- macOS 13 は実機で確かめていない。
- Opus（参考）: MDN は「Safari は Opus を `<audio>` で、CAF に入れたときだけ再生する」と書いている（macOS High Sierra と iOS 11）。古い記述の可能性があり、新しい Safari の対応は確かめていない。`.m4a` に Opus を入れる案は、仕様の形式の変更になる。

### 音質: 未確認

人が聞かないと分からないため、判定していない。聞き比べ用に、3 文を設定 12 通りに変換したファイルを用意した。

### 1 本 3 分 × 142 レッスンの音声全体の見積もり

- `spec/curriculum/chapters/ch*.md` の表の行は 165 行。うち 23 行が章末復習（`chNN-review`）で、残りの 142 が Issue の「約 142 レッスン」である。
- 計算: 3 分 × 142 = 25,560 秒（試作の 1 文 4.09 秒から約 6,248 文 = 6,248 ファイル）。サイズ = 試作の合計サイズ ÷ 85.909 秒 × 25,560 秒。

| 設定 | 142 本の合計（MB、1MB = 10^6 バイト） | 165 本なら |
|---|---:|---:|
| AAC-LC 64k（ffmpeg。仕様の形式） | 217 | 252 |
| AAC-LC 48k（ffmpeg） | 166 | 193 |
| AAC-LC 32k（ffmpeg） | 113 | 131 |
| AAC-LC 64k（afconvert、詰め直し前 / 後） | 239 / 223 | 277 / 259 |
| AAC-LC 48k（afconvert、詰め直し前 / 後） | 195 / 179 | 226 / 208 |
| AAC-LC 32k（afconvert、詰め直し前 / 後） | 134 / 118 | 155 / 137 |
| HE-AAC v1 32k（afconvert、詰め直し前 / 後） | 148 / 132 | 171 / 153 |
| HE-AAC v1 24k（afconvert、詰め直し前 / 後） | 110 / 94 | 128 / 109 |
| Opus 32k（参考） | 105 | 122 |
| Opus 24k（参考） | 80 | 93 |
| Opus 16k（参考） | 54 | 63 |

Issue の約 200MB は、仕様の 64kbps（ffmpeg）で 217MB となり、ほぼ合っている。この値には 1 文 1 ファイルのオーバーヘッド（約 1.6KB × 6,248 = 約 10MB）が含まれる。

### フォント: 仕様で使う書体は Murecho と Red Hat Mono

- [画面の未決事項](../spec/screens/open-questions.md) に「書体（Murecho、Red Hat Mono）のファイルをアプリに同梱する方法。ライセンスと、日本語の書体を同梱したときのファイルの大きさを確認する」とある。和文フォントは Noto Sans JP ではなく Murecho。書体の値の定義はデザインシステム（claude.ai の artifact）にあり、documents/ にはないので、デザインシステムそのものは読んでいない。
- 試作の TTF は、Murecho が 1,430,664 B（可変フォント、wght 100〜900、4,450 コードポイント）、Red Hat Mono が 68,356 B（可変、wght 300〜700）。
- JIS 第一水準は、第 1 水準漢字 2,965 字と記号・かな・ASCII（619 字）。第二水準は第 2 水準漢字 3,390 字。

| 内容 | サイズ |
|---|---:|
| Murecho 可変、全グリフ、woff2 | 621,440 B |
| Murecho 可変、JIS 第一水準 + 第二水準、woff2 | 526,964 B |
| Murecho 可変、JIS 第一水準、woff2 | 505,708 B |
| Murecho 可変、漢字なし（かな・記号・ASCII）、woff2 | 66,592 B |
| Murecho 静的 400 / 700、全グリフ、woff2 | 352,064 B / 356,264 B |
| Murecho 静的 400 / 700、JIS 第一水準、woff2 | 280,288 B / 284,668 B |
| Red Hat Mono 可変、woff2 | 28,140 B |
| Red Hat Mono 静的 400 / 700、woff2 | 12,164 B / 12,352 B |
| （参考。仕様では使わない）Noto Sans JP 可変の全スライス 124 ファイルの woff2 の合計 | 5,473,652 B |

- Murecho は可変のまま全グリフで woff2 にしても 0.62MB。Red Hat Mono と合わせて 0.65MB。サブセット化しても約 0.1MB しか減らない。
- 教材が使う字を調べたサブセット化は行っていない（教材がまだ無い）。ライセンスの確認は、この Issue の範囲外。

### フロントエンド JS（Monaco、Remotion、React）

Vite の本番ビルド（最小限の Player と Monaco）:

| ファイル | 生 | gzip |
|---|---:|---:|
| `index-*.js`（React + Remotion Player + Monaco 本体 + Rust 定義） | 3,327,155 B | 890,863 B |
| `editor.worker-*.js`（Monaco のワーカー） | 275,665 B | 82,570 B |
| `index-*.css` | 99,224 B | 14,370 B |
| `rust-*.js` | 4,022 B | 1,790 B |
| 合計 | 約 3.7MB | 約 0.99MB |

- 部品を外して測ると、React + ReactDOM だけで 222KB（gzip 69KB）、Remotion Player を足して 589KB（gzip 185KB）。Remotion Player は約 0.37MB、Monaco は約 2.7MB（gzip 約 0.7MB）。
- Monaco 0.57.0 はパッケージ構成が変わり、旧来の `monaco-editor/esm/vs/basic-languages/rust/rust.contribution` が `exports` の制限で解決できない。Rust の定義は `monaco-editor/languages/definitions/rust/register` から取る。実装時に注意が必要。
- [ADR 0004](../decisions/0004-monaco-without-completion.md) により補完は使わないので、言語サービスは含めていない。
- 教材そのもの（動画の場面の TSX、スライド・演習の本文、判定用のテスト）のサイズは測っていない。音声に比べて小さいと見込まれる。

### Tauri の実行ファイルと更新ファイル（空のアプリ、aarch64 の実測）

```
Finished 2 bundles at:
  .../bundle/macos/empty-app.app (3.98 MiB)
  .../bundle/dmg/empty-app_0.1.0_aarch64.dmg (1.87 MiB)
  .../bundle/macos/empty-app.app.tar.gz (updater) (1.83 MiB)
```

| 成果物 | バイト |
|---|---:|
| `Contents/MacOS/empty-app`（arm64、`strip=true`、LTO 済み） | 4,076,384 |
| `.app`（`du`） | 約 4.09MB |
| `.dmg` | 1,961,317 |
| `.app.tar.gz`（更新用。`.sig` も作られた） | 1,915,971 |

- ユニバーサルバイナリは未実測。見積もりは、実行ファイル約 8.2MB、`.dmg` と `.app.tar.gz` は圧縮後で約 3.8MB（x86_64 が arm64 と同じ大きさ・同じ圧縮率と仮定した）。差は約 +1.9MB。
- 実際のアプリは、Tauri のプラグインと自前の Rust のコードが足される。その分の増加は測っていない。

### Tauri の updater が差分更新に対応していないこと: 確認できた

- 公式ドキュメント（https://v2.tauri.app/plugin/updater/ ）は、macOS では `target/release/bundle/macos/` のアプリケーションバンドルから `.tar.gz` を作ると書き、更新用の成果物は `myapp.app.tar.gz` と `.sig` である。更新情報 `latest.json` の必須キーは `version`、`platforms.[target].url`、`platforms.[target].signature`。差分（delta、patch）の記述は無い。「書いていない」ことによる確認で、非対応と明言した箇所は見つからなかった。
- 実装（`tauri-plugin-updater` 2.10.1 の `src/updater.rs`）で確かめた。`download` は `.get(self.download_url.clone())` で全体を取得し、`buffer.extend(chunk)` でメモリに全部ためてから `verify_signature(&buffer, ...)` で署名を検証する。差分のコードは無い。macOS の `install_inner` は、メモリ上のバイト列を `GzDecoder` と tar で一時ディレクトリに展開し、現在のアプリをバックアップへ移して置き換える。
- 帰結: 教材を 1 か所直しても、音声を含む `.app.tar.gz` 全体をダウンロードし直す。AAC は圧縮されないので、音声のサイズがそのまま更新のサイズになる。ダウンロード全体を一度メモリに持つので、200MB 級の更新ではメモリも 200MB 以上使う。更新中は、インストール済みのアプリ、展開後のアプリ、バックアップで、一時的にアプリ 2〜3 個分のディスクが要る。

## 判定

基準は「インストーラーと更新のサイズが許せる範囲に収まる（範囲は検証時に決める）」。許せる範囲を決めるのはユーザーなので、OK / NG は付けない。事実として言えることは次のとおり。

- 仕様どおり（AAC 64kbps）だと、インストーラーも更新も約 220MB になる。教材を 1 文字直すだけでも 220MB をダウンロードする（差分更新が無い）。
- 音声以外（Tauri の実行ファイル、JS、フォント）は、aarch64 で約 3.6MB、ユニバーサルでも約 5.4MB。サイズの大半は音声で、音声の形式が支配的。
- 形式を変えるだけで、HE-AAC v1 24k なら約 94MB（半分以下）、AAC-LC 32k なら約 113MB。

## 残った事項

### ユーザーの判断が要る点

選択肢ごとのインストーラー（`.dmg`）と更新（`.app.tar.gz`）の見積もり。1MB = 10^6 バイト。音声以外の基準は、Tauri の `.dmg`（aarch64、実測）1.96MB、JS・CSS（gzip）0.99MB、フォント（Murecho 可変 + Red Hat Mono 可変、woff2）0.65MB の合計 3.6MB。ユニバーサルなら Tauri の部分が約 3.8MB になり、合計 5.4MB（見積もり）。教材のテキストと動画の TSX は含まない（未測定）。

| 音声の設定 | 音声（142 本） | インストーラー / 更新（aarch64） | 同（ユニバーサル、見積もり） | 音質 |
|---|---:|---:|---:|---|
| AAC-LC 64k（仕様。ffmpeg） | 217MB | 約 221MB | 約 223MB | 未確認 |
| AAC-LC 48k | 166MB | 約 169MB | 約 171MB | 未確認 |
| AAC-LC 32k | 113MB | 約 116MB | 約 118MB | 未確認 |
| HE-AAC v1 32k（afconvert、詰め直し後） | 132MB | 約 136MB | 約 137MB | 未確認 |
| HE-AAC v1 24k（同） | 94MB | 約 98MB | 約 100MB | 未確認 |
| Opus 32k（参考。`.m4a` ではなくなる） | 105MB | 約 108MB | 約 110MB | 未確認 |
| Opus 24k（参考） | 80MB | 約 84MB | 約 85MB | 未確認 |
| 音声を同梱せず、別に配る場合（参考） | 0 | 約 4MB | 約 5MB | |

- 章末復習 23 本の動画も同じ長さで作るなら、音声の列に約 1.16（165 / 142）を掛ける。
- 更新のたびに「更新」の列を全部ダウンロードする。217MB の構成で月に 4 回更新すると、月に約 0.9GB。

判断が要る点:

1. 許せるインストーラー・更新のサイズの上限。
2. 音声の形式（AAC-LC 64k のままか、48k / 32k、HE-AAC v1 か）。音質は人が聞いて決める必要がある。
3. HE-AAC の `.m4a` が WKWebView で再生できることは、再生を確かめる別の Issue で確かめる。再生できないときは、AAC-LC 32k が次の候補になる。
4. 音声のエンコーダー。ffmpeg の標準 `aac` は ffmpeg だけで作れる。HE-AAC は macOS の afconvert でしか作れなかったので、音声を作る環境が macOS であることが前提になる。
5. ユニバーサルバイナリ（Intel Mac 対応）にするか。差は約 +1.9MB で、音声に比べて小さい。

### 確かめられなかった項目

- 聞いた音質。
- WKWebView での HE-AAC の再生（macOS 13 を含む）。ffmpeg が 2 チャンネルとして扱う件で、WebView 側でチャンネル数・長さ・タイミングがずれないかも、再生の検証で見る。
- ユニバーサルバイナリの実測（x86_64 のターゲットを入れていない）。
- 実際のアプリの実行ファイルの増加（プラグインと自前のコード）。
- 教材のテキスト・動画の TSX の合計サイズ。
- 教材が使う字によるフォントのサブセット化、フォントのライセンスの確認。
- Windows。当面 macOS に集中する方針のため、確かめていない。

### 仕様の変更案

- [ナレーション音声](../spec/content-format/video/voice.md) は「AAC、モノラル、64kbps」とだけ書き、エンコーダーと、1 文 1 ファイルのオーバーヘッドを書いていない。afconvert の出力は 1 ファイル 4KB のオーバーヘッドがあり、6,248 ファイルで約 25MB になる。ffmpeg の標準 AAC なら 1.6KB。エンコーダーと、詰め直しの有無を書くとよい。
- [ADR 0063](../decisions/0063-generate-narration-with-voicepeak-and-bundle.md) の「64kbps に圧縮すると約 0.8MB」は、1 分 44 秒の動画全体に対する概算で、音声だけの 85.9 秒では 0.73MB（ffmpeg 64k）。1 本 3 分の見積もりにはほぼ影響しない。「WAV は 7.9MB」は、実測の 8,248,986 B（7.87MiB）と合っている。
- [画面の未決事項](../spec/screens/open-questions.md) の「書体の同梱方法」「日本語の書体を同梱したときのファイルの大きさ」は、Murecho の可変フォントの woff2 が約 0.62MB、Red Hat Mono が約 0.03MB、サブセット化の効果は約 0.1MB と分かった。決まったら ADR を追加する。
- [配布と更新](../spec/non-functional/distribution.md) に更新サイズの見込みの記述は無い。「教材はアプリに同梱し、アプリの更新とともに更新する」ので、更新のサイズがインストーラーのサイズとほぼ同じになることを書く余地がある。ただし、音声の形式の決定しだいで、[ADR 0037](../decisions/0037-bundle-content-and-auto-update.md) と [ADR 0063](../decisions/0063-generate-narration-with-voicepeak-and-bundle.md) を見直す可能性がある。
