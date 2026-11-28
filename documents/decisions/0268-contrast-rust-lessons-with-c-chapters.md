---
type: Decision
title: "ADR 0268"
description: "Rust のレッスンのうち C の章で体験した内容に対応するものを、C と対比して説明する内容に書き直し、ch04-01 を C の章の振り返りと Rust の仕組みの予告のレッスンにする"
---

- 日付: 2026-11-28
- 関連仕様: [カリキュラムの方針](../spec/curriculum/policy.md)「C の章と Rust の章」、[ch04 所有権](../spec/curriculum/chapters/ch04-ownership.md)、[ch05 参照と借用](../spec/curriculum/chapters/ch05-references-and-borrowing.md)、[ch01 Rust の基本](../spec/curriculum/chapters/ch01-rust-basics.md)、[ch02 基本の型](../spec/curriculum/chapters/ch02-basic-types.md)、[ch07 列挙型とパターンマッチ](../spec/curriculum/chapters/ch07-enums-and-pattern-matching.md)、[ch08 コレクション](../spec/curriculum/chapters/ch08-collections.md)、[ch09 エラー処理](../spec/curriculum/chapters/ch09-error-handling.md)、[ch17 スマートポインタ](../spec/curriculum/chapters/ch17-smart-pointers.md)、[ch23 マクロと unsafe](../spec/curriculum/chapters/ch23-macros-and-unsafe.md)、[動画の作成ルール](../spec/content-guidelines/video.md)「C との対比」、[未登場の要素を使わない](../spec/content-guidelines/no-unseen-elements.md)
- 置き換え: なし

## 背景

カリキュラムの最初に C の章を置いた（[ADR 0262](0262-teach-c-chapters-before-rust.md)）。ユーザーが最初に示した案は、C で身についたメモリ管理の複雑さを、Rust がどう克服するかを見せることを求めている（[ADR 0262](0262-teach-c-chapters-before-rust.md) の「背景」）。

Rust の章には、メモリの仕組みを説明する動画のみのレッスン ch04-01 と、周辺知識を「主に扱う内容」に含めたレッスンがある（[ADR 0118](0118-teach-background-knowledge-hidden-by-gc-languages.md)）。これらは、学習者がメモリの仕組みを知らない前提で書かれていた。カリキュラムを組み立てた後にレッスンを変えたときは、理由を ADR に記録する（[ADR 0131](0131-record-reasons-for-later-lesson-changes.md)）。

## 決定

以下は、どれも Claude が決めた。

- Rust のレッスンのうち、C の章で体験した内容に対応するものは、C ではどうなったかと、Rust がそれをどう扱うかを対比して説明する。対比するレッスンは、章とレッスンの一覧の「主に扱う内容」に、対応する C のレッスンの ID を添えて示す（[カリキュラムの方針](../spec/curriculum/policy.md)「C の章と Rust の章」、[動画の作成ルール](../spec/content-guidelines/video.md)「C との対比」）。
  - 理由: 上の「背景」のユーザーの案のとおり、C で体験したことを Rust の説明に結びつける。どのレッスンで何と対比するかを一覧に書いておけば、教材を作るときに漏れない。
- 「主に扱う内容」を、次のように変えた。レッスンの ID・タイトル・主要素は変えていない。

  | レッスン | 足した対比（括弧内は対応する C のレッスン） |
  | --- | --- |
  | ch01-01 | コンパイルしてから実行するのは C と同じであること（c01-02） |
  | ch02-01 | 整数のあふれが、C では未定義動作、Rust では panic になること（c01-04・c03-01） |
  | ch02-03 | C の暗黙の型変換が、Rust には無いこと（c01-04） |
  | ch02-06 | 配列が要素数を持つこと（c01-08）、範囲外の添字が panic で止まること（c03-02） |
  | ch04-02 | `free` に当たる処理をコンパイラが入れ、書き忘れが起きないこと（c03-05） |
  | ch04-03 | `String` が、C で作った伸びる配列と同じ形であること（c02-07） |
  | ch04-04 | ムーブが C の浅いコピーと同じコピーで、元の変数を使えなくして二重解放を防ぐこと（c03-07・c03-04） |
  | ch04-05 | `clone` が、C で自分で書いた深いコピーに当たること（c03-07） |
  | ch04-06 | 「誰が解放するのか」を、型とコンパイラが決めること（c03-08） |
  | ch05-01 | 参照の実体が C のポインタと同じアドレスであることと、ポインタとの違い（c02-01） |
  | ch05-02 | C ではポインタを渡して書き換えたこと（c02-02） |
  | ch05-03 | 借用の規則が、無効になるポインタの誤りを防ぐこと（c03-06） |
  | ch05-04 | C では実行するまで気づけなかった誤りが、コンパイルエラーになること（c02-04・c03-03） |
  | ch05-05 | スライスが長さを持つこと（c02-03） |
  | ch07-03 | `Option` と C の `NULL` の違い（c02-01） |
  | ch08-01 | `Vec` の再確保が、`realloc` で無効になるポインタと同じであること（c02-07・c03-06） |
  | ch09-01 | panic が未定義動作と違い、必ず止まること（c03-01） |
  | ch17-01 | `Box` が C の `malloc` に当たること（c02-05） |
  | ch17-04 | `Drop` が、C の壊す関数に当たること（c03-08） |
  | ch17-05 | `Rc` が、「誰が解放するのか」を参照の数で決めること（c03-08） |
  | ch17-08 | 安全な Rust でもメモリリークは起こせること（c03-05） |
  | ch23-03 | `unsafe` の中では未定義動作が起きうること（c03-01）、生ポインタが C のポインタに当たること（c02-01） |

