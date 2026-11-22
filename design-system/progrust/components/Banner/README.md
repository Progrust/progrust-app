# Banner

トップページの上に出す、環境の警告（`cargo` / `claude` が見つからない、初回準備中・失敗）。（新規の案。D2 のモックアップにない部品を、D2 の面・角丸・文字の決まりで作った）

- 角 `radius-panel` の面に、アイコン・文・操作を 1 行で並べる。
- 見つからない: `warning-soft` に `triangle-alert`（`warning`）。準備中: `.pg-banner--info`（`panel`）に回転する `loader-circle`。失敗: `.pg-banner--ng`（`danger-soft`）に `circle-alert`。
- 文はまず何が起きているか、次に学習への影響を書く（「AI の機能は使えません。レッスンは続けられます」）。
