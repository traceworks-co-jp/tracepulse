# TracePulse 実装計画

## 1. 目的とスコープ

TracePulse は、SNMP v2c を利用してネットワーク機器の状態を監視し、通信不良の予兆を短時間で可視化できるポータブルな Rust アプリケーションとして構築する。初期版では、L2/L3 スイッチおよびルータを対象とし、以下を主な価値とする。

- 単一の実行ファイルで動作
- 外部 DB / Web サーバー不要
- `config.toml` と `data.db` だけで持ち運び可能
- CLI/TUI と WebGUI を同一バイナリで切り替え可能
- 機器の自動スキャン、監視、アラート判定を一体化

対象スコープ（初期版）:
- SNMP v2c 対応
- 機器: L2/L3 スイッチ、ルータ
- 監視項目: インターフェース状態、エラー数、帯域利用率、標準 MIB で取得可能な CPU/メモリ使用率
- 監視機器数: 無料版 10 台、有料版 300 台
- 起動モード: `--cli` と `--web`

対象外（Phase 2）:
- 温度、電源、バッテリー等のベンダー固有環境情報
- SPAN / パケット深掘り解析
- Slack / Teams Webhook 通知

---

## 2. 確定済み要件

### 2.1 基本構成

- 1 実行ファイルに CLI/TUI と WebGUI を含める
- 実行ファイルと同じディレクトリに以下を生成
  - `config.toml`
  - `data.db`
- USB やフォルダ移動後でも動作継続できるようにする
- 外部 DB / Web サーバーの構築は不要

### 2.2 SNMP と検出

- SNMP バージョン: v2c
- 既定 community string: `public`
- community string はデフォルト値を持つが、個別デバイスごとに上書き可能
- 自動スキャン対象は単一 CIDR 指定（例: `192.168.1.0/24`）
- 機器の自動発見は同一サブネット範囲から対象 IP をスキャンし、SNMP 応答を持つ機器を候補として抽出
- 追加で手動登録や CSV/TOML 読み込みを将来的に拡張可能な構造にする

### 2.3 監視項目と収集間隔

- 既定ポーリング間隔: 30 秒
- 変更可能範囲: 5 秒〜10 分
- 主要監視項目:
  - インターフェースリンク状態 (`ifOperStatus` 相当)
  - 帯域利用率
  - エラーカウンタ
    - CRC
    - In/Out Drop
    - Frame
    - Discard
  - CPU 使用率
  - メモリ使用率

### 2.4 アラート判定

- 変化度（Spike）: 直近ポーリング差分に対して急増したエラー数を検知
- エラー率検知: 総トラフィックに対するエラー割合の閾値超過を検知
- 健全度スコア: 100 点満点の総合スコアを算出
  - 80 点未満: 黄（注意）
  - 60 点未満: 赤（要確認）
- 閾値はデフォルト値を内蔵し、`config.toml` でユーザー調整可能

### 2.5 障害判定と保持期間

- SNMP 応答が 3 回連続タイムアウトした場合、対象機器を `offline`（通信不能）へ遷移
- 履歴保持期間: デフォルト 7 日
- 古い履歴データは SQLite からバックグラウンドで削除

---

## 3. 全体アーキテクチャ

### 3.1 実行形式

```text
trace-pulse.exe / trace-pulse
├── CLI/TUI モード: --cli
├── WebGUI モード: --web
└── 共通ライブラリ層: config / db / snmp / poller / alert / score / ui
```

### 3.2 主要コンポーネント

1. App bootstrap
   - コマンドライン引数解析
   - 実行モード切替
   - 環境初期化（config, db, log）

2. Configuration layer
   - `config.toml` 読み込み
   - デフォルト値の定義
   - 監視設定、SNMP 設定、保持期間、閾値設定

3. Device registry
   - 監視対象機器の登録と管理
   - 手動登録 / 自動スキャン登録の共通モデル
   - ルール: SNMP community, IP, tags, polling interval

4. Discovery engine
   - CIDRスキャン
   - SNMP v2c の問い合わせ
   - 応答のある機器を候補として収集
   - ユーザーが選択した候補を登録する

