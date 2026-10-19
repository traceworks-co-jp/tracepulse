# 03. SNMP 収集・数値 OID テンプレート仕様

## 基本方針

- SNMP v2c の数値 OID を直接 GET / GETNEXT する。
- `.mib` / `.txt` のランタイム解析、ASN.1 パーサー、IMPORTS 依存解決、シンボル名解決は実装しない。
- 公式リポジトリ・配布アーカイブ・Docker イメージの監視定義は RFC 標準のみ。公開許諾された追加定義も、出典・利用条件を確認してから採用する。
- 数値 OID の記述だけで、ベンダーとの契約やアクセス権が免除されるわけではない。第三者テンプレートの説明文等にも利用条件があり得るため、適法な入手・利用を確認する。

## 同梱の標準監視

[`templates/generic.toml`](../../templates/generic.toml) をバイナリへ埋め込む。
外部の `templates/` がなくても、以下の標準監視は利用できる。

| 用途 | 標準 |
| --- | --- |
| 基本情報・SNMP 到達性 | RFC 1213: sysName / sysDescr / sysObjectID |
| インターフェース名・死活・トラフィック | RFC 1213 / 2863: ifIndex / ifDescr / ifOperStatus / ifInOctets / ifOutOctets |
| CPU・メモリ | RFC 2790: hrProcessorLoad / hrStorage |
| ハードウェア情報 | RFC 4133 / 3433: ENTITY-MIB / ENTITY-SENSOR-MIB |

既存の EtherLike-MIB のエラー監視と LLDP/CDP 隣接探索も維持する。
CDP のベンダー固有 OID 照会は監視メトリクスと分離した既存トポロジー機能として継続する。
ベンダー固有の CPU・メモリ・環境センサーの自動プローブは行わない。
機器が公開していない CPU・メモリ・センサー値は N/A とし、正常値を捏造しない。

## 信頼性の高い Walk API とトポロジ正規化

既存の `walk_oid` 系 API は互換性のため従来の `Result<Vec<SnmpVarBind>, AppError>` 動作を維持する。
部分取得と正常終了を区別する呼び出し元は `SnmpClient::walk_oid_reliable`（非同期版は
`walk_oid_reliable_async`）を使い、`ReliableWalkOutcome` の状態・理由・件数・cursorを確認する。
取得予算、最大varbind数、要求間隔、中断用 `AtomicBool` を指定できる。要求ごとのsocket timeoutは
残りdeadlineと2秒の小さい方に制限され、要求間隔は最低20msとなる。
上限到達や途中の通信失敗は完全結果ではなくpartialとして返る。明示的な未対応応答のみunsupportedとする。

共有 `snmp::topology` 正規化器はLLDPのリモート行を `[timeMark, localPortNum, remIndex]` で結合し、
管理アドレスを同じ先頭3要素で結合する。LLDPのlocalPortNumはIF-MIBのifIndexと同一視せず、
LLDP local port ID/subtype/descriptionとIF-MIBの名前・説明・必要時MACの一意な一致で解決する。
CDPは `[ifIndex, deviceIndex]` を用いる。Chassis ID等のオクテット列は正規化結果でバイト列のまま保持する。
Webの単発トポロジ探索もこの共有正規化器を使用する。LLDP local port tableとIF-MIBの
ifName・ifDescr・ifAlias・ifPhysAddressを取得して照合し、未解決・曖昧なポートのifIndexは
`null`のままとする。管理アドレスは同一ローカルポート上でもリモート行ごとに結合する。
既存のWebレポート形式、ラベル、ノード上限、LLDP/CDP重複リンク統合は維持し、各Walkの失敗は警告として返す。

## BYO ディレクトリと読み込み

1. 起動時に `<data_dir>/custom_templates/` を作成する。
2. 内蔵 RFC テンプレートに加え、直下の `*.toml` をファイル名順に読み込む。サブディレクトリ、`.mib`、`.txt` は読まない。
3. TOML と数値 OID を検証する。不正ファイル・読込失敗はパス付きでエラーログへ報告し、そのファイルだけを除外する。他の正常な定義と標準監視は維持する。
4. 読み込んだ定義をプロセス内で共有する。追加・変更の反映には再起動が必要。

`data_dir` は `TRACEPULSE_DATA_DIR`、未指定時は実行ファイルのあるディレクトリ。
カレントディレクトリを利用したい場合は `TRACEPULSE_DATA_DIR=.` とする。
起動ログは標準エラー出力へ出力する。`RUST_LOG` でログレベルを指定できる。

### Docker

```sh
docker run -p 8080:8080 \
  -v tracepulse-data:/data \
  -v ./my_templates:/data/custom_templates:ro \
  ghcr.io/traceworks-co-jp/tracepulse:latest
```

バインドマウント元は事前に作成する。標準定義はバイナリ内蔵のため、このマウントで失われない。

## TOML スキーマ

```toml
name = "Example RFC Host"                  # 必須・空白のみ不可
description = "User-supplied RFC metrics"  # 省略可能
sysObjectID_prefix = "1.3.6.1.4.1.65000."   # 実機のプレフィックスに置換

[[metrics]]
key = "uptime"                            # 一意な英数字・アンダースコア
oid = "1.3.6.1.2.1.1.3.0"                 # 必須・数値 OID
unit = "ticks"                            # 必須（空文字も可）
label = "System uptime"                   # 必須・表示名
data_type = "counter"                     # gauge または counter

[tables.interfaces]                      # 省略時は内蔵 IF-MIB 定義
base_oid = "1.3.6.1.2.1.2.2.1"
index = "1.3.6.1.2.1.2.2.1.1"
name = "1.3.6.1.2.1.2.2.1.2"
in_octets = "1.3.6.1.2.1.2.2.1.10"
out_octets = "1.3.6.1.2.1.2.2.1.16"
```

