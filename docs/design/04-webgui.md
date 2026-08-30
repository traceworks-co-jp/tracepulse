# 04. WebGUI 詳細設計

## 目的

WebGUI の画面構成、役割分担、主要操作を定義する。

## 画面一覧

### Dashboard

- 機器一覧
- 健全度
- recent alerts
- 登録解除

### Device Detail

- interface 一覧
- 帯域グラフ
- error/discard グラフ
- CPU グラフ
- hardware status

### Discovery

- CIDR 入力
- scan 実行
- 登録
- 手動追加導線

### Diagnostics

- SNMP 診断
- CPU candidates
- Hardware candidates
- Hardware sensors

### Settings

- polling interval
- community
- retention
- thresholds

## 画面項目詳細

### Dashboard

- Summary cards
- device table
- recent alerts table
- unregister button

### Device Detail

- title / status badge
- interface table
- bandwidth chart
- error chart
- CPU chart
- Hardware Status card
- alert history

### Discovery

- CIDR input
- community input
- max hosts input
- scan progress
- result table
- manual add link

### Diagnostics

- target IP / community
- run diagnostics button
- progress indicator
- CPU candidates
- Hardware candidates
- Hardware sensors

## 主要関数

- `page_dashboard`
- `page_device_detail`
- `page_discovery`
- `page_diagnostics`
- `api_summary`
- `api_devices`
- `api_device_detail`
- `api_scan_start`
- `api_scan_status`
- `api_register`
- `api_delete_device`

## UI の役割分担

- Dashboard: 一覧把握
- Detail: 日常監視
- Discovery: 追加作業
- Diagnostics: 根拠確認

## 図: 画面遷移

```text
Dashboard
 ├─ Device Detail
 ├─ Discovery
 ├─ Diagnostics
 └─ Settings
Discovery -> Register -> Dashboard
Diagnostics -> (no navigation, same page refresh)
Device Detail -> Dashboard
```

## 図: Web ルーティング

```text
GET  /                  -> page_dashboard
GET  /device/<ip>       -> page_device_detail
GET  /discovery         -> page_discovery
GET  /diagnostics       -> page_diagnostics
GET  /settings          -> page_settings
GET  /api/summary       -> api_summary
GET  /api/devices       -> api_devices
GET  /api/device/<ip>   -> api_device_detail
POST /api/discovery/...  -> api_scan_start / api_register
DELETE /api/device/<ip> -> api_delete_device
```

## UI 方針

- 情報量が多い画面はセクション分割する
- 監視に不要な詳細は Diagnostics に寄せる
- Detail は日常確認用に簡潔に保つ

## 表示責務

- Detail: 状態確認に必要な最小情報
- Diagnostics: OID / 候補 / 収集根拠
