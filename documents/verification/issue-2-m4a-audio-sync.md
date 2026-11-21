---
type: Verification Report
title: "1 文 1 ファイルの音声の再生と同期の技術検証"
description: "1 文 1 ファイルの .m4a を Player の中で同期して再生できるかを Issue #2 で検証した結果（@remotion/media の Audio と Html5Audio の動作、音声と絵のずれの表、0.75〜2 倍速での声の高さ、AAC の前後の無音と durations.json の seconds の食い違い、@remotion/media が文の頭で音を二重に鳴らすこと、人が聞いて Html5Audio を選び、許せるずれの量を数値で決めないとした判断）を記す。"
---

- 日付: 2026-11-21
- Issue: [#2](https://github.com/Progrust/progrust-app/issues/2)
- 確かめた OS: macOS 27.2.1（Apple M1）
- 関連仕様: [動画の音声](../spec/content-format/video/voice.md)、[動画のタイムラインと合図](../spec/content-format/video/timeline-and-cues.md)、[教材データ形式の未決事項](../spec/content-format/open-questions.md)、[ADR 0063](../decisions/0063-generate-narration-with-voicepeak-and-bundle.md)、[ADR 0066](../decisions/0066-trim-comma-pauses-to-0-2-seconds.md)

## 環境

- macOS 27.2.1、Apple M1、メモリ 16GB、rAF 58.8Hz。WebKit.framework 22625.2.8.11.9（Safari 27.2 と同じ）。
- rustc 1.99.1、node v25.9.0、ffmpeg 9.0.2、afconvert（macOS 付属）。
- Tauri CLI 2.12.6、tauri 2.12.7、wry 0.57.3。React 19.3.1、Vite 8.3.8。
- remotion・@remotion/player・@remotion/media 4.0.545、mediabunny 1.66.0。
- 計測時のウィンドウは、全回で表示・フォーカス済み・隠れていない状態を計測の前後で確かめ、rAF の間隔が 50ms を超えた回はなかった（詳細は [Issue #1 の検証結果](issue-1-remotion-player-webview.md)「環境」）。

## 方法

### 作ったもの

Issue #1 と同じ試作（Tauri の最小アプリに Remotion の `Player` を置き、第 4 章のレッスン 3175 フレーム・台本 21 文を再生する）を使った。音声の部品（`@remotion/media` の `Audio`、`remotion` の `Html5Audio`）とファイル（ffmpeg の m4a、afconvert の m4a、WAV）を inputProps で切り替えた。m4a は次のコマンドで WAV から作った。

```sh
ffmpeg -i x.wav -c:a aac -b:a 64k -ac 1 x.m4a        # AAC-LC、モノラル、48kHz
afconvert -f m4af -d aac -b 64000 -c 1 x.wav x.m4a   # 比較用
```

### 計測の仕組み

- release ビルドの .app を環境変数（`PROGRUST_BENCH=sync` など）付きで起動する。ページ（`tauri://localhost`）が自分で計測し、結果を Tauri のコマンドで JSON に書いて自分で終了する。起動時と計測の前にウィンドウを前面に出して常に手前に置く。
- 絵の時刻: 各フレームが最初に表示された rAF の時刻に、局所（±700ms）で直線を当てはめて求めた。「絵が表示された」は DOM 反映のあとの rAF までとする（定義は [Issue #1 の検証結果](issue-1-remotion-player-webview.md)「方法」）。
- 音の時刻（`@remotion/media`）: AudioContext の出力に並列につないだ AnalyserNode で RMS が 0.01 を超えた時刻。各文の WAV の頭の無音の長さから「声が始まるべきフレーム」を決め、その絵の時刻と比べた。あわせて `@remotion/media` の verbose ログに出る予定の時刻（scheduled）も絵と比べた。
- 音の時刻（Html5Audio）: Player の共有の `<audio>` の `currentTime` と、そのときの絵の位置から計算した、文の中の秒の差。
- 正の値は音が絵より遅いこと。単位は ms。
- 声の高さ: 440Hz の正弦波の .m4a を各速度で鳴らし、出力のいちばん強い周波数を測った（聞いて確かめる代わりの指標）。
- 文の頭の音の重なり（`@remotion/media`）: verbose ログに出る、断片（約 21ms）ごとの予定の時刻（AudioContext の時刻）とファイルの中の秒を使った。鳴らす時間が重なるように予定された 2 つの断片のうち、ファイルの中の秒が違うものを数えた。記録したのは開始の予定だけで、断片を止めたかは記録していない。

### 人が聞いた確認

計測のあと、ユーザーが release ビルドの試作を、人が触る画面で起動して聞いた。出力は Mac の内蔵スピーカー。音声のファイルは ffmpeg の m4a にし、ヘッダーで `@remotion/media` の Audio と Html5Audio を切り替えた。音声と絵のずれ（1 倍、0.75 倍・2 倍など速度を変えたとき、前の文・次の文・±5 秒・シークバーでのシークの直後）と、速度を変えたときの声の高さと音質を比べた。Player の `preservePitch` は true、Html5Audio の `preservePitch` も true にした。

## 結果

### .m4a を再生できるか

再生できた。

- `canPlayType('audio/mp4; codecs="mp4a.40.2"')` は `"probably"`、`MediaSource.isTypeSupported` は true。
- `<audio>` で ffmpeg・afconvert の m4a と WAV を再生でき、3 回とも `currentTime` が 300ms で 0.30 秒進んだ。
- この Mac の WebKit には WebCodecs の AudioDecoder があり、AAC（mp4a.40.2、48kHz、1ch）に対応している。
- ジェスチャーなしで AudioContext が `resume()` で running になる。wry の既定の `autoplay: true` で `mediaTypesRequiringUserActionForPlayback = None` になるため。

### 2 つの Audio 部品

- 4.0.545 で実在するのは、`remotion` の `Html5Audio`（`remotion` の `Audio` は非推奨の別名）と、`@remotion/media` の `Audio`。どちらも WKWebView で鳴った。
- `@remotion/media` の `Audio` は `from` と `durationInFrames` で位置を決める。mediabunny と WebCodecs でデコードし、Web Audio の AudioBufferSourceNode で鳴らす。
- Html5Audio は `durationInFrames` が「ソースの何フレームを鳴らすか」なので、位置は `Sequence` で決める。音は Player が用意した共有の `<audio>`（5 個）で鳴らす。
- AudioDecoder がない WebKit（macOS 13 と同じ状態）では、`@remotion/media` の Audio は自動で Html5Audio に切り替わる（[Issue #1 の検証結果](issue-1-remotion-player-webview.md)「macOS 13 の見込み」）。

### 音声と絵のずれ（`@remotion/media`）

文の声の始まり。中央値（範囲）。

| ファイル | 1 倍の通し（21 文） | 0.75 倍（6 文） | 1.25 倍 | 1.5 倍 | 2 倍 |
|---|---|---|---|---|---|
| ffmpeg の m4a（2 回） | −8.8・−11.0（−27〜+7） | −25〜−26（−48〜−2） | −6・−4（−34〜+8） | −26・−9（−49〜+3） | −11・−13（−24〜+1） |
| afconvert の m4a（1 回） | +16.1（+7.5〜+36） | +27（+5〜+37） | +0.2（−2〜+12） | −5（−9〜+3） | −4（−8〜+10） |
| WAV（1 回） | −8.7（−14〜+7） | −31（−101〜+16） | −11（−93〜+72） | −23（−90〜+77） | −5（−79〜+60） |

- ffmpeg の m4a の文の切れ目のずれは、1 倍で −27〜+7ms（音がわずかに先）、全速度で −49〜+8ms。
- verbose ログの予定の時刻と絵の差は、全速度・全回で中央値 −30〜−65ms、最大 65ms。シークの直後（7 か所 × 2 回）と、同じ文の中で後ろへ戻るシーク（4 か所 × 2 回）でも同じ範囲（中央値 −28〜−43ms、最大 52ms）だった。シークの直後に、音が別の位置から鳴ること（1 秒以上ずれた予定）は 0 件。
- AnalyserNode の値と予定の値には約 30ms の差がある（AnalyserNode の窓 2048 サンプル = 43ms と、AudioContext の時刻の対応の不確かさによる）。絶対値には ±30ms ほどの誤差がある。
- 出力の遅れ（`outputLatency` 20.7ms）と、画面に出るまでの遅れ（1 フレーム程度）は含まない。
- WAV は 1 倍以外でずれの幅が大きい（±100ms）。理由は調べていない。m4a を使うので影響はない。

### 音声と絵のずれ（Html5Audio）

`<audio>` の `currentTime` で見た値。

| ファイル | 1 倍の通し | 文の頭（21 文） | 0.75〜2 倍 | シークの直後（7 か所） | 同じ文の中で後ろへ戻るシーク（4 か所） |
|---|---|---|---|---|---|
| ffmpeg の m4a（2 回） | 中央値 +111・+110、最大 +159 | 中央値 +84・+83（+66〜+133） | 中央値 +128〜+169、最大 +210 | +87〜+107 | +51〜+94 |
| afconvert の m4a（1 回） | +100.5、最大 +108 | +74（+67〜+81） | +115〜+141、最大 +152 | +74〜+97 | +66〜+91 |

- Html5Audio の音は、絵より約 100ms 遅れたまま進む。速度を変えると 130〜170ms 遅れる。
- Remotion は、`<audio>` と絵のずれが `acceptableTimeShiftInSeconds`（既定 0.65 秒）を超えるまで位置を合わせ直さない。このため、このずれは直らずに残る。
- `currentTime` が実際に聞こえる位置と合っているかは分からない。WebKit のメディアの出力の遅れは含まない。
- AudioDecoder を消したまねの計測でも、Html5Audio と同じ性質だった（1 倍で中央値 +117ms、最大 +135ms。1.5・2 倍で中央値 +144〜+147ms、最大 +158ms。シークの直後 +52〜+104ms）。

参考: ITU-R BT.1359 は、音が先 45ms・音が後 125ms を検知の限界とする。許せるずれの量は Issue で「検証時に決める」とされており、決めるのはユーザー。見た目・聞いた感じでの判断は確かめていない。

### 0.75〜2 倍速での声の高さ

出力のいちばん強い周波数（440Hz の正弦波。2 回とも同じ値）。

| 部品と設定 | 0.75 倍 | 1 倍 | 1.25 倍 | 1.5 倍 | 2 倍 |
|---|---|---|---|---|---|
| `@remotion/media`、Player の `preservePitch={true}` | 439.8Hz | 439.9 | 439.8 | 439.9 | 439.8 |
| `@remotion/media`、`preservePitch={false}`（**Player の既定**） | 330.1 | 439.9 | 550.0 | 660.1 | 879.9 |
| `<audio>`、`preservesPitch=true`（Html5Audio の既定） | 440 | 440 | 440 | 440 | 440 |
| `<audio>`、`preservesPitch=false` | 330 | 440 | 550 | 660 | 880 |

- 0.75〜2 倍のすべての速度で再生できた。
- Html5Audio は、Player の中で `<audio>` の `playbackRate` を Player の速度にし、`preservesPitch=true` を設定していた（Player の中の要素の値で確認）。Html5Audio は Web Audio を通らないので AnalyserNode では測れず、表の `<audio>` の行は、Player を通さない `<audio>` を `createMediaElementSource` で測った値（WebKit 自身の動き）である。
- macOS 13 の WebKit には、unprefixed の `preservesPitch` が Safari 17.2 より前にはなく `webkitPreservesPitch` だけある（MDN の互換データ）。既定は true なので、声の高さは保たれる見込み（未確認）。

### 文の頭の音の重なり（`@remotion/media`）

計測（「音声と絵のずれ（`@remotion/media`）」と同じ回）の verbose ログで、鳴らす時間が重なるように予定され、ファイルの中の秒が違う 2 つの断片を数えた。

| ファイル | 1 倍の通し | 0.75〜2 倍 |
|---|---|---|
| ffmpeg の m4a（2 回） | 2 回とも、文の頭を検出できた 20 文（21 文のうち 1 文は検出できなかった）のすべてで重なる予定があった。位置はファイルの 64〜85ms、2 つの断片のファイルの中の秒の差は 21.4ms か 42.7ms | 0.75〜1.5 倍（各 5〜6 文）: 1 回目は全文、2 回目は 6 文中 3〜5 文。2 倍は文の頭の検出が欠けたので文の数は数えていない（重なる予定は 14〜19 か所）。位置は 16〜119ms、差は 21〜112ms |
| afconvert の m4a（1 回） | 21 文のすべてで重なる予定があった。位置は 43〜85ms、差は 21.4〜64.0ms | 0.75 倍は 6 文中 5 文、1.25・1.5 倍は 6 文のすべて、2 倍は検出できた 5 文のすべて |

- 文の声は、ファイルの 34〜102ms から始まる（各文の WAV の頭の無音の長さ）。重なりは、声の出だしに当たる。たとえば m7（「なぜ、使えなくするのでしょうか。」、フレーム 1476 から）は、声が 50ms から始まる。2 回とも、ファイルの 106.7ms からの断片が先に、85.3ms からの断片が約 2ms 後に予定され、約 20ms の間、重なっていた。
- シークの直後（7 か所 × 2 回）と同じ文の中で後ろへ戻るシーク（4 か所 × 2 回）では、重なる予定は 1 回のシークにつき 0〜1 か所だった。
- 原因は調べていない。Html5Audio は `<audio>` で鳴らすので、この仕組みでは鳴らさない。

### 人が聞いた結果

- 音声と絵のずれ: どちらの部品でも、1 倍、速度を変えたとき、シークの直後のどれも、特に気にならなかった。ユーザーの判断では、動画には口の動き（口パク）などがないので、大きくずれていない限り気付けない。
- 音質: `@remotion/media` では、1 倍の再生で、文の言い始めがダブって聞こえる場所があった。m7「なぜ、使えなくするのでしょうか。」の「なぜ」の言い始めが特に気になった。Html5Audio で同じ場所を聞くと、ダブらず、特に気にならなかった。
- 声の高さ: Html5Audio で 0.75 倍・2 倍などに変えて聞いても、声の高さと音質は特に気にならなかった。
- ユーザーは、Html5Audio を使うと決めた。許せるずれの量は、数値では決めず、見て聞いて気にならなければよいとした（[ADR 0092](../decisions/0092-play-narration-with-html5audio.md)）。

### AAC の前後の無音（priming）と durations.json の seconds

試作の durations.json の seconds は WAV の長さ。21 文で比べた。

| 測り方 | ffmpeg の m4a | afconvert の m4a |
|---|---|---|
| ffprobe の format の duration | WAV と一致（start_time 0） | **WAV より 44〜65ms 長い**（start_time 0.044） |
| afinfo | valid frames が WAV と一致。priming 1024 | valid frames が WAV と一致。priming 2112 |
| `decodeAudioData` の長さ | WAV より 0.4〜19ms 長い | WAV と完全に一致 |
| `decodeAudioData` の頭の位置（WAV との相互相関） | ずれ 0 サンプル | ずれ 0 サンプル |
| `<audio>` の duration | **WAV より 22.67ms（1088 サンプル）短い**（全文） | WAV と一致 |
| `<audio>` で鳴らしたときの声の始まり（3 文 × 3 回） | WAV と 2ms 以内 | WAV と 2ms 以内 |
| `@remotion/media` での声の始まり（1 倍） | WAV とほぼ同じ（−9〜−11ms と −8.7ms） | **WAV より約 25ms 遅い**（+16.1ms） |

- ffmpeg の m4a は、WebKit の `decodeAudioData`・`<audio>`・`@remotion/media` のどれでも頭の位置が WAV と同じになる（priming が取り除かれる）。長さの違いは末尾の無音の扱いで、聞こえる長さへの影響はない見込み。`<audio>` で短くなる原因は調べていない。
- afconvert の m4a は、WebKit 自身では WAV と一致するが、`@remotion/media`（mediabunny）では声が約 25ms 遅れる。2112 サンプルの priming のうち約 1024 サンプルしか取り除かれていない可能性がある（2112 − 1024 = 1088 サンプル = 22.7ms）。確かめてはいない。
- 長さを ffprobe か afinfo の valid frames で測り、エンコーダーに ffmpeg の `aac` を使う限り、durations.json の seconds と実際の長さのずれは出ない。

## 判定

| 基準 | 結果 |
|---|---|
| .m4a を再生できる | 満たした |
| シークの直後と文の切れ目でずれが目立たない | 満たした。Html5Audio は約 100ms（速度を変えると最大 210ms）音が遅れるが、人が見て聞いて気にならなかった。許せる量は数値では決めない（ADR 0092） |
| すべての再生速度で声の高さが変わらない | Html5Audio で満たした。代わりの指標で保たれ、0.75 倍・2 倍などで人が聞いても気にならなかった。`@remotion/media` は、文の言い始めがダブって聞こえた |

使う部品を Html5Audio にすれば基準をすべて満たすので、Issue を閉じる。

## 残った事項

判断が要る点: なし。Audio 部品と許せるずれの量は、ユーザーが聞いて決めた（「人が聞いた結果」、ADR 0092）。

確かめられなかった項目: なし。

仕様の変更案

- 動画の音声の仕様の「切り詰めたあとの音声を `.m4a` に変換し、変換後のファイルの長さを `seconds` に書く」は、エンコーダーと長さの測り方で値が変わる。エンコーダーを ffmpeg の `aac` に決め、長さは ffprobe（または afinfo の valid frames）で測ると書く案。Issue #36 に起票済み。2026-11-21 に、ffmpeg の `aac` で変換し、ffprobe の format の duration で測ると決まり、仕様を直した（[ADR 0127](../decisions/0127-encode-narration-with-ffmpeg-aac-and-measure-with-ffprobe.md)）。

検証中に見つけた別の問題（事実のみ）

- `tauri://` のアセットは Range 要求に 206 を返さず、200 で全体を返す。mediabunny が警告する。1 文 30〜50KB のファイルでは問題は出なかった。長い音声ファイルでは、読み直しが失敗するおそれがある（mediabunny の警告の文言）。Issue #35 に起票済み。2026-11-21 に、音声を Tauri のリソースに置いて asset:// で読むと決まった（[ADR 0128](../decisions/0128-serve-narration-from-resources-via-asset-protocol.md)）。
- WKWebView の `AudioContext.outputLatency` が、音を出す前は 1.7×10^14 のような壊れた値を返す（鳴らしたあとは 0.0207）。Remotion は `getAudioSyncAnchorTolerance` でこの値を使い、`@remotion/media` の同期の許容量を決めている。今回の計測では、ずれは大きくならなかった。
- `@remotion/media` では、Player が文ごとに `waiting` イベントを出す（[Issue #1 の検証結果](issue-1-remotion-player-webview.md)「残った事項」）。
