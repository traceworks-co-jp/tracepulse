# 08. 運用・配布詳細設計

## 目的

現場配布、起動、データ保存、運用時の注意点を整理する。

## 配布形態

- 単一 exe
- 同一フォルダ配布
- `config.toml` と `data.db` をローカル保存

## 実運用ファイル

- `trace-pulse.exe`
- `config.toml`
- `data.db`
- 必要に応じてログ出力先フォルダ

## 配布時の確認項目

- 実行権限
- 書き込み権限
- 161/UDP 到達性
- community 設定

## 起動後確認

- WebGUI 起動確認
- Dashboard の表示確認
- Discovery の scan 確認
- Diagnostics の SNMP 応答確認

## 図: 配布イメージ

```text
trace-pulse.exe
config.toml
data.db
logs/
```

## 図: 運用シーケンス

```text
Operator -> start exe
exe -> read config.toml
exe -> open/create data.db
exe -> start CLI or WebGUI
Operator -> monitor dashboard / run diagnostics
```

## 起動方法

- `--cli`
- `--web`

## ローカル保存

- 実行ディレクトリに設定と DB を作成
- 書き込み権限が必要

## 運用注意

- SNMP community を個別管理する
- retention を環境ごとに見直す
- しきい値は初期値のまま固定しない

## 障害時

- WebGUI が落ちても CLI/TUI で確認可能
- SQLite が壊れた場合は再初期化手順を用意する