- ch04-01「メモリの仕組み」は、動画のみのレッスンのまま残し、内容を、C の章で学んだことの振り返りと、Rust がそれらの誤りをコンパイル時に防ぐこと、その仕組みをこの先で学ぶことの予告に変える（[ch04 所有権](../spec/curriculum/chapters/ch04-ownership.md)）。ID とタイトルは変えない。[ADR 0118](0118-teach-background-knowledge-hidden-by-gc-languages.md) の「決定」の表の、ch04-01 で「スタックとヒープ、スタックフレーム、ガベージコレクションがないこと、二重解放・解放後の使用」を扱うとした行を、この点で変える。
  - 理由: スタックとヒープ、二重解放、解放後の使用は、C の章で演習つきで学ぶようになった。一方で、C の章を終えてから所有権の章までに Rust の基本の 3 章があり、所有権に入る前に、C で体験したことを思い出させる場所が要る。ID とタイトルを変えなければ、後ろのレッスンの ID と、作成済みの教材の `lesson.json` の照合が変わらない。
- ch05-04 が前提とするレッスンを、ch04-01 から c02-04 に変える。[ADR 0118](0118-teach-background-knowledge-hidden-by-gc-languages.md) の「決定」の表の、ch05-04 の行に当たる内容（関数から戻るとローカル変数がスタックフレームごと消えること）は、ch05-04 に残す。
  - 理由: スタックフレームが戻ると消えることは、c02-04 で学ぶようになった。
- Rust が防ぐ範囲を、実際より広く言わない。コンパイル時に防ぐもの、実行時に止めるもの、防がないものを区別して説明する（[動画の作成ルール](../spec/content-guidelines/video.md)「C との対比」）。
  - 理由: 配列の範囲外アクセスは実行時の panic で、メモリリークは安全な Rust でも起こせる。「Rust はすべて防ぐ」と教えると誤りになる。
- Rust のレッスンの動画と解説には、対比のために、C の章で扱った範囲の C のコードを登場させてよい。Rust の演習では、学習者に C のコードを書かせない（[未登場の要素を使わない](../spec/content-guidelines/no-unseen-elements.md)）。

## 検討した他の案

- 案: ch04-01 を消す
  - 採用しなかった理由: ch04 の後ろのレッスンの ID がずれる。作成済みの教材と、これまでの ADR・仕様の本文の ID が合わなくなる。所有権の前の振り返りも無くなる
- 案: 対比するかどうかを、レッスンを作るときに決め、一覧には書かない
  - 採用しなかった理由: 教材は AI も作る。一覧に書いていないと、対比が抜けたり、対応しない C のレッスンと対比したりする

## 影響

- [ADR 0118](0118-teach-background-knowledge-hidden-by-gc-languages.md) の表のうち、ch04-01 の行と、ch05-04 が ch04-01 を既習とする点が、この ADR で変わった。
- 作成済みの ch04-01 と ch04-04 の教材は、「主に扱う内容」が変わったので、作り直しが要る。
- データ競合（ch20-01）は、C の章で扱っていない（[ADR 0263](0263-c-chapters-scope-three-chapters.md)）ので、対比を足していない。
