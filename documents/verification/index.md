# 技術検証の結果

* [隠しテストを読み込む #[path] の解決の技術検証](issue-14-hidden-test-path-resolution.md) - 隠しテストを読み込む `#[path]` が `judge.target` の場所によらず解決されるかを Issue #14 で検証した結果（`../` の数の決め方、絶対パス案、テスト名へのモジュール経路の付加、`use super::*;` の基準）を記す。
* [test 方式のテスト結果の読み取りの技術検証](issue-15-test-result-parsing.md) - test 方式の libtest の出力からテストごとの結果と panic メッセージを確実に読み取れるかを Issue #15 で検証した結果（出力形式、学習者の出力との混合、process::exit・abort・無限ループ、10 秒の制限での打ち切り、1 回起動する方式とテストごとに別プロセスで実行する方式の比較）を記す。
* [配布物と更新のサイズの技術検証](issue-20-distribution-size.md) - 音声・フォント・フロントエンド JS・Tauri の実行ファイルを含む配布物と更新のサイズを Issue #20 で検証した結果（音声の形式ごとのサイズ、142 レッスン分の見積もり、HE-AAC の扱い、Murecho のサイズ、Monaco と Remotion の JS、Tauri の updater が差分更新をしないこと、選択肢ごとのインストーラーのサイズ）を記す。
