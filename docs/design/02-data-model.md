# 02. データモデル詳細設計

## 目的

SQLite 上の永続化対象を定義し、画面表示と監視ロジックで共通利用できる形にする。

## 主なテーブル

### devices

- 機器基本情報
- IP / name / community / status / last_seen_at
- `community` は平文ではなく `tpenc:v1:` で始まるEnvelope Encryption形式のTEXT。
  Repositoryでのみ復号し、IPとカラムを認証コンテキストに紐付ける。
- `display_name` / `tags` はユーザー定義の表示用メタデータ。
- 表示名の空欄保存は前後空白を除去して `null` とし、保存成功時に詳細見出しへ即時反映する。解除後は `sys_name` / `hostname` / `ip` の既存フォールバックを用い、SNMP再取得の完了を待たない。

### credential_storage

- 単一行の鍵検証用Envelope（`key_check`）と、旧平文ページの後処理状態（`cleanup_pending`）。
- データ鍵はカラム値ごとのEnvelopeに内包され、このテーブルに平文鍵は保存しない。
- EnvelopeのBase64内容は、鍵ラップ用Nonce（12バイト）、ラップ済みデータ鍵
  （32バイト＋16バイト認証タグ）、値用Nonce（12バイト）、値の暗号文＋16バイト認証タグ。
- マスターキーはDB外の `secret.key`（Unix `0600`）または `TRACEPULSE_MASTER_KEY`。
- 鍵・Nonce・パスワードハッシュのSaltは、OS乱数で初期化した暗号学的擬似乱数生成器と
  各暗号ライブラリの生成APIで毎回生成する。OS乱数の取得失敗時は処理を中止する。
  `tpenc:v1:` の形式・暗号方式・認証コンテキストは維持し、既存の暗号化データを復号できる。
- DBと同じデータディレクトリの設定ファイルにある `snmp.default_community` も
  同一マスターキー・別認証コンテキストで暗号化する。

### interface_samples

- ifIndex 単位の収集値
- errors / discards / octets / bandwidth_utilization

### device_metrics

- CPU / memory 等の集約指標

### interface_alert_settings

- 既存テーブルを作り直さず追加するポート監視設定・最新状態。PKは `(device_id, if_index)`、機器削除でCASCADE。
- `link_down_alert_enabled` / `topology_link` / `active` はNOT NULL、既定0のboolean整数。
- `if_name` はNOT NULL、既定空文字。`if_admin_status` / `if_oper_status` / `observed_at` / `last_notified_at` はNULL許容。時刻はUnix秒。
- 最新SNMP状態、異常状態、24時間抑制時刻と `alert_events` 保存を同じトランザクションで更新し、古い観測は無視する。既存の `interface_samples` の構造は変えない。

### 通知・静観の永続状態

- `data.db` の `notification_intake` は元イベントIDで重複排除し、`notification_outbox` はチャネル別の成功・失敗・retryを記録する。成功済みチャネルは再送しない。配信scanは1秒、leaseは2分、既定3回、backoffは300ms〜5秒。
- `notification_port_journal` は検知状態・履歴と同じtransactionで追加する。起動時/5秒周期でOutboxへ取り込み、取り込み成功後だけ削除する。
- `maintenance_state` / `maintenance_periods` / `maintenance_changes` は現在の静観・期間・変更履歴。`basic_maintenance_reconciled` は解除後の新鮮な観測に基づくリスクまとめの重複を防ぐ。
- 完了/失敗確定Outboxとintakeは7日、未完了行は期限なし。解除まとめreceiptは1日。外部受付後・成功commit前の障害に限り外部で重複し得る。

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
- 暗号化対象は資格情報のみ。メトリクスのINSERTには暗号化・復号を介在させない。
- 初回の平文移行は全件トランザクション。移行後にVACUUMとWAL切詰めを行い、
  失敗時は起動エラーとする。中断した後処理は次回起動で再開する。
- 鍵不一致・欠落・破損Envelopeを平文へフォールバックせず、明示的なエラーとする。
- API・画面は資格情報をマスクまたは除外し、空入力更新では既存値を保持する。
- 将来のパスワード認証向けにArgon2id処理を用意するが、現行にはusers／credentialsテーブル、
  SNMP v3、WebGUIログインを新設しない。

鍵のバックアップ・復元と制約は[README](../../README.md#credential-protection)を参照。

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
