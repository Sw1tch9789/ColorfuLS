---
title: "実行例"
linkTitle: "Examples"
description: "複数の実行例と色ルールのサンプル"
weight: 20
type: docs
---

## デフォルト表示

```bash
$ ./target/release/cols
README.md
Cargo.toml
src
```

## 隠しファイルを含める

```bash
$ ./target/release/cols -a
.gitignore
.cargo
Cargo.toml
README.md
src
```

## 詳細表示

```bash
$ ./target/release/cols -l
-rw-r--r--  1 user group  123 Apr 12 14:15 README.md
drwxr-xr-x  3 user group 4096 Apr 12 13:50 src
```

## color_rules の例

```text
dir:.*README.* => magenta
file:^.*\.rs$ => \x1b[32m
\.txt$ => cyan
\.md$ => \x1b[33m
```

README を目立たせたいときは `dir:` 付きのルールを使い、ソースや文書の色を分けると見やすくなります。