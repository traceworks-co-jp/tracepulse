# 01. システムアーキテクチャ詳細設計

## 目的

TracePulse 全体の構成、起動経路、責務分割、データの流れを定義する。

## 対象

- 単一バイナリ構成
- CLI/TUI と WebGUI の共存
- SQLite 永続化
- SNMP v2c ベース監視

## 構成方針

- UI 層と収集層を分離する
- 監視対象データは Repository 経由で永続化する
- WebGUI と CLI/TUI は同一の監視コアを参照する
- 変更が UI に閉じないよう、SNMP 収集は Web/CLI 共通化する

## レイヤ構成

### 1. 起動層

- `main.rs`
- コマンドライン引数の解釈
- CLI/TUI または WebGUI の起動分岐

### 2. アプリケーション層

- `app`
- 設定読み込み
- Repository 初期化
- 監視ループ開始
- WebServer 起動

### 3. ドメイン層

- `device`
- `monitor`
- `alert`

### 4. インフラ層

- `snmp`
- `db`
- `web`

## 主要エントリポイント

- `src/main.rs`
  - CLI 引数解析
  - `App::new(...)` 呼び出し
- `src/app/mod.rs`
  - `App::run_cli()`
  - `App::run_web()`
- `src/web/server.rs`
  - `WebServer::new(...)`
  - `WebServer::start()`

## 主要依存関係

- `app` → `config`, `db`, `snmp`, `web`
- `web` → `db`, `snmp`, `monitor`
- `monitor` → `snmp`, `db`
- `snmp` → `device`, `config`, `error`

## 起動シーケンス

1. `App::new` で設定と DB を初期化
2. `run_cli` または `run_web` を選択
3. WebGUI の場合は `WebServer::start`
4. WebServer が polling loop を別スレッドで開始
5. HTTP ループで画面/API を提供

## 失敗時の扱い

- 設定読込失敗: デフォルト値で継続
- DB 初期化失敗: 起動失敗
- SNMP 失敗: 対象機器のみ offline / warning へ寄せる
- Web bind 失敗: 起動失敗

## データフロー

1. 起動時に `config.toml` を読み込む
2. SQLite を初期化する
3. 機器登録済み一覧を取得する
4. SNMP で対象機器をポーリングする
5. 取得値を metrics / alerts / interface_samples に保存する
6. WebGUI は保存済みデータを表示する

## 責務分割

- `snmp`: 取得処理のみ
- `db`: 永続化のみ
- `monitor`: 収集結果の解釈と保存
- `web`: 表示と操作 API
- `app`: 起動制御

## 非機能要件

- ポータブル実行可能
- 依存サービス不要
- 異常時に落ちにくい単一プロセス
- 監視コアは Web 停止時も CLI 側で再利用可能

## 図: コンポーネント関係

```text
main
 └─ App
     ├─ config
     ├─ Repository
     ├─ SnmpClient
     ├─ WebServer
     └─ Monitor / Alert
```

## 図: 起動シーケンス

```text
User -> main -> App::new
App::new -> config load
App::new -> sqlite init
App::new -> Repository
main -> App::run_cli / run_web
run_web -> WebServer::start
WebServer::start -> polling loop thread
```
