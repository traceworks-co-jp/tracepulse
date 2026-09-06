# TracePulse

TracePulse is a portable, single-binary network device monitoring application written in Rust. It provides both an interactive terminal dashboard (TUI) and an embedded WebGUI for SNMP-based network health monitoring, anomaly detection, and topology discovery.

TracePulse は、SNMP を用いてネットワーク機器の異常兆候を検知し、対話型 TUI (ターミナルUI) と WebGUI の両方を同一バイナリで提供する Rust 製のポータブル監視アプリケーションです。

---

## 特徴 / Key Features

- **ポータブル＆単一バイナリ (Single Binary & Portable)**
  - 内蔵 SQLite (`data.db`) と設定ファイル (`config.toml`) により、USB メモリや単一フォルダで持ち運び可能なポータブル構成を実現。
  - Portable execution without external database setup; stores metrics locally in SQLite (`data.db`).
- **3ペイン対話型 TUI (Interactive 3-Pane TUI Dashboard)**
  - `Ratatui` / `Crossterm` を採用したフルスクリーン TUI。Web ブラウザなしでターミナル上でトポロジー・ポートエラー・アラートを一目で特定。
  - Fullscreen TUI built with Ratatui/Crossterm for terminal-only troubleshooting without needing a web browser.
- **直感的な WebGUI (Embedded WebGUI)**
  - ブラウザから複数台の監視、帯域・エラー・CPU/メモリ使用率のリアルタイムグラフ、LLDP/CDP による自動トポロジー描画が可能。
  - Clean web dashboard for multi-device status, real-time charts, and automated network topology map.
- **テンプレート駆動マルチベンダー対応 (Template-Driven Multi-Vendor Support)**
  - TOML テンプレート (`templates/*.toml`) による Cisco, Fortinet, Yamaha 等の OID 拡張や、メモリ取得方式 (`direct` %, `calculated` %) の自動計算に対応。
  - Extendable TOML templates for multi-vendor OIDs (Cisco, Fortinet, Yamaha, etc.) supporting direct (%) and calculated memory metrics.
- **自動探索＆トポロジー認識 (Discovery & Topology Detection)**
  - CIDR 並列スキャンと LLDP / CDP 隣接情報取得による対向機器・対向ポートの自動マッピング。
  - Parallel CIDR host scanning and automated neighbor port discovery via LLDP and CDP.
- **障害予兆＆健全度スコアリング (Health Scoring & Anomaly Detection)**
  - L1/L2 エラー、ディスカード、Late Collision (Duplex Mismatch)、帯域スパイクの検知とヘルススコア自動算出。
  - Detects physical L1/L2 errors, discards, late collisions, and traffic spikes with composite health scores.

---

## 実行方法 / How to Run

### 1. TUI モード / TUI Mode
ターミナル上でインタラクティブな 3 ペインダッシュボードを起動します。
Launch the interactive terminal dashboard:

```bash
cargo run -- --cli
# またはビルド済みバイナリ / Or compiled binary:
./tracepulse --cli
```

### 2. WebGUI モード / WebGUI Mode
内蔵 Web サーバーを起動し、ブラウザから確認できます。
Start the embedded web server and access via web browser:

```bash
cargo run -- --web
# または / Or:
./tracepulse --web
```
起動後、ブラウザで `http://localhost:8080` へアクセスしてください。
After launching, open `http://localhost:8080` in your web browser.

---

## ユーザー操作ガイド / User Operations Guide

### TUI モード操作方法 / TUI Mode Keyboard Shortcuts

TUI モードでは、キーボードのみで直感的に全操作を行えます。

