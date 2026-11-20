---
type: Specification
title: "ch17 スマートポインタ"
description: "ch17 スマートポインタの章の説明と、Box、Option の中身を借りる・取り出す方法、Deref と参照外し型強制、Drop、Rc、Cell と RefCell、Rc と RefCell の組み合わせ、Weak と循環参照を扱うレッスン（ch17-01〜ch17-08 と章末復習）の ID・タイトル・主に扱う内容を定める。"
---

ヒープ上の値の所有と共有を扱うスマートポインタを学ぶ

| ID | タイトル | 主に扱う内容 |
| --- | --- | --- |
| ch17-01 | Box | ヒープに値を置く、再帰的な型 |
| ch17-02 | Option の中身を借りる・取り出す | `as_ref`・`as_deref`・`take`・`replace`、`std::mem::take`・`std::mem::replace` |
| ch17-03 | Deref と参照外し型強制 | `Deref` トレイト、`&String` が `&str` として渡せる理由 |
| ch17-04 | Drop | 破棄時の処理、`std::mem::drop` で早めに破棄する |
| ch17-05 | Rc | 参照カウントによる共有、`Rc::clone` |
| ch17-06 | Cell と RefCell | 内部可変性、実行時の借用チェックと panic |
| ch17-07 | Rc と RefCell の組み合わせ | 共有したまま変更する `Rc<RefCell<T>>` |
| ch17-08 | Weak と循環参照 | 循環参照によるメモリリーク、`Weak`、親を指すポインタ |
| ch17-review | 章末復習 | |
