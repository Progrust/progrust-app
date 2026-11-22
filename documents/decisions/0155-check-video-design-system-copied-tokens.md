---
type: Decision
title: "ADR 0155"
description: "デザインシステムの検査で、「Progrust Video」の `video-` で始まらないトークンと書体の族が、「Progrust」と同じ名前・同じ値であることを確かめる"
---

- 日付: 2026-11-22
- 関連仕様: [デザインシステムのファイルと変更の手順](../spec/screens/design-system-files.md)「検査」、[画面の見た目とテーマ](../spec/screens/appearance.md)「デザインシステム」
- 置き換え: なし

## 背景

書体・基本の色・章カラーなど、画面と共通の値は「Progrust」を正とし、動画のデザインシステム「Progrust Video」はその写しを持つ（[画面の見た目とテーマ](../spec/screens/appearance.md)「デザインシステム」、[ADR 0075](0075-video-design-system-progrust-video.md)）。「Progrust Video」の README は、書体、基本の色、章カラー、意味の色、コードの色を、アプリ側の同名トークンの写しとし、このシステムで新しく決めるのは `video-*` のトークンと部品の規則だけだとしている。

この写しは手で合わせていた（[ADR 0152](0152-keep-design-system-files-in-repository.md) の「背景」）。2 つのデザインシステムが同じリポジトリに入ったので、2 つの `tokens.json` を機械的に比べられるようになった。Issue #69 は、写しが合っているかを確かめる検査が無いことを問題として挙げた。

2026-11-22、ユーザーが呼んだ Issue の仕分け（`/triage-issues`）で、Claude は Issue #69 を直すと決めた。調べると、「Progrust Video」の `tokens.json` の、名前が `video-` で始まらないトークンは 81 個で、すべて「Progrust」に同じ名前があり、値も同じだった。説明にも、すべて写しと書かれていた。書体の族（`sans`・`mono`）の値も同じだった。

この判断は、アプリの操作感に関わらない技術的な判断で、`CLAUDE.md` はそのような判断を Claude に任せている。以下の決定と理由は、Claude の判断である。

## 決定

- `python3 tools/design-system.py check` に、「Progrust Video」の写しを確かめる規則（`copied-token`）を足す（[デザインシステムのファイルと変更の手順](../spec/screens/design-system-files.md)「検査」）。
  - 理由: 「Progrust」の共通の値を変えて、「Progrust Video」の写しを合わせ忘れると、画面と動画で同じはずの色や書体が食い違う。検査は、デザインシステムを変えたときと CI で実行するので、合わせ忘れが残らない。
- 写しとみなすのは、「Progrust Video」の `tokens.json` の、名前が `video-` で始まらないトークン（書体のスタイルを除く、すべての族）と、書体の族（`type.families`）とする。それぞれ、「Progrust」の `tokens.json` に同じ名前で、同じ値であることを確かめる。
  - 理由: 「Progrust Video」の README が、このシステムで新しく決めるのは `video-*` のトークンだけだとしている（「背景」を参照）。名前で決めると、写しにするトークンを足したときにも、検査の側を直さずに済む。
  - 「Progrust」の値を正とする。食い違ったときは、「Progrust Video」の値を合わせるように指摘する。
- 「Progrust」にあって「Progrust Video」に無いトークンは、指摘しない。
  - 理由: 動画は「Progrust」のトークンを全部は使わない。写しに持つのは、動画で使う値だけである。
- 書体のスタイル（`type.groups`）は比べない。
  - 理由: 動画の文字の大きさと太さは、動画のために決めている（`video-h1` など）。共通なのは書体の族だけである。
- 写しの元と写しの組（「Progrust Video」は「Progrust」の写しを持つこと、写しでないトークンの接頭辞が `video-` であること）は、`tools/design-system.py` の中に書く。
  - 理由: 組は、仕様が定めている 1 組だけである（[画面の見た目とテーマ](../spec/screens/appearance.md)「デザインシステム」）。索引（`design-system.json`）は、アーティファクトの種類「Design System」が形を決めていて、このプロジェクトのための項目を足すと、アーティファクトのページがどう扱うか分からない。

## 検討した他の案

- 案: トークンの説明（`usage`）に「写し」と書いてあるトークンだけを比べる
  - 採用しなかった理由: 写しのトークンを足したときに説明を書き忘れると、検査から漏れる。名前で決めるほうが漏れない。
- 案: 「Progrust」の共通の値を書き換えたら、「Progrust Video」の写しも自動で書き換える道具を作る
  - 採用しなかった理由: どの値を共通にするかは、デザインシステムを変える人が決めることで、書き換えの道具がそれを決めると、意図しない値まで動画に移る。食い違いを見つけて知らせれば、手で合わせる手間は小さい。

## 影響

- 「Progrust」の共通の値だけを変えて、「Progrust Video」の写しを合わせないと、デザインシステムの検査が指摘する。[デザインシステムのファイルと変更の手順](../spec/screens/design-system-files.md)「変更の手順」の、写しも合わせるという決まりを、検査で守れる。
- 「Progrust Video」だけで決める値に、`video-` で始まらない名前を付けると、検査が指摘する。そのような値は、`video-` で始まる名前にする。
- 「Progrust Video」に 2 つ目の写しの元ができたり、組が変わったりしたときは、`tools/design-system.py` を直す。
