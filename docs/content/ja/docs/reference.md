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

## 仕様メモ（.github/assets/spec.md）

`.github/assets/spec.md` で定義されている入出力仕様の要点は次のとおりです。

- `cols [OPTIONS] [FILE]`
	- 指定ディレクトリ（省略時はカレントディレクトリ）の内容を、プロファイルに基づいて色付け表示する。
- `cols --profile [application]`
	- プロファイルを開く。

本実装では `--profile` に加えて `-pf` も同等のショートカットとして利用できます。

## 色指定

`color_rules` では次の形式が使えます。

- `#RRGGBB`
- `rgb:R,G,B`
- `ansi:N`
- 色名: `red`, `green`, `blue`, `magenta`, `cyan`, `yellow`
- 既存の ANSI エスケープ列

## color_rules の記述方法

`color_rules` は 1 行 1 ルールで、次の形式で書きます。

```text
[dir:|file:]<regex> => <color-or-ANSI>
```

- `dir:` を付けるとディレクトリにだけ適用
- `file:` を付けるとファイルにだけ適用
- 接頭辞なしはファイルとディレクトリの両方に適用
- `#` から始まる行はコメント
- 空行は無視

例:

```text
dir:^docs$ => cyan
file:^.*\.rs$ => magenta
.*README.* => magenta
^\.gitmodules$ => yellow
.*cargo.* => #66CDAA
```

## 内部の考え方

- `parse_rule_line` がルール行を分解します。
- `color_spec_to_escape` が色指定を ANSI 表現へ変換します。
- `color_for_entry` がファイル種別とルールの両方を見て色を決めます。
- `format_long_entry_with_widths` が `-l` 表示を整形します。