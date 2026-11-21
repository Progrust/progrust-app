---
type: Decision
title: "ADR 0127"
description: "文の音声を ffmpeg に組み込みの AAC エンコーダーで .m4a に変換し、durations.json の seconds と cue-reference.json の長さを ffprobe の format の duration で測る"
---

- 日付: 2026-11-21
- 関連仕様: [動画の音声](../spec/content-format/video/voice.md)「音声を作るツール」、[動画のタイムラインと合図](../spec/content-format/video/timeline-and-cues.md)
- 置き換え: なし

## 背景

音声を作るツールは、切り詰めたあとの音声を `.m4a` に変換し、変換後のファイルの長さを `durations.json` の `seconds` に書く。どのエンコーダーで変換し、長さをどう測るかは、決めていなかった（Issue #36）。`seconds` は、文とシーンの開始位置を決めるのに使う。

Issue #2 の技術検証で、同じ WAV から作った `.m4a` でも、エンコーダーと測り方によって長さが変わることが分かった（[1 文 1 ファイルの音声の再生と同期の技術検証](../verification/issue-2-m4a-audio-sync.md)「AAC の前後の無音（priming）と durations.json の seconds」）。21 文で比べた結果は次のとおり。

- ffmpeg の `aac` で作った `.m4a`: ffprobe の `format` の `duration` は、元の WAV の長さと一致した。WebKit の `<audio>` が報告する長さは、WAV より 22.67ms 短かった（原因は調べていない）。`<audio>` で鳴らしたときの声の始まりは、WAV と 2ms 以内だった。
- macOS の afconvert で作った `.m4a`: ffprobe の `format` の `duration` は、WAV より 44〜65ms 長かった。`<audio>` が報告する長さは、WAV と一致した。

アプリは、文の音声を `<audio>`（Remotion の `Html5Audio`）で鳴らす。この判断のときにユーザーが聞いて確かめた音声は、ffmpeg の `aac` で作ったものだった（[ADR 0092](0092-play-narration-with-html5audio.md)）。動画は 30fps で、1 フレームは約 33.3ms である。秒で表した長さは、フレーム数に四捨五入して使う。

Claude は、ffmpeg の `aac` で変換し、ffprobe で測ることを推奨した。ユーザーは「その他の内容は推奨通りで問題ありません。」と示し、推奨を承認した。以下の理由は Claude が書いた。

## 決定

- 切り詰めたあとの音声を、ffmpeg に組み込みの AAC エンコーダー（`-c:a aac -b:a 64k -ac 1`）で `.m4a` に変換する（[動画の音声](../spec/content-format/video/voice.md)「音声を作るツール」）。
  - 理由: ユーザーが聞いて確かめ、ADR 0092 の判断のもとになった音声が、この作り方だった。音声を作るツールは、もともと ffmpeg を使うことにしている。
- `durations.json` の `seconds` には、変換後の `.m4a` を ffprobe の `format` の `duration` で測った値を書く。
  - 理由: 検証した 21 文のすべてで、この値が元の WAV の長さと一致した。変換後のファイルだけから測れるので、リポジトリに入れた `.m4a` から、後で同じ値を測り直せる。
- `cue-reference.json` に書く音声の長さも、同じ測り方の値にする。
  - 理由: アプリは、合図の位置を、`seconds` と `cue-reference.json` の長さの比で決める。測り方が違うと、音声を作り直していなくても比が 1 にならない。

## 検討した他の案

- 案: macOS の afconvert で変換する
  - 採用しなかった理由: `<audio>` が報告する長さは WAV と一致したが、ffprobe の値が 44〜65ms 長くなり、変換後のファイルから測った値が実際の長さと合わない。macOS でしか使えない。ユーザーが聞いて確かめた音声でもない
- 案: `seconds` に、WebKit の `<audio>` が報告する長さを書く
  - 採用しなかった理由: 音声を作るツールは、作者の環境のコマンドラインで動く。WebKit を動かさないと測れない値は、ツールでは測れない
- 案: `seconds` に、変換する前の WAV の長さを書く
  - 採用しなかった理由: 検証では ffprobe の値と同じだったが、WAV はリポジトリに入れないので、後で測り直せない

## 影響

- `<audio>` で鳴らしたときの音声は、`seconds` より 22.67ms 短く終わる見込みである。1 フレーム（約 33.3ms）より短く、`seconds` のほうが長いので、音声が文の終わりで切れることはないと見ている。これは、21 文を、1 つの版の ffmpeg（9.0.2）と WebKit で確かめた結果からの推定である。
- ffmpeg の版を上げるとき、またはエンコーダーの指定を変えるときは、ffprobe の値と元の音声の長さが一致することを確かめ直す。
- 作者の環境には、ffmpeg と一緒に ffprobe が要る。