5. Polling engine
   - 定期収集ループ
   - interface / system statistics の取得
   - 収集データを時系列 SQLite に保存

6. Alert engine
   - スパイク検知
   - エラー率検知
   - 健全度スコア計算
   - status の遷移（healthy / warning / critical / offline）

7. Storage layer
   - SQLite によるデータ保持
   - 最近の状態、過去履歴、機器情報、アラートイベントを保存
   - 7日を超えるデータのクリーンアップ

8. UI layer
   - CLI/TUI: 対話型画面で接続機器とアラート表示
   - WebGUI: 内蔵サーバー + ダッシュボード

---

## 4. 技術方針

### 4.1 言語と依存関係

- Rust を使用
- リッチな WebUI は最初から作らず、内蔵 Web サーバーと API を最小構成で実装
- 依存ライブラリは以下を中心に採用する予定
  - `tokio` で非同期処理
  - `rusqlite` で SQLite 管理
  - `serde` / `toml` で設定ファイル管理
  - `snmp` 系クライアントまたは独自 SNMP 実装を利用
  - `axum` または `warp` で WebGUI の API を実装
  - `ratatui` または `crossterm` で TUI を実装

### 4.2 ポータビリティ

- 実行ファイル同梱で外部サービスを不要にする
- `config.toml` と `data.db` は実行ファイルの相対パスに保存
- 監視データは SQLite に保持し、USB や移動先環境でも動作可能

### 4.3 スケーラビリティ

- 初期版は無料版 10 台を目安にする
- 監視ポーリングをスレッド/タスク分離する
- 機器ごとの収集ロジックを独立タスク化して、将来的に 300 台まで拡張可能な構造を目指す

---

## 5. データモデル

### 5.1 機器テーブル

- `id`
- `name`
- `ip`
- `community`
- `device_type`
- `status` (`online`, `offline`, `warning`, `critical`)
- `last_seen_at`
- `created_at`
- `updated_at`

### 5.2 インターフェーステーブル

- `id`
- `device_id`
- `if_index`
- `if_name`
- `if_alias`
- `link_status`
- `in_octets`
- `out_octets`
- `in_errors`
- `out_errors`
- `in_discards`
- `out_discards`
- `bandwidth_utilization`
- `sampled_at`

### 5.3 ヒストリーテーブル

- `id`
- `device_id`
- `interface_id`
- `sampled_at`
- `score`
- `status`
- `alert_type`
- `details_json`

### 5.4 設定ファイル

`config.toml` には以下を定義する。

```toml
[polling]
interval_seconds = 30

[snmp]
default_community = "public"

[alert]
error_rate_threshold = 0.05
spike_threshold = 10
health_warning_threshold = 80
health_critical_threshold = 60

[retention]
history_days = 7
```

---

## 6. 実装フェーズ

### Phase 1: 基盤構築

目的: アプリの土台を作り、設定と DB を持つ最小構成を用意する。

実装項目:
- プロジェクト初期構成
- CLI 引数の定義 (`--cli`, `--web`)
- `config.toml` の読み込み
- SQLite データベース生成
- データモデル定義
- ログ出力とエラー処理

成果物:
- アプリが起動し、設定ファイルと SQLite を生成できる

---

### Phase 2: SNMP 自動検出と機器登録

目的: 監視対象を簡単に登録できるようにする。

実装項目:
- CIDR 指定でのスキャン
- SNMP v2c 一問合せの処理
- 応答あり機器の一覧取得
- 手動登録と自動登録の統合
- community string の個別上書き対応

成果物:
- 監視対象機器を登録できる

---

### Phase 3: ポーリングと収集

目的: SNMP から計測値を定期取得し、SQLite に保存する。

実装項目:
- ポーリングタスクの定義
- `ifTable` / `ifXTable` 相当の MIB 取得
- インターフェース状態の収集
- エラー数、ドロップ数、帯域利用率の収集
- CPU / メモリ使用率の取得
- タイムアウト時の再試行と offline 判定

成果物:
- 収集データが SQLite に保存される

---

### Phase 4: 予兆検知と健全度スコア

