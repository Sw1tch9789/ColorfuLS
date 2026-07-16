---
title: "リファレンス"
linkTitle: "Reference"
description: "オプションと内部仕様の要点"
weight: 30
type: docs
---

## オプション

| オプション | 説明 |
| --- | --- |
| `-a`, `--all` | 隠しファイルを含むすべてのエントリを表示します。 |
| `-l`, `--long` | 詳細情報を表示します。 |
| `--profile`, `-pf` | プロファイルファイルを開きます。 |

## 色指定

`color_rules` では次の形式が使えます。

- `#RRGGBB`
- `rgb:R,G,B`
- `ansi:N`
- 色名: `red`, `green`, `blue`, `magenta`, `cyan`, `yellow`
- 既存の ANSI エスケープ列

## 内部の考え方

- `parse_rule_line` がルール行を分解します。
- `color_spec_to_escape` が色指定を ANSI 表現へ変換します。
- `color_for_entry` がファイル種別とルールの両方を見て色を決めます。
- `format_long_entry_with_widths` が `-l` 表示を整形します。