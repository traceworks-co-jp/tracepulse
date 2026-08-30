# 02. データモデル詳細設計

## 目的

SQLite 上の永続化対象を定義し、画面表示と監視ロジックで共通利用できる形にする。

## 主なテーブル

### devices

- 機器基本情報
- IP / name / community / status / last_seen_at

### interface_samples

- ifIndex 単位の収集値
- errors / discards / octets / bandwidth_utilization

### device_metrics

- CPU / memory 等の集約指標

### alert_events

- アラート履歴
- severity / type / details

## 代表的な Repository 関数

- `save_device_config`
- `find_device_by_ip`
- `list_devices`
- `update_device_status`
- `remove_device_by_ip`
- `save_sample`
- `save_device_metrics`
- `save_alert`
- `get_latest_interfaces`
- `get_interface_history`
- `get_device_metrics_history`
- `get_alert_history`
- `get_recent_alerts`
- `get_recent_interface_spikes`

## 画面との対応

- Dashboard: `list_devices`, `get_recent_alerts`
- Device Detail: `get_latest_interfaces`, `get_interface_history`, `get_alert_history`
- Discovery: `find_device_by_ip`, `save_device_config`
- Diagnostics: DB 参照は最小限、SNMP 実機取得が主体

## データ保持方針

- 最新状態と履歴を分離する
- 最新状態は summary 表示用
- 履歴は時系列グラフ用
- retention により定期削除可能にする

## エンティティ責務

### Device

- 登録対象の識別
- WebGUI の一覧表示対象

### InterfaceSample

- 帯域利用率の算出元
- スパイク検知の差分元

### DeviceMetrics

- CPU 等の集約値
- 健全度スコアに利用

### AlertEvent

- 監視結果のイベント化
- ダッシュボードの recent alerts に利用

## 設計上の注意

- デバイス削除時は関連履歴も削除する
- null を許容する項目は表示側で N/A に寄せる
- SNMP 取得失敗は空値として保存せず、保存失敗と区別する

## 図: データ関係

```text
Device 1 ──* InterfaceSample
Device 1 ──* DeviceMetrics
Device 1 ──* AlertEvent
```

## 図: 保存シーケンス

```text
Monitor -> Repository::save_sample
Monitor -> Repository::save_device_metrics
Monitor -> Repository::save_alert
UI -> Repository::get_latest_interfaces
UI -> Repository::get_recent_alerts
```