目的: 実際の「通信不良の予兆」を判断できるロジックを実装する。

実装項目:
- スパイク検知
- エラー率検知
- 指標の重み付け
- 健全度スコア計算
- 状態遷移ロジック
- アラートイベントの保存

成果物:
- 機器ごとに warning / critical / offline の状態が判定される

---

### Phase 5: CLI/TUI 実装

目的: 現場で1〜数台をすぐに診断できる対話型画面を作る。

実装項目:
- TUI 画面レイアウト設計
- 機器一覧表示
- 健全度表示
- エラー急増表示
- リアルタイム更新
- 最小限の操作パターン（選択、再読込、終了）

成果物:
- ターミナル上で機器状態を確認できる

---

### Phase 6: WebGUI 実装

目的: 複数台の統合監視ダッシュボードを内蔵 Web で提供する。

実装項目:
- 内蔵 Web サーバー起動
- ダッシュボード API
- 機器一覧と健全度表示
- エラー急増機器のハイライト
- 画面更新の簡易 UI

成果物:
- `http://localhost:8080` でダッシュボードが閲覧できる

---

### Phase 7: クリーンアップと運用機能

目的: 実運用に耐える最小整備を行う。

実装項目:
- 7日を超える履歴の自動削除
- SQLite 整合性チェック
- 例外処理と構造化ログ
- リソース管理（タイムアウト、再接続）
- 監視規模上限の制限実装

成果物:
- 長時間運用しても破綻しない

---

## 7. 実装優先順位

### Must have（初期版）

1. CLI 実行基盤
2. 設定と SQLite 管理
3. SNMP v2c 収集
4. 機器検出と登録
5. エラー率 / スパイク / 健全度判定
6. TUI と WebGUI の最低限動作
7. 7日保持と自動削除

### Should have

- config.toml の詳細設定
- 個別 device 設定の上書き
- 監視レート制御とバックオフ
- 可視的な赤/黄ハイライト

### Nice to have（将来）

- CSV 一括登録
- 長期ログ保管の拡張
- Slack / Teams 通知
- SPAN ベースの深掘り解析

---

## 8. 実装時の設計ルール

- 監視ロジックと UI ロジックを分離する
- SNMP 取得部分を抽象化して、将来的に MIB 依存の差異を吸収可能にする
- SQLite は 1 つの永続層として扱い、WebGUI でも同一の DB を利用する
- CLI/TUI と WebGUI は共通の Device/Alert モデルを共有する
- 監視処理はバックグラウンドタスクで独立させ、UI は非同期に更新されるように設計する

---

## 9. 推奨ディレクトリ構成

```text
trace-pulse/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── app/
│   │   ├── mod.rs
│   │   ├── cli.rs
│   │   └── web.rs
│   ├── config/
│   │   ├── mod.rs
│   │   └── settings.rs
│   ├── db/
│   │   ├── mod.rs
│   │   ├── sqlite.rs
│   │   └── models.rs
│   ├── device/
│   │   ├── mod.rs
│   │   ├── registry.rs
│   │   └── discovery.rs
│   ├── snmp/
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   └── mib.rs
│   ├── monitor/
│   │   ├── mod.rs
│   │   ├── poller.rs
│   │   └── scoring.rs
│   ├── alert/
│   │   ├── mod.rs
│   │   └── rules.rs
│   └── ui/
│       ├── mod.rs
│       ├── tui.rs
│       └── dashboard.rs
├── config.toml
├── data.db
├── README.md
├── IMPLEMENTATION_PLAN.md
└── .gitignore
```

---

## 9.1 Rust ファイル単位の実装分解

以下は、実装時にそのまま着手できるように、モジュールごとのファイル責務を細分化した版である。

