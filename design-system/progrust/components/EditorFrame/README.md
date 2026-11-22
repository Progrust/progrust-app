# EditorFrame

演習画面のエディタの枠。ファイルのタブ・追加ボタン・「初期コードに戻す」と、Monaco Editor を入れる領域。

- 枠は `bg` の塗り・`line` の枠・角 `radius-panel`。タブ列は高さ 46px で `line-faint` の下線。
- ファイルのタブ（`.pg-filetab`）は等幅 13px。選択中は `ink` の塗り。
- Monaco Editor のテーマは `code-*` のトークンから作る（地 `bg`、行番号 `code-line-number`、文字 `ink`）。書体は `code`（既定 14px）。
- この見本の中のコードは Monaco ではなく、色の割り当てを示すための静的な表示。
