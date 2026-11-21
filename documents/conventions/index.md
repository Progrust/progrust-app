# 運用ルール

* [ドキュメントの置き場所と構成](bundle-structure.md) - ドキュメントを `documents/` に集約すること、`spec/`・`decisions/`・`conventions/`・`verification/` の役割、asset（`.md` 以外のファイル）の扱い、`log.md`・`README.md` を置かないこと、一時作業用ディレクトリを参照しないことを定める。
* [frontmatter の書き方](frontmatter.md) - concept の frontmatter で使うキー（`type`・`title`・`description`・`status`・`superseded_by`）とその順序、値の引用符、7つの `type` の語彙と使い分け、使わないキーを定める。
* [文書の分割単位と名前の付け方](splitting.md) - 1つの concept に1つのトピックを書くこと、約200行を超えるときの分け方、機能のディレクトリに置く `scope.md`・`acceptance.md`・`open-questions.md`、用語集と `overview.md` の単位、ファイル名・`title`・`description` の付け方、本文の見出しの書き方を定める。
* [参照とリンクの書き方](links.md) - 他の文書への参照を Markdown リンクで書くこと、本文の相対パスと frontmatter の `/` 始まりのパス、コードブロックの中でのパスの書き方、仕様と ADR のリンクの向き、仕様に関係する ADR の探し方、ファイルを移動・改名したときのリンクの直し方を定める。
* [index.md の書き方と更新](index-files.md) - 各ディレクトリの `index.md` に書ける行（見出し行と `* [title](パス) - description` の行）、下位ディレクトリの載せ方、concept を追加・削除・改名したときの `index.md` の直し方、バンドルのルートの `index.md` の frontmatter を定める。
* [仕様の書き方](spec-writing.md) - 仕様には現時点の事実を書き、どの仕様も変更できるものとすること、同じ事実を複数の場所に書かないこと（SSOT）、仕様の理由を ADR に書くこと、機能のディレクトリの構成、新しい用語を用語集に追加することを定める。
* [ADR の運用ルール](decision-records.md) - 設計判断記録（ADR）に記録する判断、ファイル名と番号、frontmatter での状態（`status`・`superseded_by`）の表し方、本文の書き方と関連仕様に載せる仕様、テンプレート、作成後に変えてよい範囲、置き換えと廃止、仕様の「未決事項」との関係、`decisions/index.md` の構成、古い ADR の本文に出る仕様書の名前の読み方を定める。
* [ドキュメントの検査](checks.md) - `documents/` を変更したときに `python3 tools/check-documents.py` を実行すること、出力と終了コードの読み方、検査する項目と規則名、指摘の直し方、公式の validator（`tools/okf/validate.py`）の置き場所を定める。
* [Issue の起票](issues.md) - AI エージェントが作業中に見つけた不具合・問題点・課題を GitHub の Issue に起票するときの決まり（起票する問題の範囲、一覧のための Issue を作らずラベルで一覧すること、確認せずに起票すること、タイトルと本文の書き方、`AI起票`・`未判断`・`保留` と種類のラベル、重複の確かめ方、閉じた Issue と同じ問題を見つけたときに起票するかどうか、`gh issue create` の手順、起票した後の扱い、保留にするときと閉じるときにコメントに書くこと）を定める。
* [技術検証の結果の書き方](verification-reports.md) - 技術検証の結果を `verification/` に1つの Issue につき1文書で置くこと、ファイル名、`type: Verification Report` の title と description の形、本文の先頭の行と見出し（環境・方法・結果・判定・残った事項）、試作コードに頼らずに本文だけで分かるように書くこと、仕様・ADR との関係、検証し直したときの書き換え方を定める。
