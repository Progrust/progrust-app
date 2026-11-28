# Banner

トップページの上に出す、環境の警告（`cargo` / `claude` が見つからない、初回準備中・失敗）。（新規の案。D2 のモックアップにない部品を、D2 の面・角丸・文字の決まりで作った）

- 角 `radius-panel` の面に、アイコン・文・操作を 1 行で並べる。
- 見つからない: `warning-soft` に `triangle-alert`（`warning`）。準備中: `.pg-banner--info`（`panel`）に回転する `loader-circle`。失敗: `.pg-banner--ng`（`danger-soft`）に `circle-alert`。
- 文はまず何が起きているか、次に学習への影響を書く（「AI の機能は使えません。レッスンは続けられます」）。
- 導入のコマンドを示すとき（`clang` が見つからない。仕様の `architecture/c-toolchain.md`「見つからない場合」）は、操作のボタンを置かず、文の中にコマンドを `.pg-cmd` で書く。`.pg-cmd` は書体 `code`・`raised` の地の短い帯で、押すとコマンド全体が選ばれる（`user-select: all`）。ボタンにしないのは、アプリがコマンドを実行せず、URL も開かないからである。文は、何が見つからないか、学習への影響、ターミナルでコマンドを実行すること、導入した後に設定画面の「再検出」を押すこと、の順に書く（例: 「C のコンパイラ（clang）が見つかりません。C の演習は実行・提出できません。ターミナルで `xcode-select --install` を実行して Command Line Tools を導入し、設定画面の「再検出」を押してください」）。面とアイコンは「見つからない」と同じ。
