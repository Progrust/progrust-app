---
type: Specification
title: "動画の音声"
description: "VOICEPEAK で作る音声ファイルの形式と置き場所、声の設定（`voice.json`）、読みの対応表（`readings.json`）、音声の長さ（`durations.json`）、音声を作るツールの動作（「、」にあたる無音の見つけ方、`.m4a` への変換のしかたと長さの測り方を含む）を定める。"
---

- ナレーションの音声は、教材の作者の環境で、VOICEPEAK の CLI から作る。作った音声はリポジトリに入れ、アプリに同梱する。学習者の環境に VOICEPEAK は要らない。CI では音声を作らない。
- 音声は 1 文 1 ファイルとし、`voice/{文の ID}.m4a` に置く。形式は AAC、モノラルとし、ビットレートは 64kbps を目安とする。

**声の設定（`content/voice.json`）**

```json
{
  "narrator": "Natsuki Karin",
  "speed": 120,
  "pause": 0.2,
  "emotion": { "hightension": 20, "buchigire": 5, "nageki": 0, "sagesumi": 20, "sasayaki": 80 }
}
```

| フィールド | 説明 |
|---|---|
| `narrator` | VOICEPEAK のナレーター名。`Natsuki Karin` は夏色花梨である |
| `speed` | 読む速さ（VOICEPEAK の速さの値） |
| `pause` | 「、」の間の長さ（秒。0 以上） |
| `emotion` | 感情の項目名と値。項目はナレーターごとに決まる |

- 声の設定は全レッスンで共通とする。
- 声の設定を変えたら、全レッスンの音声を作り直す。

**読みの対応表（`content/readings.json`）**

```json
{
  "Rust": "ラスト",
  "String": "ストリング",
  "値": "あたい"
}
```

- 読み間違えやすい語と、コードの決まった語（キーワード・型名・関数名など。[ナレーションでのコードの読み方](../../content-guidelines/readings.md)）の、`say` に書く読みの対応を、全レッスン共通で持つ。キーが語、値が読みである。
- 作者は `say` を書くとき、この表に従う（[動画の作成ルール](../../content-guidelines/video.md)「台本」）。音声を作るツールは `say` を書き換えない。

**音声の長さ（`durations.json`）**

```json
{
  "m1": {
    "seconds": 3.757,
    "say": "二行目で、変数エスワンにストリングのあたいを入れました。",
    "voice": {
      "narrator": "Natsuki Karin",
      "speed": 120,
      "pause": 0.2,
      "emotion": { "hightension": 20, "buchigire": 5, "nageki": 0, "sagesumi": 20, "sasayaki": 80 }
    }
  }
}
```

| フィールド | 説明 |
|---|---|
| `seconds` | その文の音声ファイル（`voice/{文の ID}.m4a`）の長さ（秒）。測り方は下の「音声を作るツール」に定める |
| `say` | その音声を作ったときの `say` |
| `voice` | その音声を作ったときの声の設定（`voice.json` と同じ形） |

- キーは文の ID とする。台本のすべての文について 1 件ずつ持つ。
- 音声を作るツールが書く。作者は手で書き換えない。

**音声を作るツール**

`tools/` に置く。作者の環境の VOICEPEAK と ffmpeg（ffprobe を含む）を使う。実行するコマンドは `npm run content:voice`（[開発環境と依存の版](../../architecture/development-environment.md)「開発で使うコマンド」）とする。

- 台本の文を 1 文ずつ順に VOICEPEAK に渡して、音声を作る。VOICEPEAK を同時に 2 つ以上動かさない。
- `durations.json` の `say`・`voice` が台本の `say`・`voice.json` と同じで、音声ファイルがある文は、作り直さない。すべての文を作り直す指定もできる。
- `say` が 140 字を超える文がある台本と、[動画の台本（script.json）](script-json.md) の形式に合わない台本は、エラーにして、そのレッスンの音声を作らない。ほかのレッスンの音声は作る。
- 「、」の間は次のように作る。`say` をそのまま VOICEPEAK に渡して音声を作り、できた音声の中の「、」にあたる無音を `pause` 秒まで切り詰める。`pause` 秒より短い無音と、文の途中の「。」にあたる無音は、そのままにする。
- 「、」にあたる無音は、次のように見つける。
  - 音量が −50dBFS より小さいサンプル（16 ビットのサンプルでは、絶対値が 32768 × 10^(−50/20) より小さいもの）が続くところを、無音とする。
  - 音声の先頭と末尾に接する無音を除き、0.2 秒以上続く無音を、句読点にあたる無音とする。
  - 句読点にあたる無音と、`say` の文の途中の句読点を、前から順に 1 つずつ対応づける。「、」（`，`・`,` を含む）に対応した無音が「、」にあたる無音で、「。」（`．`・`？`・`！`・`?`・`!` を含む）に対応した無音が「。」にあたる無音である。続けて並んだ句読点（間の空白を含む）は 1 つと数え、「。」の類を含めば「。」とする。`say` の先頭と末尾の句読点は数えない。
  - 文の途中の句読点の数と、句読点にあたる無音の数が違う文は、音声を作らずにエラーにする。ほかの文の音声は作る。
- 「、」にあたる無音を切り詰めるときは、無音の真ん中を取り除き、前後に `pause` 秒の半分ずつを残す。
- 切り詰めたあとの音声を、ffmpeg に組み込みの AAC エンコーダー（`aac`）で `.m4a` に変換する。

  ```sh
  ffmpeg -i {切り詰めたあとの音声}.wav -c:a aac -b:a 64k -ac 1 {文の ID}.m4a
  ```

- 変換後の `.m4a` の長さを、ffprobe の `format` の `duration` で測り、`durations.json` の `seconds` に書く。`cue-reference.json` に書く音声の長さも、同じ測り方の値にする。
- 台本にない文の音声ファイルと、`durations.json`・`cue-reference.json` の記録は消す。
- `cue-reference.json` の書き方は [動画のタイムラインと合図](timeline-and-cues.md) に定める。
