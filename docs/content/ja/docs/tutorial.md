---
title: "チュートリアル"
linkTitle: "Tutorial"
description: "ColorfuLS を最初から試すための手順"
weight: 10
type: docs
---

## 1. ビルド

まずは Rust の開発環境を用意し、以下を実行します。

```bash
cargo build --release
```

実行ファイルは `target/release/cols` に生成されます。

## 2. そのまま表示する

```bash
./target/release/cols
```

現在のディレクトリのファイルを、色付きで一覧表示します。

## 3. 隠しファイルも表示する

```bash
./target/release/cols -a
```

`.` で始まるファイルやディレクトリも表示対象に含めます。

## 4. 詳細表示を使う

```bash
./target/release/cols -l
```

詳細表示では、権限、リンク数、所有者、グループ、サイズ、更新時刻が出ます。

## 5. color_rules を作る

プロジェクト直下に `color_rules` を置くか、`COLOR_RULES` 環境変数で場所を指定します。

```text
dir:.*README.* => magenta
file:^.*\.rs$ => \x1b[32m
\.md$ => \x1b[33m
```

この例では、README をマゼンタ、Rust ファイルを緑、Markdown を黄色にします。

## 6. プロファイルを開く

```bash
./target/release/cols --profile
./target/release/cols -pf
```

どちらもプロファイルファイルを開くための入力として扱われます。