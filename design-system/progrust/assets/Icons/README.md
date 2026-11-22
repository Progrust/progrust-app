# Icons

Lucide（lucide-static 0.460.0、ISC ライセンス。`LICENSE.txt`）の SVG。Progrust の画面で使うものだけを置いている。

- 線は `currentColor`、24px の viewBox、端と角は丸い。`<img>` で表示すると線は黒になるので、画面ではインラインの SVG か `lucide-react` で使い、文字の色（`ink`・`muted` など）に合わせる。
- 線幅は 16px 以上で 2.2、14px 以下で 2.4 に上書きする（ファイルは Lucide の既定の 2 のまま）。

| アイコン | 使う場所 |
|---|---|
| `search.svg` | ヘッダーの検索窓 |
| `refresh-cw.svg` | ヘッダーのジョブ数 |
| `chevron-left.svg` / `chevron-right.svg` | 戻る、相互参照のリンク |
| `chevron-down.svg` | セレクト、折りたためる一覧 |
| `arrow-right.svg` | 質問の送信 |
| `x.svg` | 閉じる、不正解、失敗したテスト |
| `plus.svg` | ファイルを追加 |
| `check.svg` | 正解、通ったテスト、チェックボックス |
| `circle-check.svg` | 完了の通知 |
| `circle-alert.svg` / `circle-x.svg` | エラー、失敗の通知 |
| `triangle-alert.svg` | 環境の警告 |
| `info.svg` | 補足の案内 |
| `loader-circle.svg` | 実行中、回答の作成中、準備中（回転させる） |
| `clock.svg` | 目安の時間、待機中のジョブ |
| `play.svg` / `square.svg` | 実行・動画の再生 / 停止（画面では塗りつぶした三角と四角にする） |
| `rotate-ccw.svg` | 初期コードに戻す、動画を 5 秒戻す |
| `house.svg` / `book-open.svg` / `wand-sparkles.svg` / `settings.svg` | ホーム / レッスン / 問題生成 / 設定（メニューにアイコンを付ける場合の候補） |
| `message-circle.svg` | 質問パネルを開く |
| `terminal.svg` | コンソール |
| `list-checks.svg` | 判定結果 |
| `file-code.svg` / `folder-open.svg` / `external-link.svg` | ファイル、学習記録を開く、外部のリンク |
