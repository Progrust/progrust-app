---
type: Decision
title: "ADR 0092"
description: "文の音声を、AAC の .m4a・1 文 1 ファイルのまま、Remotion の Html5Audio で鳴らす"
---

- 日付: 2026-11-21
- 関連仕様: [動画のタイムラインと合図](../spec/content-format/video/timeline-and-cues.md)、[動画の音声](../spec/content-format/video/voice.md)
- 置き換え: なし

## 背景

1 文 1 ファイルの音声を Player の中で鳴らす部品は、Remotion 4 では 2 つある。`@remotion/media` の `Audio`（WebCodecs でデコードし、Web Audio で鳴らす）と、`remotion` の `Html5Audio`（`<audio>` 要素で鳴らす）である。どちらを使うかを決める必要があった。あわせて、音声の形式（AAC の `.m4a`）を対応環境の WebView で再生できるかが、教材データ形式の未決事項として残っていた。これらを Issue #2 の技術検証で確かめた（[1 文 1 ファイルの音声の再生と同期の技術検証](../verification/issue-2-m4a-audio-sync.md)）。

- 対応環境の WKWebView では、ffmpeg で作った `.m4a`（AAC-LC、モノラル、64kbps）を、どちらの部品でも再生できた。
- 計測では、音声と絵のずれは `@remotion/media` がおおむね ±50ms 以内だった。Html5Audio は音が約 100ms 遅れ、再生速度を変えると最大 210ms 遅れた。
- 作者が実際に見て聞いたところ、どちらの部品のずれも、1 倍、速度を変えたとき、シークの直後のどれも特に気にならなかった。動画には口の動き（口パク）などがないので、大きくずれていない限り気付けない。
- `@remotion/media` では、1 倍の再生で、文の言い始めがダブって聞こえる場所があった。Html5Audio で同じ場所を聞くと、ダブらなかった。Html5Audio は、速度を変えて聞いても、声の高さと音質が気にならなかった。計測のログでも、`@remotion/media` は、どの文でも頭の 100ms ほどの中で、ファイルの中の位置が約 21〜43ms 違う 2 つの断片を、重なる時刻に鳴らす予定にしていた。

## 決定

- 音声の形式（AAC の `.m4a`）は、対応環境の WebView で再生できると確かめたので、[ADR 0063](0063-generate-narration-with-voicepeak-and-bundle.md) の形式を見直さない。1 文 1 ファイルの並べ方も変えない（[動画の音声](../spec/content-format/video/voice.md)）。
- アプリは、文の音声を Remotion の `Html5Audio` で鳴らす（[動画のタイムラインと合図](../spec/content-format/video/timeline-and-cues.md)）。
- 音声と絵のずれの許せる量は、数値では決めない。見て聞いて気にならなければよいとする。

## 検討した他の案

- 案: `@remotion/media` の `Audio` で鳴らす
  - 採用しなかった理由: 文の言い始めがダブって聞こえた。ずれは Html5Audio より小さいが、どちらのずれも気にならなかったので、決め手にならなかった
- 案: 許せるずれの量を数値で決める（例: ITU-R BT.1359 の、音が先 45ms・音が後 125ms）
  - 採用しなかった理由: 口の動きなどがないので、大きくずれていない限り気付けない

## 影響

- 音は絵より約 100ms 遅れたまま進み、再生速度を変えると 130〜170ms（最大 210ms）遅れる。Remotion は、`<audio>` と絵のずれが `acceptableTimeShiftInSeconds`（既定 0.65 秒）を超えるまで位置を合わせ直さないので、このずれは直らずに残る。
- 再生速度を変えても、声の高さは WebKit の `<audio>` が保つ（`Html5Audio` の `preservePitch` は既定で true）。
- 教材データ形式の未決事項のうち、音声の形式を WebView で再生できることの確認は、この判断で解消した。
