# TracePulse

TracePulse は、SNMP v2c を使ってネットワーク機器の異常兆候を検知し、CLI/TUI と WebGUI の両方を同一バイナリで提供する Rust 製のポータブル監視アプリです。

## 目的

- 導入コストゼロで動作するネットワーク機器監視ツールを提供する
- 専門知識が少ない担当者でも、異常の兆候を見つけられるようにする
- USB メモリやフォルダ単位で持ち運びが可能なポータブル構成を実現する
- 現場診断用の CLI/TUI と、複数台監視向けの WebGUI を同一アプリで切り替えて利用する

## 主な機能

- SNMP v2c によるネットワーク機器探索
- 単一 CIDR 指定での自動スキャン
- 機器登録と状態管理
- インターフェース監視
  - Link Up/Down
  - 帯域利用率
  - In/Out Error / Discard
- CPU / Memory 使用率の基礎監視
- 予兆検知
  - Spike 検知
  - Error rate 検知
  - 健全度スコア計算
- CLI / TUI モード
- WebGUI モード（内蔵 Web サーバー）
- SQLite によるデータ保存
- 7日超過履歴のクリーンアップ基盤

## 対象範囲（初期実装）

- L2/L3 スイッチ
- ルータ
- SNMP v2c 対応機器
- 標準 MIB を利用可能な計測項目

以下は Phase 2 以降の対象として整理されています。

- ベンダー固有の環境情報（温度/電源/バッテリー等）
- SPAN / パケット深掘り解析
- Slack / Teams への外部通知

## 実行方法

### CLI / TUI モード

```bash
cargo run -- --cli
```

### WebGUI モード

```bash
cargo run -- --web
```

WebGUI はローカルのブラウザから `http://localhost:8080` で確認できることを想定しています。

## 設定ファイル

実行時に `config.toml` が存在しない場合、デフォルト値が使用されます。

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

## データ保存

- 実行ファイルと同じディレクトリに `config.toml` と `data.db` を生成
- SQLite により監視データと履歴を保持
- 7日超のデータはバックグラウンドで削除する構想に対応

## 現在の実装状況

このリポジトリには現時点で、以下の基盤が実装されています。

- Rust プロジェクトの初期構成
- 設定ファイルの読み込みとデフォルト値
- SQLite の初期化とデータモデル
- 機器登録と CIDR 検出の雛形
- SNMP v2c 用 OID とクライアントの雛形
- 監視ロジックの基本構成
- Alert / スコアリングの基礎
- CLI/TUI の最小画面
- WebGUI の最小ダッシュボード
- 履歴削除ジョブと最小検証ケース

## 今後の方向性

実際の本格運用に向けて、次の項目を段階的に拡張します。

1. 実デバイスへの SNMP 実通信の本実装
2. 実際の IF-MIB 収集と解析
3. エラー率とスパイク判定の精度改善
4. 本格的な TUI / WebUI デザイン
5. 監視ログとアラート履歴の管理強化
6. Phase 2 以降の高度機能の追加

## ライセンス

このリポジトリは現時点では個人開発・設計段階のプロトタイプとして扱われます。