```text
src/
├── main.rs                                # アプリ起動エントリーポイント
├── lib.rs                                 # 共通の公開 API と module 経由の集約
├── app/
│   ├── mod.rs                             # app モジュールの定義
│   ├── cli.rs                             # --cli 実行時の初期化と主ループ
│   └── web.rs                             # --web 実行時の初期化と内蔵Web起動
├── config/
│   ├── mod.rs                             # config モジュール定義
│   ├── settings.rs                        # config.toml 読み込みとデフォルト値
│   └── defaults.rs                        # 閾値・ポーリング期・保持期間などのデフォルト
├── db/
│   ├── mod.rs                             # db モジュール定義
│   ├── sqlite.rs                          # SQLite 接続とマイグレーション
│   ├── models.rs                          # Device, Interface, Sample, Alert の構造体
│   └── repository.rs                      # CRUD とクエリ実装
├── device/
│   ├── mod.rs                             # entity と service の定義
│   ├── registry.rs                        # 監視対象リストの管理
│   ├── discovery.rs                       # CIDR スキャンと SNMP 応答機器抽出
│   ├── types.rs                           # DeviceConfig / DeviceStatus / DeviceSummary
│   └── validator.rs                       # device に対する基本検証
├── snmp/
│   ├── mod.rs                             # snmp モジュール定義
│   ├── client.rs                          # SNMP v2c 接続と PDU 呼び出し
│   ├── oid.rs                             # OID 定義と解析
│   ├── parser.rs                          # SNMP 応答のパース
│   └── mib.rs                             # IF-MIB / SYSTEM-MIB / HOST-RESOURCES-MIB の抽象
├── monitor/
│   ├── mod.rs                             # monitor モジュール定義
│   ├── poller.rs                          # ポーリングループとジョブ実行
│   ├── interface.rs                       # インターフェース情報収集
│   ├── system.rs                          # CPU / Memory 収集
│   ├── sampler.rs                         # 測定データの正規化と保存トリガー
│   └── metrics.rs                         # 指標計算 (utilization, error rate 等)
├── alert/
│   ├── mod.rs                             # alert モジュール定義
│   ├── rules.rs                           # スパイク / エラー率 / offline 判定ロジック
│   ├── scorer.rs                          # 健全度スコア計算
│   ├── event.rs                           # AlertEvent の生成と保存
│   └── status.rs                          # Healthy / Warning / Critical / Offline への遷移
├── ui/
│   ├── mod.rs                             # ui 共通定義
│   ├── tui.rs                             # TUI のレイアウトとイベント管理
│   ├── dashboard.rs                       # WebGUI 表示用のダッシュボード描画データ生成
│   ├── refresh.rs                         # リアルタイム更新と画面再描画
│   └── components/
│       ├── mod.rs
│       ├── device_list.rs                 # 機器一覧
│       ├── detail_panel.rs                # 詳細パネル
│       └── alert_feed.rs                  # アラート一覧
├── web/
│   ├── mod.rs                             # web モジュール定義
│   ├── server.rs                          # axum/warp による内蔵Webサーバー
│   ├── routes.rs                          # API ルート定義 (/api/devices, /api/alerts 等)
│   ├── handlers.rs                        # HTTP ハンドラー
│   └── response.rs                        # JSON レスポンス形式
├── cleanup/
│   ├── mod.rs                             # cleanup モジュール定義
│   ├── scheduler.rs                       # バックグラウンドでの削除ジョブ
│   └── retention.rs                       # 7日超過履歴削除ロジック
├── app_state.rs                           # 共有ステート: config, db, registry, poller 統合
├── error.rs                               # AppError / Result 型
├── logging.rs                             # ログ設定
├── utils.rs                               # IP 解析、タイム処理、数値正規化
└── tests/
    ├── mod.rs
    ├── snmp_mock.rs
    ├── alert_rules.rs
    ├── cleanup_tests.rs
    └── integration_cli.rs
```

### 1. main.rs / lib.rs

- `main.rs`
  - エントリーポイント
  - 引数解釈 (`--cli`, `--web`) を行う
  - `AppState` を生成して各モードへ委譲する
  - 実行時の基本例外処理を行う

- `lib.rs`
  - 各モジュールを `mod` で公開する
  - バイナリとライブラリ展開の両方を見据えた構造にする
  - テストや将来の API 参照を容易にする

### 2. config/ 配下

- `settings.rs`
  - `config.toml` を読み、`AppConfig` にマッピングする
  - 監視間隔、community、閾値、保持期間を保持する
  - デフォルト値とユーザー定義のマージを行う

