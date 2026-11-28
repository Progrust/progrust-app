---
type: Specification
title: "ch17 スマートポインタ"
description: "ch17 スマートポインタの章の説明と、Box、Option の中身を借りる・取り出す方法、Deref と参照外し型強制、Drop、Rc、Cell と RefCell、Rc と RefCell の組み合わせ、Weak と循環参照を扱うレッスン（ch17-01〜ch17-08 と章末復習）の ID・タイトル・主に扱う内容・主要素を定める。"
---

ヒープ上の値の所有と共有を扱うスマートポインタを学ぶ

| ID | タイトル | 主に扱う内容 | 主要素 |
| --- | --- | --- | --- |
| ch17-01 | Box | ヒープに値を置く（C の `malloc` に当たり、解放はスコープの終わりで自動で行われる。c02-05）、型の大きさがコンパイル時に決まっている必要があること、再帰的な型 | `smart_ptr.box` |
| ch17-02 | Option の中身を借りる・取り出す | `as_ref`・`as_deref`・`take`・`replace`、`std::mem::take`・`std::mem::replace` | `option.borrow_take` |
| ch17-03 | Deref と参照外し型強制 | `Deref` トレイト、`&String` が `&str` として渡せる理由 | `smart_ptr.deref` |
| ch17-04 | Drop | 破棄時の処理（C で、作る関数と対にして書いた壊す関数に当たる。c03-08）、`std::mem::drop` で早めに破棄する | `smart_ptr.drop` |
| ch17-05 | Rc | 参照カウントによる共有（「誰が解放するのか」を、参照の数で決める。c03-08）、`Rc::clone` | `smart_ptr.rc` |
| ch17-06 | Cell と RefCell | 内部可変性、実行時の借用チェックと panic | `smart_ptr.cell_refcell` |
| ch17-07 | Rc と RefCell の組み合わせ | 共有したまま変更する `Rc<RefCell<T>>` | `smart_ptr.rc_refcell` |
| ch17-08 | Weak と循環参照 | 循環参照によるメモリリーク（安全な Rust でもメモリリークは起こせる。c03-05）、`Weak`、親を指すポインタ | `smart_ptr.weak` |
| ch17-review | 章末復習 | | |
