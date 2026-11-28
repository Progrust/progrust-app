---
type: Decision
title: "ADR 0271"
description: "Command Line Tools が入っていない Mac で、導入のダイアログを出さずに clang の有無を確かめる手順は用意せず、clang の検出は作者の Mac（Command Line Tools が入っている）で動けばよいとする"
---

- 日付: 2026-11-28
- 関連仕様: [C のツール（clang）への依存](../spec/architecture/c-toolchain.md)「検出」
- 置き換え: なし

## 背景

C の演習は学習者の Mac の `clang` でビルドし、アプリは起動時と「再検出」のときに、設定で指定したパス → `PATH` → `/usr/bin` の順に `clang` を探して、`clang --version` を起動する（[ADR 0266](0266-build-c-exercises-with-clang-and-check-memory-on-submit.md)）。Command Line Tools が入っていない Mac の `/usr/bin/clang` は、起動すると Command Line Tools の導入を促すダイアログを出す入口のプログラムなので、ダイアログを出さずに有無を確かめる方法が、アーキテクチャの未決事項に残っていた。

この方法を決めるための技術検証の Issue（#295）で、Claude は、作者の Mac（macOS 27.2.1、Apple M1）で次のことを確かめた。

- Command Line Tools が `/Library/Developer/CommandLineTools` に入っていて、Xcode は入っていない。
- `/usr/bin/clang` は `/usr/lib/libxcselect.dylib` を使う入口のプログラムで、`/Library/Developer/CommandLineTools/usr/bin/clang` を起動する。
- `xcode-select -p` は、環境変数を何も渡さなくても `/Library/Developer/CommandLineTools` を出力して、終了コード 0 で終わる。

Command Line Tools が入っていない状態は、この Mac のシステムのファイルを動かすか、仮想マシンを用意しないと作れない。Claude は、その用意の仕方をユーザーに尋ねた。

## 決定

- Command Line Tools が入っていない Mac で、導入のダイアログを出さずに `clang` の有無を確かめる手順は用意しない。`clang` の検出は、今の仕様のまま（設定で指定したパス → `PATH` → `/usr/bin`）とする（[C のツール（clang）への依存](../spec/architecture/c-toolchain.md)「検出」）。
  - 理由: ユーザーが決めた。今は一般公開を考えておらず、作者の Mac で動けばよい（[ADR 0090](0090-support-only-authors-mac-until-completion.md)）。作者の Mac には、Rust と Tauri のビルドのために Command Line Tools が入っている。
  - ユーザーは、あわせて、技術検証の Issue #295 を要らないものとし、削除するよう指示した。

## 検討した他の案

- 案: Command Line Tools が入っていない状態を用意して、起動せずに有無を確かめる方法（`xcode-select -p` の終了コード、Command Line Tools の決まった場所の `clang` の有無など）を技術検証で決める
  - 採用しなかった理由: 作者の Mac では起きない場合である。確かめるには、この Mac のシステムのファイルを一時的に動かすか、仮想マシンを用意する必要がある
- 案: `/usr/bin` を探索順から外し、Command Line Tools と Xcode の決まった場所だけを探す
  - 採用しなかった理由: 同じく、作者の Mac では起きない場合のための変更になる

## 影響

- Command Line Tools が入っていない Mac でアプリを起動すると、`clang` の検出で、`/usr/bin/clang` が Command Line Tools の導入を促すダイアログを出す。一般公開するとき（[非機能要件の未決事項](../spec/non-functional/open-questions.md) の、macOS の他のバージョンと他の OS への対応）に見直す。