- `defaults.rs`
  - 初期値の定義のみを集約
  - 例: `polling.interval_seconds = 30`, `health_warning = 80`, `health_critical = 60`

### 3. db/ 配下

- `sqlite.rs`
  - SQLite の接続、初期化、マイグレーションを担当
  - `data.db` の作成とパス解決を担当する

- `models.rs`
  - `Device`, `InterfaceSample`, `AlertEvent`, `HealthSnapshot` などの構造体定義
  - SQLite とアプリの間で使う DTO を定義する

- `repository.rs`
  - 用途別にデータアクセス追加
    - `save_device()`
    - `list_devices()`
    - `save_sample()`
    - `cleanup_old_history()`
    - `get_recent_alerts()`

### 4. device/ 配下

- `registry.rs`
  - 監視対象一覧の管理責務
  - 登録 / 更新 / 削除 / 取得 / 自動更新を担当
  - 重複 IP 抑止と一覧順の保持を担当する

- `discovery.rs`
  - `192.168.1.0/24` のような CIDR から機器候補を抽出
  - SNMP 応答がある IP のみを見つける
  - 候補リストを `DeviceConfig` に変換する

- `types.rs`
  - `DeviceConfig` と `DeviceStatus` の定義
  - `community` / `host` / `group` / `last_seen` を持つ

### 5. snmp/ 配下

- `client.rs`
  - SNMP v2c セッションの確立
  - `walk`, `get`, `getnext` に対応する最小ラッパー
  - タイムアウト処理を責務とする

- `oid.rs`
  - OID の定数管理
  - 例: `ifOperStatus`, `ifInErrors`, `ifOutErrors`, `ifHCInOctets`, `hrProcessorLoad`, `memTotalSwapTx` などの整理

- `parser.rs`
  - SNMP レスポンスから数値を抽出する
  - 文字列や unsigned int をアプリ側の型へ変換する

- `mib.rs`
  - Interface / System / Memory の取得ロジック抽象化
  - 取得項目ごとの差異を吸収するファイル

### 6. monitor/ 配下

- `poller.rs`
  - 一定間隔で各デバイスを回すポーリングループ
  - `tokio::spawn` で各デバイスを独立タスクにする設計
  - 失敗時の再試行、タイムアウト、status更新を担当する

- `interface.rs`
  - インターフェース固有の取得ロジック
  - `ifIndex`, `ifName`, `ifOperStatus`, `ifInErrors`, `ifOutErrors`, `ifInDiscards`, `ifOutDiscards`, `bandwidth` を収集する

- `system.rs`
  - CPU とメモリ使用率を収集
  - 標準 MIB で取得できる範囲に限定する

- `metrics.rs`
  - `error_rate`, `bandwidth_utilization`, `delta_error_count` を計算
  - alert 判定に使う単位の正規化を行う

### 7. alert/ 配下

- `rules.rs`
  - スパイク検知ロジック
  - ダelta 指標から増加率を計算
  - しきい値判定をまとめて持つ

- `scorer.rs`
  - 健全度スコア計算
  - 100点満点でパラメータを重み付けし、warning/critical を切り替える

- `status.rs`
  - `healthy -> warning -> critical -> offline` の遷移を管理
  - `3回連続タイムアウト` を `offline` 判定に変換する

- `event.rs`
  - AlertEvent を生成し、DB に保存する
  - Web や TUI が参照できるイベントを用意する

### 8. ui/ 配下

- `tui.rs`
  - TUI のレイアウトを定義
  - キー入力 (`q`, `r`, `s`) などのイベント処理を扱う
  - バッファ更新と画面描画の制御

- `dashboard.rs`
  - WebGUI 用の一覧データ整形
  - UI 表示に必要な一時データを生成する

- `components/device_list.rs`
  - 機器一覧を描画する
  - `Health`, `Status`, `Alert Count` を 1 行ビューに出す

- `components/detail_panel.rs`
  - 選択デバイスの CPU / Memory / Interfaces の詳細表示

### 9. web/ 配下

- `server.rs`
  - `axum` または `warp` でサーバー起動
  - `localhost:8080` で待機する

