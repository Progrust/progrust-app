---
type: Specification
title: "演習問題生成の目的とスコープ"
description: "演習問題生成が扱う範囲（生成するのは Rust の問題だけであること）と、生成した問題を Rust のレッスンの演習と同じ形式で保存し同じ仕組みで実行・判定・記録することを定める。"
---

学習者の設定と分析ファイルをもとに、AI が演習問題を生成する機能を定める。
生成する問題は、すべて Rust の問題（Rust の演習）とする。C の問題は生成しない。
生成した問題は Rust のレッスンの演習と同じ形式（[レッスン定義の形式（lesson.json）](../content-format/lesson-json.md)・[演習のファイル形式](../content-format/exercise.md)）で保存し、同じ仕組みで実行・判定・記録する。
