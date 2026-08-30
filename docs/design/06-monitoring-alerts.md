# 06. 監視・アラート詳細設計

## 目的

収集値からアラートを生成し、履歴として保持する。

## 監視対象

- interface errors
- interface discards
- bandwidth utilization
- CPU usage

## 主要関数

- `run_polling_loop`
- `check_interface_spikes`
- `save_device_metrics`
- `save_alert`
- `get_recent_alerts`
- `get_recent_interface_spikes`

## ポーリングシーケンス

1. 登録済み機器一覧を取得
2. SNMP で interface / CPU を収集
3. metrics と sample を保存
4. spike を判定
5. alert_events に保存
6. dashboard / detail に反映

## アラート種別

- interface_error_spike
- warning
- critical
- offline

## 表示箇所

- Dashboard summary
- Dashboard recent alerts
- Device Detail alert history

## 図: ポーリングシーケンス

```text
run_polling_loop
 -> list_devices
 -> query_device / query_interface
 -> save_sample / save_device_metrics
 -> check_interface_spikes
 -> save_alert
 -> update_device_status
```

## 図: アラート発生フロー

```text
sample change
 -> threshold check
 -> alert_event create
 -> repository save
 -> dashboard recent alerts
 -> device detail alert history
```

## 判定

### Spike

- 前回との差分が閾値を超える場合

### Health

- CPU / bandwidth / error rate などをスコア化

## 保存

- alert_events に保存
- dashboard recent alerts に表示
- device detail の alert history に表示

## 表示

- warning / critical を色分けする
- 同一機器の最新イベントを上位に出す

## 注意

- 判定ロジックと表示ロジックを分離する
- 保存前に重複イベントを増やしすぎない