- `routes.rs`
  - `/api/devices`, `/api/alerts`, `/api/summary`, `/api/device/:id` を定義

- `handlers.rs`
  - 各 API の実装
  - DB から状態を取得して JSON に変換する

### 10. cleanup/ 配下

- `scheduler.rs`
  - バックグラウンドジョブとして定期実行する
  - 監視ループと並行して動作させる

- `retention.rs`
  - `history_days` を超えたレコードを削除
  - 7日超のサンプルの自動削除を担う

### 11. app_state.rs / error.rs / logging.rs / utils.rs

- `app_state.rs`
  - config / db / registry / poller / alert をまとめて持つ共有構造体
  - CLI/TUI と Web で共通に参照できるようにする

- `error.rs`
  - `AppError` enum を定義
  - `SNMPTimeout`, `ConfigError`, `DbFailure`, `ParseError` などを分ける

- `logging.rs`
  - `tracing` か `env_logger` を設定
  - `WARN/ERROR` は重要レベルに設定し、現場での運用に適した出力にする

- `utils.rs`
  - IP 文字列の解析
  - エラー計算・平均化
  - 時間処理、delta 計算の共通ロジック

### 12. tests/ 配下

- `snmp_mock.rs`
  - SNMP 応答のモックデータを定義する

- `alert_rules.rs`
  - spike / error rate / score のユニットテスト

- `cleanup_tests.rs`
  - 削除ロジックと retention の検証

- `integration_cli.rs`
  - CLI の最小動作確認

---

## 9.2 実装順序（ファイル単位）

1. `main.rs`
2. `app/cli.rs`
3. `app/web.rs`
4. `config/settings.rs`
5. `config/defaults.rs`
6. `db/sqlite.rs`
7. `db/models.rs`
8. `db/repository.rs`
9. `device/types.rs`
10. `device/registry.rs`
11. `device/discovery.rs`
12. `snmp/oid.rs`
13. `snmp/client.rs`
14. `snmp/parser.rs`
15. `snmp/mib.rs`
16. `monitor/interface.rs`
17. `monitor/system.rs`
18. `monitor/metrics.rs`
19. `monitor/poller.rs`
20. `alert/rules.rs`
21. `alert/scorer.rs`
22. `alert/status.rs`
23. `alert/event.rs`
24. `ui/tui.rs`
25. `ui/dashboard.rs`
26. `web/server.rs`
27. `web/routes.rs`
28. `web/handlers.rs`
29. `cleanup/scheduler.rs`
30. `cleanup/retention.rs`
31. `tests/*.rs`

この順序に従うことで、依存の少ない基盤から順番に構築でき、後の UI や Web 実装を安全に追加できる。

---

## 9.3 タスク単位の詳細設計（具体的な実装内容 / 入力 / 出力 / テスト観点）

以下は、各実装タスクを実際の開発単位として使えるように、具体的な実装内容・入力・出力・テスト観点まで落とし込んだ一覧である。

### 1. プロジェクト基盤の構築

- 実装内容
  - `Cargo.toml` に依存関係を追加する
  - `src/main.rs` に CLI 引数のエントリポイントを定義する
  - `src/lib.rs` と基本モジュールを組み立てる
  - `app/cli.rs`, `app/web.rs` の最小実装を作る
  - ロギングとエラー型の雛形を定義する
- 入力
  - 実行コマンド: `trace-pulse --cli`, `trace-pulse --web`
- 出力
  - アプリが起動してモードごとの初期化処理が完了する
  - ログが出力される
- テスト観点
  - `--cli` と `--web` の引数が正しく解釈できる
  - 未知の引数で `AppError` を返す
  - `main` から `run_app` が呼ばれることを確認できる

### 2. 設定管理の実装

- 実装内容
  - `config.toml` を読む `Settings` 構造体を定義する
  - デフォルト値を `defaults.rs` に持たせる
  - `AppConfig` を生成して監視設定を保持する
  - TOML の欠損値にデフォルト値を自動補完する
- 入力
  - `config.toml` のファイル内容
  - 例: polling.interval_seconds, snmp.default_community, alert.*
