# 06. 監視・アラート詳細設計

## 目的

収集値からアラートを生成し、履歴として保持する。

## 共通通知基盤

`notifier` / `notifications` の送信・設定実装はCommunityを正本とし、Enterpriseも同じ実装を利用する。`/settings` と `GET/POST /api/notifications`、`POST /api/notifications/test` でSlack / Teams / Webhook / メール / Syslog等を設定する。全体/機器別静観は `GET/POST /api/maintenance`。静観中も収集と履歴保存を継続し、外部通知だけを止め、解除後の新鮮な観測で継続リスクをまとめる。

死活・Online復旧、CPU/メモリ閾値および重要ポートは共通機能。Enterpriseの予兆監視・監査・週次/月次レポートのproducerは共通コアへ移さない。

WebおよびCLI/TUI起動時に共通runtimeを接続する。送信前・再試行前の静観判定はfail-closedで、DB障害時に無条件送出しない。重要ポートは検知transaction内のjournalからも再取り込みし、broadcast消失を回復する。通知の完了管理はチャネルごとに永続化し、配信失敗でworker全体を終了しない。共通通知と重要ポートの変更APIはHost/Originを検証する。

CPU/メモリの解除まとめは現在の設定閾値を使用し、解除前と解除後の新鮮な各サンプルで継続超過を確認する。値が欠損した項目は評価済みにせず後続サンプルを待ち、新たに発生した超過を「継続」と誤表示しない。まとめの重複防止記録とOutbox追加は同一トランザクションで保存する。

## 重要ポートのリンクダウン

- 設定は `(device_id, if_index)` 単位で永続化。手動指定の既定値はOFF。
- `ifAdminStatus=1` かつ `ifOperStatus=2` の場合だけCriticalの `PORT_LINK_DOWN` を生成する。shutdown、未知値、取得失敗をdownと推測しない。
- 継続downは24時間重複抑制。`ifOperStatus=1` でInfoの `RESOLVED_PORT_LINK_DOWN`（`[RESOLVED]`）を発行し、抑制をクリアする。
- `PUT /api/devices/{id}/interfaces/{if_index}/alert` のbodyは `{"link_down_alert_enabled":boolean}`。詳細画面のトグルと赤色表示は日英対応する。
- Enterpriseは保存済みLLDP/CDP接続の両端を自動指定できる。手動OR自動で有効、幹線同期は手動ピンを消さない。設定解除は物理リンク復旧の通知を生成しない。

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
