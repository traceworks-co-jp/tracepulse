# TracePulse

- [English Documentation](#english)
- [日本語ドキュメント](#日本語)

---

<a id="english"></a>
# English

TracePulse is a portable, single-binary network device monitoring application written in Rust. It provides both an interactive terminal dashboard (TUI) and an embedded WebGUI for SNMP-based network health monitoring, anomaly detection, and topology discovery.

## Key Features

- **Single Binary & Portable Execution**
  - Self-contained execution with SQLite (`data.db`) and configuration (`config.toml`). Portable across folders or USB drives without external database installation.
- **Interactive 3-Pane TUI Dashboard**
  - Fullscreen terminal UI built with `Ratatui` and `Crossterm`. Enables rapid troubleshooting for topology, port errors, and alert streams without needing a web browser.
- **Embedded WebGUI**
  - Web dashboard featuring multi-device monitoring, real-time charts for bandwidth, error counters, and CPU/memory utilization, as well as auto-generated topology maps via LLDP/CDP.
- **Template-Driven Multi-Vendor Support**
  - Easily extend OIDs for Cisco, Fortinet, Yamaha, etc., using TOML templates (`templates/*.toml`). Supports both direct (`mode = "direct"`) and calculated (`mode = "calculated"`) memory metrics.
- **Network Discovery & Topology Detection**
  - Parallel CIDR host scanning and automated neighbor port discovery via LLDP and CDP.
- **Health Scoring & Anomaly Detection**
  - Real-time detection of L1/L2 errors, discards, late collisions (duplex mismatch), traffic spikes, and automated health score calculation.

---

## How to Run

### 1. TUI Mode
Launch the interactive terminal dashboard:

```bash
cargo run -- --cli
# Or compiled binary:
./tracepulse --cli
```

### 2. WebGUI Mode
Start the embedded web server and access via web browser:

```bash
cargo run -- --web
# Or:
./tracepulse --web
```
After launching, open `http://localhost:8080` in your web browser.

---

## User Operations Guide

### TUI Mode Keyboard Shortcuts

In TUI mode, all operations can be performed using keyboard shortcuts:

| Key | Function |
| :---: | :--- |
| **`Tab`** | **Switch Pane Focus**: Toggle focus between Device List (Top) and Interface List (Middle). |
| **`↑` / `↓`** or **`k` / `j`** | **Navigate / Scroll**: Move selection up/down. Auto-scrolls for devices with many ports. |
| **`Enter`** | **Error Breakdown Modal**: (When Interface Pane focused) View detailed port error breakdown (FCS/CRC, Alignment, Giant, MAC Rx). |
| **`r`** | **Manual Poll**: Perform immediate SNMP polling for all devices with progress bar overlay. |
| **`d`** | **Network Discovery Modal**: Open CIDR discovery dialog for automated device scanning & registration. |
| **`p`** | **Toggle Protocol View**: Switch middle pane to Protocol Share chart (NetFlow/sFlow). |
| **`q`** / **`Esc`** | **Quit / Cancel**: Safely exit TUI or close the active modal dialog. |

#### Network Discovery Modal Controls

1. Press **`d`** to open the modal. Target CIDR is automatically populated based on local IP (e.g., `192.168.11.0/24`).
2. **`Tab` / `↑` / `↓`**: Switch between input fields (`Target CIDR Range` ↔ `SNMP Community` ↔ `SNMP Version`).
3. **`←` / `→` / `Home` / `End`**: Move cursor within text fields.
4. **`SNMP Version Selection`**: Toggle SNMP version (`v2c`, `v1`, `v3`) using **`←` / `→` / `Space`** or keys **`1` / `2` / `3`**.
5. **`Enter`**: Start 64-thread parallel scan. Discovered devices are auto-registered and immediately polled.

---

### WebGUI Operations Guide

1. **Dashboard**
   - System-wide health summary cards (Online, Warning, Offline, Total, Error Spikes).
   - Registered device list with status indicators and recent alert stream.
2. **Device Detail**
   - Real-time Sparkline charts for Bandwidth Utilization (%), Error/Discard counters, CPU/Memory Utilization (%).
   - Diagnostic status badges per port (e.g., Duplex Mismatch, Congestion, L1 Error).
   - Hardware environmental status (Temperature °C, Fan RPM, Power Supply status).
3. **Device Discovery & Topology Map**
   - CIDR scanner and auto-generated network topology map based on LLDP/CDP neighbors.
4. **Settings & Diagnostics**
   - Configure polling intervals, alert thresholds, and retention periods.
   - Real-time probe tool for sysObjectID, vendor OIDs, and candidates.

---

## Configuration & File Structure

Configuration files and database are created and referenced in the same directory as the executable:

- `tracepulse` (Executable)
- `config.toml` (Configuration File)
- `data.db` (SQLite Database)
- `templates/*.toml` (Vendor OID Templates)

### Example `config.toml`:

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

## License

This repository is provided as a personal project / open-source prototype.

---

<a id="日本語"></a>
# 日本語

TracePulse は、SNMP を用いてネットワーク機器の異常兆候を検知し、対話型 TUI (ターミナルUI) と WebGUI の両方を同一バイナリで提供する Rust 製のポータブル監視アプリケーションです。

## 主な特徴

- **ポータブル＆単一バイナリ**
  - 内蔵 SQLite (`data.db`) と設定ファイル (`config.toml`) により、外部 DB のセットアップ不要で USB メモリや単一フォルダで持ち運び可能な構成を実現。
- **3ペイン対話型 TUI ダッシュボード**
  - `Ratatui` / `Crossterm` を採用したフルスクリーン TUI。Web ブラウザなしでターミナル上でトポロジー・ポートエラー・アラートを一目で特定。
- **直感的な WebGUI**
  - ブラウザから複数台の監視、帯域・エラー・CPU/メモリ使用率のリアルタイムグラフ、LLDP/CDP による自動トポロジー描画が可能。
- **テンプレート駆動マルチベンダー対応**
  - TOML テンプレート (`templates/*.toml`) による Cisco, Fortinet, Yamaha 等の OID 拡張や、メモリ取得方式 (直接 `%` 取得、`Used/Free` からの自動計算) に対応。
- **自動探索＆トポロジー認識 (Discovery)**
  - CIDR 並列スキャンと LLDP / CDP 隣接情報取得による対向機器・対向ポートの自動マッピング。
- **障害予兆＆健全度スコアリング**
  - L1/L2 エラー、ディスカード、Late Collision (Duplex 不整合)、帯域スパイクの検知とヘルススコア自動算出。

---

## 実行方法

### 1. TUI モード
ターミナル上でインタラクティブな 3 ペインダッシュボードを起動します。

```bash
cargo run -- --cli
# またはビルド済みバイナリ:
./tracepulse --cli
```

### 2. WebGUI モード
内蔵 Web サーバーを起動し、ブラウザから確認できます。

```bash
cargo run -- --web
# または:
./tracepulse --web
```
起動後、ブラウザで `http://localhost:8080` へアクセスしてください。

---

## ユーザー操作ガイド

### TUI モード操作方法

TUI モードでは、キーボードのみで直感的に全操作を行えます。

| キー | 機能 |
| :---: | :--- |
| **`Tab`** | **フォーカス切替**: 上ペイン (`Device List`) ↔ 中ペイン (`Interface List`) のフォーカスを切り替え。 |
| **`↑` / `↓`** または **`k` / `j`** | **選択・スクロール**: リストの上下移動。ポート数が多い機器では自動的にスクロール追従。 |
| **`Enter`** | **エラー詳細ダイアログ**: (中ペインフォーカス時) 選択ポートの物理エラー (FCS/CRC, Alignment, Giant, MAC Rx) の詳細内訳を表示。 |
| **`r`** | **手動ポーリング (Manual Poll)**: 全登録機器の SNMP 情報を即時再取得 (進捗プログレスバーを表示)。 |
| **`d`** | **ネットワーク自動探索 (Discovery)**: CIDR スキャン＆自動登録ダイアログを開く。 |
| **`p`** | **プロトコル表示切替**: 中ペインをプロトコル別通信シェアグラフ (NetFlow/sFlow) に切替。 |
| **`q`** / **`Esc`** | **終了 / キャンセル**: TUI を安全に終了、またはアクティブなモーダルダイアログを閉じる。 |

#### ネットワーク自動探索 (Discovery) ダイアログの操作

1. **`d` キー** を押すとダイアログが開きます。自端末のローカル IP から推測されたサブネット（例: `192.168.11.0/24`）が自動入力されます。
2. **`Tab` / `↑` / `↓`**: 入力項目 (`Target CIDR Range` ↔ `SNMP Community` ↔ `SNMP Version`) の切り替え。
3. **`←` / `→` / `Home` / `End`**: テキスト入力フィールド内のカーソル移動。
4. **`SNMP Version 選択`**: `SNMP Version` フィールド選択中に **`←` / `→` / `Space`** または **`1` / `2` / `3`** キーで `v2c`, `v1`, `v3` を切り替え。
5. **`Enter`**: 64 スレッド並列スキャンを開始。応答があった機器を DB へ自動登録し、初回ポーリングを一括実行してダッシュボードを更新。

---

### WebGUI モード操作方法

1. **Dashboard (ダッシュボード)**
   - システム全体の健全度サマリ (Online / Warning / Offline / Total / Error Spikes)
   - 登録機器一覧、障害・スパイク発生インジケーター、アラートログ
2. **Device Detail (デバイス詳細)**
   - 帯域利用率 (%)、エラー / ディスカードカウンタ、CPU / メモリ使用率 (%) のリアルタイム Sparkline グラフ
   - ポート単位の健康状態・診断バッジ (Duplex Mismatch, Congestion, L1 Error 等)
   - ハードウェア環境センサー (温度 °C、ファン rpm、電源ステータス)
3. **Device Discovery (デバイス探索)**
   - CIDR スキャンおよび LLDP / CDP 隣接関係に基づくトポロジーマップ自動生成
4. **Settings & Diagnostics (設定・診断)**
   - ポーリング間隔、アラート閾値の設定
   - ベンダー OID プリセット・候補のリアルタイム試行プローブ

---

## 設定ファイル & 構成

実行ファイルと同じディレクトリに設定ファイルおよび DB が自動生成・参照されます。

- `tracepulse` (実行バイナリ)
- `config.toml` (設定ファイル)
- `data.db` (SQLite データベース)
- `templates/*.toml` (ベンダー別 OID テンプレート)

### `config.toml` 設定例:

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

## ライセンス

このリポジトリは個人開発・オープンソースプロトタイプとして提供されています。
