---
title: "ColorfuLS"
description: "色付きのファイル一覧を表示する Rust 製 CLI"
params:
  body_class: td-navbar-links-all-active
  ui:
    navbar_theme: dark
---

# ColorfuLS

ColorfuLS は、`cols` コマンドでカレントディレクトリを色付きで一覧表示する Rust 製 CLI です。
README.md の内容に加えて、プロファイル、チュートリアル、実行例を Markdown で整理しています。

## すぐに読む

- [ドキュメント](/docs/)
- [チュートリアル](/docs/tutorial/)
- [実行例](/docs/examples/)

## このサイトで分かること

- `-a/--all` と `-l/--long` の使い方
- `--profile` と `-pf` でプロファイルを開く方法
- `color_rules` による色指定の書き方
- ファイル種別ごとの色分けの動作
- 実行例とプロファイルのサンプル

## 要点

1. `cols` はカレントディレクトリを一覧表示します。
2. `color_rules` により、README や Rust ソース、Markdown を個別に色付けできます。
3. Unix と Windows で long 表示の扱いを分けています。