| キー / Key | 機能 (日本語) | Function (English) |
| :---: | :--- | :--- |
| **`Tab`** | **フォーカス切替**: 上ペイン (`Device List`) ↔ 中ペイン (`Interface List`) のフォーカスを切り替えます。 | **Switch Pane Focus**: Toggle focus between Device List (Top) and Interface List (Middle). |
| **`↑` / `↓`**<br>or **`k` / `j`** | **選択・スクロール**: リストの上下移動。ポート数が多い機器では自動的にスクロール追従します。 | **Navigate / Scroll**: Move selection up/down. Auto-scrolls for devices with many ports. |
| **`Enter`** | **エラー詳細ダイアログ**: (中ペインフォーカス時) 選択ポートの物理エラー（FCS/CRC, Alignment, Giant, MAC Rx）の詳細プログレス内訳を表示。 | **Error Breakdown Modal**: (When Interface Pane focused) View detailed port error breakdown (FCS/CRC, Alignment, Giant, MAC Rx). |
| **`r`** | **手動ポーリング (Manual Poll)**: 全登録機器の SNMP 情報を即時再取得します（進捗プログレスバーを表示）。 | **Manual Poll**: Perform immediate SNMP polling for all devices with progress bar overlay. |
| **`d`** | **ネットワーク自動探索 (Discovery)**: CIDR スキャン＆自動登録ダイアログを開きます。 | **Network Discovery Modal**: Open CIDR discovery dialog for automated device scanning & registration. |
| **`p`** | **プロトコル表示切替**: 中ペインをプロトコル別通信シェアグラフ（NetFlow/sFlow）に切替。 | **Toggle Protocol View**: Switch middle pane to Protocol Share chart (NetFlow/sFlow). |
| **`q`** / **`Esc`** | **終了 / キャンセル**: TUI を安全に終了、またはアクティブなモーダルダイアログを閉じます。 | **Quit / Cancel**: Safely exit TUI or close the active modal dialog. |

#### ネットワーク自動探索 (Discovery) ダイアログの操作 / Network Discovery Modal Controls

1. **`d` キー** を押すとダイアログが開きます。自端末のローカル IP から推測されたサブネット（例: `192.168.11.0/24`）が自動で入力されます。
   Press **`d`** to open the modal. Automatically populates the target CIDR based on local IP (e.g., `192.168.11.0/24`).
2. **`Tab` / `↑` / `↓`**: 入力項目（`Target CIDR Range` ↔ `SNMP Community` ↔ `SNMP Version`）の切り替え。
   Switch active fields.
3. **`←` / `→` / `Home` / `End`**: 入力テキスト内のカーソル移動。
   Move cursor within text fields (`Target CIDR Range` / `SNMP Community`).
4. **`SNMP Version 選択`**: `SNMP Version` フィールド選択中に **`←` / `→` / `Space`** または **`1` / `2` / `3`** キーで `v2c`, `v1`, `v3` を簡単に切り替え可能。
   Toggle SNMP version (`v2c`, `v1`, `v3`) using **`←` / `→` / `Space`** or keys **`1` / `2` / `3`**.
5. **`Enter`**: 64 スレッド並列スキャンを開始。リアルタイム進捗バー表示後、応答のあった機器を DB へ自動登録し、初回ポーリングを一括実行してダッシュボードを更新します。
   Press **`Enter`** to execute 64-thread parallel scan. Discovered devices are auto-registered and polled.

---

### WebGUI モード操作方法 / WebGUI Operations Guide

1. **Dashboard (ダッシュボード)**
   - システム全体の健全度サマリ (Healthy / Warning / Critical / Offline)
   - 登録機器一覧、障害・スパイク発生インジケーター、アラートログ
2. **Device Detail (デバイス詳細)**
   - 帯域利用率 (%)、エラー / ディスカードカウンタ、CPU / メモリ使用率 (%) のリアルタイム Sparkline グラフ
   - ポート単位の健康状態・診断バッジ (Duplex Mismatch, Congestion, L1 Error 等)
   - ハードウェア環境センサー（温度 °C、ファン rpm、電源ステータス）
3. **Device Discovery (デバイス探索)**
   - CIDR スキャンおよび LLDP / CDP 隣接関係に基づくトポロジーマップ自動生成
4. **Settings & Diagnostics (設定・診断)**
   - ポーリング間隔、アラート閾値の設定
   - ベンダー OID プリセット・候補のリアルタイム試行プローブ

---

## 設定ファイル & 構成 / Configuration & File Structure

実行ファイルと同じディレクトリに設定ファイルおよび DB が自動生成・参照されます。

- `tracepulse` (実行バイナリ / Executable)
- `config.toml` (設定ファイル / Configuration)
- `data.db` (SQLite データベース / Database)
- `templates/*.toml` (ベンダー別 OID テンプレート / Vendor Templates)

### `config.toml` 設定例 / Example `config.toml`:

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

## ライセンス / License

このリポジトリは個人開発・オープンソースプロトタイプとして提供されています。
