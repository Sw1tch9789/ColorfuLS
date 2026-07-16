# Badge
[![Coverage Status](https://coveralls.io/repos/github/Sw1tch9789/ColorfuLS/badge.svg)](https://coveralls.io/github/Sw1tch9789/ColorfuLS)

# ColorfuLS
ディレクトリやファイル、拡張子ごとに色を設定して、リスト表示を行う。

## Description
色付け設定は、プロファイルを参照して行う。

## Usage
cols [OPTIONS] [FILE]

## Options
```
-l ファイルまたはディレクトリの詳細な情報を表示します。
-a 隠しファイルを含むすべてのファイルやディレクトリを表示します。
--profile / -pf プロファイルを開きます。
```

## Docker 配布

このプロジェクトでは、CLI イメージとドキュメントイメージを Docker Hub に配布できます。

- CLI image: `DOCKERHUB_NAMESPACE/colorfuls`
- Docs image: `DOCKERHUB_NAMESPACE/colorfuls-docs`

### 1. ログイン

```bash
just docker-login
```

### 2. ローカルビルド（push しない）

```bash
just container-local
```

### 3. Docker Hub へ push（multi-arch）

```bash
DOCKERHUB_NAMESPACE=<your-dockerhub-user> just publish
```

`just publish` は次を実行します。

1. `docs` を Hugo で生成
2. `Containerfile` で CLI イメージをビルド＆push
3. `Containerfile.docs` で docs イメージをビルド＆push

### デフォルト設定の同梱

CLI イメージには `color_rules` を同梱しており、デフォルトで `COLOR_RULES=/etc/colorfuls/color_rules` を参照します。