実用例: [`examples/custom_templates/standard_host.toml`](../../examples/custom_templates/standard_host.toml)。
この例の識別子と CPU インデックスは、実機に合わせて変更する。

### 選択規則

- `sysObjectID_prefix` は数値 OID の**要素境界**で前方一致する。末尾の `.` は省略可能。たとえば `...65000` は `...650000` に一致しない。
- 複数一致時は最長プレフィックスを選択する。名前では選択しない。
- 既存 BYO 定義の `enterprise_id = 65000` も利用でき、`1.3.6.1.4.1.65000` として扱う。両方指定時は `sysObjectID_prefix` を優先する。
- 同一プレフィックスの重複はファイル名順で最初の定義を採用し、後続をエラーログへ報告する。
- BYO 定義にはプレフィックスか非ゼロの enterprise_id が必要。内蔵の汎用フォールバックを置き換えることはできない。
- 一致なしは内蔵 RFC 定義。独自テンプレートが interfaces を省略しても IF-MIB を引き続き利用する。

### メトリクス

- `[[metrics]]` の各 `oid` をスカラー GET する。インスタンス `.0` またはテーブルの具体的な行インデックスまで指定する。自動的に `.0` を補完しない。
- Integer / Gauge32 / Counter32 / Counter64 / TimeTicks を数値として受け付ける。ゼロ・負数を有効な測定値として維持する。文字列を数値へ暗黙変換しない。利用中の SNMP ライブラリは 9 バイトの符号付き BER 整数として符号化された Counter64（2^63 以上）を復号できないため、その場合は取得エラーとなる。
- `data_type` は表示・API 上の種別。counter は累積の現在値であり、自動レート変換しない。
- 機器詳細ダッシュボードの **Template Metrics** にラベル・現在値・単位を表示する。ページ更新時に取得し、独自メトリクスの履歴保存・閾値アラート・予測評価は行わない。
- API `/api/device/<ip>` は `template_metrics` 配列に `key / label / oid / unit / data_type / value / value_text / error` を返す。`value_text` は復号済みの Counter64 をブラウザでも桁落ちさせず表示するための文字列表現。取得失敗時は `value: null` とエラーを返し、画面には N/A とエラーを表示する。機器識別等の失敗は `template_metrics_error` に返す。拡張機能による passive モードでは SNMP を行わない。
- ベンダーの CPU を既存 CPU チャート・アラートにも利用する場合は、下記の `[cpu]` を設定する。`key = "cpu_usage"` だけでは既存チャートへ自動割当しない。

### 既存 BYO の互換セクション

```toml
[cpu]
candidates = ["1.3.6.1.2.1.25.3.3.1.2"]

[memory]
mode = "calculated"
used_prefix = "1.3.6.1.2.1.25.2.3.1.6"
total_prefix = "1.3.6.1.2.1.25.2.3.1.5"
# direct モードでは利用率を返す数値 OID を oid に指定する
```

CPU 候補、メモリ、センサーの OID も数値形式を検証する。
`util_prefix` は `memory.oid`、旧 `used_oid / free_oid / total_oid` は各 prefix の別名として扱う。
既存の `[metrics]` 形式のメモリ定義も維持する。
実測値でない `fallback_builtin_power = true` は受け付けない。
センサーの状態値の意味は機器依存のため、独自状態を通常の数値メトリクスとして表示する方が安全な場合がある。

### インターフェーステーブル

`[tables.interfaces]` を指定する場合は上記 5 項目がすべて必須。
4 つの列 OID は base_oid の配下でなければならない。
`index` の値は単一の ifIndex に対応し、列 OID に `.ifIndex` を付けて取得する。
リンク状態・エラー・パケット数等は IF-MIB の同じ ifIndex を使用するため、任意の複合インデックステーブルには対応しない。
トラフィックは既存の差分計算へ渡す。counter 幅は既存の IF-MIB Counter32 を前提とする。

## 旧構成からの移行

- 旧 `templates/*.toml` のベンダー定義は配布しない。利用権を確認した独自定義だけを `custom_templates/` に移す。
- `templates/` の外部自動探索は廃止。内蔵標準定義を上書きする用途には利用できない。
- 自動ベンダープローブ廃止により、ベンダー独自 CPU 等は BYO または明示的な手動 OID 設定が必要。
- LLDP/CDP の既存トポロジー探索は監視メトリクスとは分離し、従来どおり維持する。

## 免責事項

WebGUI の設定画面（SNMP セクション）にも表示する。

> TracePulse monitors standard RFC-compliant metrics by default. When configuring proprietary vendor OIDs or importing third-party templates, please ensure compliance with your respective hardware vendor's EULA and support agreements. TraceWorks assumes no liability for unauthorized access to proprietary vendor metrics.

TracePulse は既定で RFC 標準準拠のメトリクスを監視します。ベンダー独自 OID の設定や第三者テンプレートの利用時は、各ハードウェアベンダーの EULA およびサポート契約への準拠をご確認ください。TraceWorks は、ベンダー独自メトリクスへの無許可アクセスについて責任を負いません。