- 出力
  - `AppConfig` インスタンス
  - `Settings` の完全状態
- テスト観点
  - 設定ファイルが存在しない場合にデフォルト設定で動く
  - 不正な値（範囲外の interval, negative threshold）に対してバリデーションエラーを返す
  - TOML からの読み込みが正常にマッピングされる

### 3. SQLite 永続化の実装

- 実装内容
  - `data.db` を作成する
  - `schema.sql` 相当の `CREATE TABLE` を実行する
  - `Device`, `InterfaceSample`, `AlertEvent` の保存と取得を行う `repository` を実装する
- 入力
  - データ構造体
  - 一意キー: device_id, interface_id, sample timestamp
- 出力
  - SQLite 上のテーブルとレコード
  - `list_devices()`, `save_sample()`, `cleanup_old_history()` の結果
- テスト観点
  - DB が存在しない時に自動生成される
  - 保存した値を再読み込みして一致する
  - 古い履歴が所定期間で削除される

### 4. 機器登録と管理

- 実装内容
  - `DeviceConfig` を管理する `DeviceRegistry`
  - add/update/remove/list の API を持つ
  - `community` を個別に上書き可能にする
  - 1台あたりの状態と最終確認時刻を持つ
- 入力
  - IPアドレス, hostname, community string, device type
- 出力
  - `Vec<DeviceConfig>`
  - device status と metadata
- テスト観点
  - 重複デバイスの登録を抑止できる
  - 更新時に last_seen_at が更新される
  - 無効な IP または空の host で失敗する

### 5. SNMP 自動検出

- 実装内容
  - CIDR をパースする
  - 指定範囲のホストを順にスキャンする
  - SNMP v2c の `sysName`, `sysDescr` などを問い合わせる
  - 応答のある端末を candidate として収集する
- 入力
  - CIDR 文字列: `192.168.1.0/24`
  - SNMP community string
- 出力
  - `Vec<DiscoveredDevice>`
  - ユーザー選択対象のデバイス候補
- テスト観点
  - `192.168.1.0/24` の CIDR パースが成功する
  - SNMP応答がない IP は candidate から除外される
  - timeout 時にロギングされ、処理が継続する

### 6. ポーリング収集エンジン

- 実装内容
  - `poller` が設定されたデバイスを定期的に監視する
  - `interface.rs` と `system.rs` で収集処理を分割する
  - エラー数、帯域利用率、CPU/メモリを読んでサンプル保存する
- 入力
  - `DeviceConfig` リスト
  - `AppConfig.polling.interval_seconds`
- 出力
  - `InterfaceSample` / `SystemSample` のレコード
  - `last_seen_at` の更新
- テスト観点
  - 30秒タイマーが正しく回る
  - SNMP timeout 時にエラーとして記録される
  - 収集対象のデバイスが 0 台でも落ちない

### 7. 予兆判定アラート

- 実装内容
  - 前回サンプルとの差分を計算してスパイク検知する
  - 総トラフィックに対するエラー割合を計算する
  - 健全度スコアを算出する
  - `status` を `healthy`, `warning`, `critical`, `offline` に遷移させる
- 入力
  - 直近 sample と前回 sample
  - `AppConfig.alert.*` の閾値
- 出力
  - `AlertEvent` / `HealthSnapshot`
  - 監視対象の status
- テスト観点
  - エラー急増時に spike alert が生成される
  - エラー率が閾値超過時に alert が生成される
  - スコアが 80 未満で warning、60 未満で critical と判定される

### 8. CLI/TUI 診断画面

- 実装内容
  - `ratatui` / `crossterm` を用いて機器一覧を描画する
  - 現在の health と alert を表示する
  - 選択中デバイスの詳細画面を表示する
  - `q` で終了、`r` で再描画などの最小操作を持つ
- 入力
  - `DeviceRegistry` + `HealthSnapshot` + `AlertEvent`
- 出力
  - ターミナル画面の描画結果
- テスト観点
  - 特定デバイス選択時に詳細表示が切り替わる
  - 画面が再描画可能で落ちない
  - 監視データが空でもクラッシュしない

