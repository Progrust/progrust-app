---
type: Decision
title: "ADR 0078"
description: "カリキュラムの唯一の定義を `curriculum.md` とし、`chapters.json` をそのデータ形式とする"
---

- 日付: 2026-11-20
- 関連仕様: [カリキュラムの目的とスコープ](../spec/curriculum/scope.md)、[カリキュラムの受け入れ条件](../spec/curriculum/acceptance.md)、[章とレッスンの並びの形式（chapters.json）](../spec/content-format/chapters-json.md)、[教材チェックツール](../spec/content-guidelines/check-tool.md)
- 置き換え: なし

## 背景

これまで `content/chapters.json` をカリキュラム（章立てと学習順）の唯一の定義としていた。カリキュラムを仕様書（`curriculum.md`）に取り込むにあたり、章とレッスンの ID・タイトル・順序を 2 か所に書くことになるため、どちらを正とするかを決める必要があった。

`chapters.json` は、章の ID・タイトル・説明とレッスンの ID・順序を持つが、レッスンのタイトル（`lesson.json` にある）と、各レッスンで主に扱う内容は持たない。

## 決定

カリキュラムの唯一の定義を `curriculum.md` とし、`chapters.json` はそれをアプリが読む形式で表したものとする。

理由: `chapters.json` はアプリが使うデータであり、仕様書とは役割が違う。仕様書には、各レッスンで主に扱う内容のように、データにない内容も書くことがあるため。

`chapters.json` と `lesson.json` の `title` が `curriculum.md` と一致することを、教材チェックツールで検証する（`curriculum.md`「受け入れ条件」、`content-guidelines.md`「教材チェックツール」）。

## 検討した他の案

- 案: `chapters.json` を正とし、`curriculum.md` には方針（並べ方の規則・扱わない内容・演習の作り方の注意）だけを書く
  - 採用しなかった理由: アプリが使うデータを仕様の定義にすることになる。データにない内容（各レッスンで主に扱う内容など）を仕様として書く場所がなくなる
- 案: `chapters.json` ができるまでの仮置きとし、できたら `curriculum.md` から一覧を消す
  - 採用しなかった理由: `chapters.json` はアプリが使うデータで、仕様書の代わりにはならない。一覧を消すと、データにない内容（各レッスンで主に扱う内容など）が仕様から失われる

## 影響

- 章とレッスンの ID・タイトル・順序を `curriculum.md` と `chapters.json`（レッスンのタイトルは `lesson.json`）の 2 か所に持つ。SSOT の原則（0007 など）からは外れるため、一致を教材チェックツールで検証して、ずれを防ぐ。
- カリキュラムを変えるときは、先に `curriculum.md` を変え、`chapters.json` と `lesson.json` を合わせる。
- 教材チェックツールが `curriculum.md` の表を読み取る必要がある。
