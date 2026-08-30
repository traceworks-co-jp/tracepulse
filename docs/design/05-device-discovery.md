# 05. デバイス Discovery 詳細設計

## 目的

新規機器を発見し、重複なく登録する流れを定義する。

## フロー

1. CIDR 入力
2. scan 開始
3. progress 表示
4. 結果一覧表示
5. 既登録機器は disabled
6. 選択機器を登録
7. dashboard に反映

## 入力項目詳細

- CIDR: 必須
- community: デフォルト `public`
- max hosts: デフォルト 256

## 主要関数

- `page_discovery`
- `api_scan_start`
- `run_scan_job`
- `api_scan_status`
- `api_register`
- `parse_register_body`

## スキャン結果項目

- IP
- hostname
- status
- registration state
- duplicate 判定

## progress 表示

- running: scanned / total
- done: 結果一覧へ遷移
- error: エラー文を表示

## 手動登録

- scan 結果が無い場合でも使える
- 単一 IP で直接登録可能

## 図: Discovery シーケンス

```text
User -> page_discovery
User -> api_scan_start
api_scan_start -> run_scan_job
run_scan_job -> SnmpClient::probe_device
run_scan_job -> api_scan_status
User -> api_register
api_register -> Repository::save_device_config
```

## 図: 登録状態

```text
pending
  -> running
    -> done
    -> error
```

## 入力項目

- CIDR
- community
- max hosts

## 例外

- CIDR 不正
- 0 件
- SNMP 応答なし
- 既存重複

## manual add

- scan 近傍に導線を置く
- 単一 IP の直入力を許可する

## UI 要件

- scan 中に進捗を出す
- scan 停止/失敗時にユーザーが理由を理解できる
- duplicate はチェックボックス disabled で抑止する