### 9. Webダッシュボード

- 実装内容
  - `axum` / `warp` で Web サーバーを起動
  - `/api/devices`, `/api/alerts`, `/api/summary` を設計
  - JSON として返す
  - 一覧画面には health と alert severity をハイライト表示する
- 入力
  - `DeviceRegistry`, `HealthSnapshot`, `AlertEvent`
- 出力
  - HTTP JSON API と HTML ダッシュボード
- テスト観点
  - `/api/summary` が正しい JSON を返す
  - 空データでも HTTP 200 を返す
  - 複数デバイスが一覧に出る

### 10. 履歴クリーンアップ

- 実装内容
  - `retention_days` を超える履歴を削除するジョブを定義する
  - SQLite の DELETE をバックグラウンドで定期実行する
  - アラート履歴が消される場合の整合性も確保する
- 入力
  - `history_days` 設定
  - `sampled_at` を持つレコード群
- 出力
  - old records の削除結果
  - `cleanup` のログ
- テスト観点
  - 7日超のデータのみ削除される
  - 7日前の境界値が正しく処理される
  - 削除中に DB エラーが発生した場合にログ出力される

### 11. 統合検証

- 実装内容
  - 最小の統合テストを作る
  - CLI, Web, alert, cleanup の一連の処理を疎通確認する
  - app 起動から収集判定までの基本走行テストを定義する
- 入力
  - モック SNMP 応答
  - 設定ファイル
  - 仮データベース
- 出力
  - テスト結果（pass/fail）
  - 実行ログ
- テスト観点
  - CLI から起動し、デバイス登録→収集→判定まで通る
  - Web API が 200 を返し、ダッシュボードが描画可能な JSON を返す
  - offline 判定が 3回タイムアウト後に発火する

---

## 9.4 テスト自動化の前提

AI による自動対応を前提にするなら、以下の観点でテストケースを生成するのが効率的である。

- 単体テスト観点
  - `ParseCIDR`, `NormalizeValue`, `CalculateErrorRate`, `ComputeHealthScore` などの純粋関数を精密に検証する
- 失敗系テスト
  - malformed configuration, timeout, invalid IP, empty DB, corrupt SQL row を確認する
- 境界値テスト
  - 0%, 50%, 80%, 100% 等の閾値付近を必ずテストする
- 並列/非同期テスト
  - 複数デバイスのポーリングが並列に動いても DB 破損しないことを確認する
- 回帰テスト
  - 既存の health score と alert 判定を壊さないように、固定サンプルを使って比較する

---

## 10. 成功基準

初期版が完了したとみなす条件は以下とする。

- `trace-pulse --cli` で 1〜数台の機器を登録・診断できる
- `trace-pulse --web` でローカルブラウザにダッシュボードを表示できる
- SNMP v2c で機器に接続し、インターフェース状態やエラー数を取得できる
- 収集した指標から スパイク / エラー率 / 健全度スコア を算出できる
- 3回連続タイムアウト時に offline 判定が動く
- 7日超過データが自動削除される
- 監視対象が 10 台までなら無料版制限が実装されている

---

## 11. 推奨リリース計画

### Milestone 1: MVP
- 基盤、設定、DB、SNMP 収集、アラート判定
- CLI/TUI でローカル診断可能

### Milestone 2: Dashboard
- WebGUI で複数機器を一覧表示
- 健全度とエラー急増の表示

### Milestone 3: Production polish
- 自動クリーンアップ
- タイムアウト/再接続処理
- アラート閾値の user config 対応
- 監視規模制限の適用

---

## 12. 結論

初期版の TracePulse は、SNMP v2c を利用した「個人でも運用できる軽量なネットワーク監視ツール」として実装するのが最適である。特に、L2/L3 スイッチとルータを対象とし、インターフェース/エラー/帯域/CPU/メモリの標準MIB監視を中心に据えることで、導入コストゼロで現場での予兆検知を行いやすいポータブル製品として成立する。

実装は、基盤 → SNMP収集 → 判定ロジック → CLI/TUI → WebGUI → 運用整備の順で進めるのが最も安全かつ短期間で成果を出せる。 
