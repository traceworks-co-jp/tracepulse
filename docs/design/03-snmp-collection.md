# 03. SNMP 収集・数値 OID テンプレート仕様

## 基本方針

- SNMP v2c の数値 OID を直接 GET / GETNEXT する。
- `.mib` / `.txt` のランタイム解析、ASN.1 パーサー、IMPORTS 依存解決、シンボル名解決は実装しない。
- RFC 標準を基礎とし、Cisco・Yamaha RT・Juniper の数値 OID テンプレートも公式リポジトリ・配布アーカイブ・Docker イメージに同梱する。これ以外の追加定義は、出典・利用条件を確認してから採用する。
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
対応ベンダーの CPU・メモリ・環境センサーは、sysObjectID で選択した TOML に従って取得する。ベンダー別のハードコードされたメトリクスプローブは追加しない。
機器が公開していない CPU・メモリ・センサー値は N/A とし、正常値を捏造しない。

### 同梱ベンダーテンプレート

標準定義と次の 3 定義をバイナリに埋め込む。外部 `templates/` の配置は不要で、編集しても内蔵定義は変わらない。

| 定義 | 自動選択 | CPU | メモリ | センサー |
| --- | --- | --- | --- | --- |
| [`cisco.toml`](../../templates/cisco.toml) | enterprise 9 | 1分平均: Total1minRev → Core1min → avgBusy1 | Enhanced MemPool → 旧 MemPool | FRU 電源状態、ENTITY の名前 |
| [`yamaha.toml`](../../templates/yamaha.toml) | enterprise 1182 | 5秒 → 1分 → 5分 | yrhMemoryUtil の利用率 | 標準センサーのみ |
| [`juniper.toml`](../../templates/juniper.toml) | enterprise 2636 | jnxOperatingCPU | jnxOperatingBuffer の利用率 | Operating 温度、名前で分類した FRU 電源・ファン |

- CPU は候補順で最初に有効値がある OID を採用する。Cisco・Juniper のテーブル内は 0〜100 の有効行の最大値を使い、未対応コンポーネントの先頭 0 行だけで全体を判定しない。これは全 CPU の平均ではない。
- Cisco メモリは各ソース内で used と free を**完全な行キー**で結合し、有効な最大容量プールを選ぶ。Enhanced MemPool は 2 要素インデックスを保持する。該当ソースから利用率と used bytes を計算し、取得不可・容量ゼロなら旧 MemPool、さらに HOST-RESOURCES へフォールバックする。プールを合算した総メモリ量ではない。
- Yamaha の各 CPU・メモリ gauge は公式定義で 0〜100 の百分率。snmp_exporter の `scale: 0.01` を転記すると割合になるため適用しない。enterprise 1182 全体を識別対象とするので、RT hardware OID 未対応の機器では標準監視へフォールバックする。
- Juniper CPU・メモリは各 Operating テーブルの最大有効利用率で、同じコンポーネントの値とは限らない。メモリに heap は混在させず、Buffer 利用率をバイト数に変換しない。使用バイト数は独立に取得可能な標準 hrStorage のみを使う。Operating・FRU の 4 要素インデックスを保持する。
- Juniper の FRU は `power / pem / psu` または `fan` を名前に含む行だけ分類する。Routing Engine 等を電源として表示しない。名前の取得失敗・未一致行はこの分類から除外する。Online(6) は正常、Offline(8) はアラーム、それ以外は注意表示とする。
- Cisco FRU の On(2) は正常。Administratively off(3) は故障扱いにせず注意表示、電力拒否・環境停止・故障等はアラームとする。状態だけのセンサーに V / rpm 等の測定単位を付けない。
- 新規定義は模擬 SNMP エージェントで検証する。機種・OS・SNMP view ごとの公開範囲は異なるため、実機で確認し、必要なら BYO で機種別に上書きする。

#### OID の出典

ユーザー提供の生成済み `snmp.yml` の `cisco_device` / `yamaha_rt` / `juniper` にある数値 OID・列挙値を参照し、TracePulse のスキーマ向けに説明・定義を独自に記述した。参考プロジェクトは [Prometheus snmp_exporter](https://github.com/prometheus/snmp_exporter)（[Apache-2.0](https://github.com/prometheus/snmp_exporter/blob/main/LICENSE)、[生成済み定義](https://github.com/prometheus/snmp_exporter/blob/main/snmp.yml)）。Yamaha の百分率の範囲は [公式 RT hardware 定義](https://www.rtpro.yamaha.co.jp/RT/docs/mib/yamaha-rt-hardware.mib.txt) でも確認した。生成済み YAML・MIB 本体・MIB の説明文は同梱しない。これはベンダー契約への準拠やアクセス許諾を保証するものではない。

## 信頼性の高い Walk API とトポロジ正規化

既存の `walk_oid` 系 API は互換性のため従来の `Result<Vec<SnmpVarBind>, AppError>` 動作を維持する。
部分取得と正常終了を区別する呼び出し元は `SnmpClient::walk_oid_reliable`（非同期版は
`walk_oid_reliable_async`）を使い、`ReliableWalkOutcome` の状態・理由・件数・cursorを確認する。
取得予算、最大varbind数、要求間隔、中断用 `AtomicBool` を指定できる。要求ごとのsocket timeoutは
残りdeadlineと2秒の小さい方に制限され、要求間隔は最低20msとなる。
上限到達や途中の通信失敗は完全結果ではなくpartialとして返る。明示的な未対応応答のみunsupportedとする。
要求範囲外の OID は正常終了として判定し、範囲内の同一・逆行 OID は取得失敗（既取得行があれば partial）とする。

共有 `snmp::topology` 正規化器はLLDPのリモート行を `[timeMark, localPortNum, remIndex]` で結合し、
管理アドレスを同じ先頭3要素で結合する。LLDPのlocalPortNumはIF-MIBのifIndexと同一視せず、
LLDP local port ID/subtype/descriptionとIF-MIBの名前・説明・必要時MACの一意な一致で解決する。
CDPは `[ifIndex, deviceIndex]` を用いる。Chassis ID等のオクテット列は正規化結果でバイト列のまま保持する。
Webの単発トポロジ探索もこの共有正規化器を使用する。LLDP local port tableとIF-MIBの
ifName・ifDescr・ifAlias・ifPhysAddressを取得して照合し、未解決・曖昧なポートのifIndexは
`null`のままとする。管理アドレスは同一ローカルポート上でもリモート行ごとに結合する。
ポート ID と説明が異なるインターフェースに一致する場合も曖昧として扱い、一方を優先して推測しない。
既存のWebレポート形式、ラベル、ノード上限、LLDP/CDP重複リンク統合は維持し、各Walkの失敗は警告として返す。

### 正規化品質と last-known-good の維持

`snmp::topology::normalize_lldp_neighbors_with_quality(remote_rows, management_addresses, local_ports, interfaces)`
および `normalize_cdp_neighbors_with_quality(rows, interfaces)` は `NeighborNormalizationOutcome { neighbors, quality }`
を返す。既存の同名（`_with_quality` なし）API と `neighbors` の出力は同じで、部分行やリモートポートの生バイト列も維持する。

`quality.is_complete()` は正規化の品質のみを判定する。last-known-good の置換・削除は、関連する全 Walk の正常完了と
この値の両方を確認してから行う。不正な行インデックスは `rejected_remote_rows`、必須の機器 ID / ポート ID・subtype の不足や
不正値、重複列、復号不能な名前は `incomplete_remote_rows` に報告する。各項目は別途
`invalid_remote_identity_rows / invalid_remote_port_rows / malformed_address_rows / malformed_local_rows / unresolved_local_ports`
で確認できる。件数は列数ではなく行数で、複数カテゴリ間では重複し得る。

LLDP は chassis ID と port ID/subtype を必須とし、曖昧・未解決のローカルポートも不完全とする。
CDP は device ID と port ID を必須とし、管理アドレスと IF-MIB の表示名は省略可能。
不正な関連管理アドレス・ローカル行・IF-MIB 行は保守的に品質を低下させる。
空のリモートテーブルは正常な空結果で、無関係な補助テーブルの不正行だけでは不完全にしない。

## BYO ディレクトリと読み込み

1. 起動時に `<data_dir>/custom_templates/` を作成する。
2. 内蔵 RFC・ベンダーテンプレートに加え、直下の `*.toml` をファイル名順に読み込む。サブディレクトリ、`.mib`、`.txt` は読まない。
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
- BYO が内蔵ベンダーと同一プレフィックスなら、定義全体を置き換えて警告ログへ報告する。フィールド単位のマージではないため、必要な CPU・メモリ・センサーをすべて記述する。BYO 同士の重複はファイル名順で最初の定義を採用し、後続をエラーログへ報告する。
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
aggregation = "first" # 既定 first / テーブルの最大有効値なら max

[memory]
mode = "calculated"
used_prefix = "1.3.6.1.2.1.25.2.3.1.6"
total_prefix = "1.3.6.1.2.1.25.2.3.1.5"
# direct モードでは利用率を返す数値 OID を oid に指定する
# aggregation = "max" は direct モードのテーブルのみで指定可能
# used_is_bytes = true は calculated の used 列がバイト数の場合のみ指定
```

CPU 候補、メモリ、センサーの OID も数値形式を検証する。
`util_prefix` は `memory.oid`、旧 `used_oid / free_oid / total_oid` は各 prefix の別名として扱う。
既存の `[metrics]` 形式のメモリ定義も維持する。
実測値でない `fallback_builtin_power = true` は受け付けない。

`[[memory.fallbacks]]` は `[memory]` と同じ項目を持つ、入れ子にならない順序付きソース。主ソースで有効値を得られないとき順に試す。direct は 0〜100 の値のみ受理し、異常値を 100 に丸めない。calculated は完全な行キーで used と free/total を結合し、正の容量かつ used ≤ capacity の最大容量行を使う。`used_is_bytes` の既定は false で、利用率を使用バイト数として誤解釈しない。CPU・メモリの取得結果は既存チャート・履歴・診断に使用する。

`[[sensors.temperature]]` / `[[sensors.power]]` / `[[sensors.fan]]` は `descr_prefix / value_prefix / state_prefix` を指定する。value/state 列から実際の行キーを取得し、その完全なキーで名前を GET する。名前だけ存在する ENTITY 行はセンサーにしない。複合インデックスは完全な `oid` を識別子として使い、API の旧 `index` は表示用の連番（単一インデックスなら元の数値）とする。列の Walk は 1024 要求までに制限し、通信失敗・逆行 OID・上限到達はそのグループの取得エラーとしてログと診断に報告する。他のグループ・標準センサーは維持する。

利用中の `snmp 0.2.2` は primitive な endOfMibView を varbind として公開せず、イテレーター終端にするため、それも Walk 終了として扱う。このライブラリ境界では空応答・一部の復号不能応答と endOfMibView を厳密には区別できない。終端時の取得件数は debug ログに記録する。

状態の意味は機器依存のため、次の項目で明示できる。

```toml
[[sensors.power]]
descr_prefix = "1.3.6.1.2.1.47.1.1.1.1.7"
state_prefix = "1.3.6.1.4.1.9.9.117.1.1.2.1.2"
normal_states = [2]
alarm_states = [8]
state_labels = { "2" = "On", "8" = "Failed" }
# match_names = ["power", "psu"] # 任意: 大文字小文字を無視した部分一致の OR
```

`alarm_states` を省略した既存 BYO では従来の EnvMon コード 2/3/4/6 を維持する。明示的な空配列はアラームなしを意味する。正常と異常の重複は不正。`state_labels` にないコードは Unknown と実際の値を表示する。`normal_states` を設定したセンサーは API の `status_kind` と WebGUI の通常/注意/異常表示にも反映する。

### インターフェーステーブル

`[tables.interfaces]` を指定する場合は上記 5 項目がすべて必須。
4 つの列 OID は base_oid の配下でなければならない。
`index` の値は単一の ifIndex に対応し、列 OID に `.ifIndex` を付けて取得する。
リンク状態・エラー・パケット数等は IF-MIB の同じ ifIndex を使用するため、任意の複合インデックステーブルには対応しない。
トラフィックは既存の差分計算へ渡す。counter 幅は既存の IF-MIB Counter32 を前提とする。

## 旧構成からの移行

- 公式同梱のベンダーは上記 3 定義に限定する。それ以外の旧 `templates/*.toml` は、利用権を確認して `custom_templates/` に移す。
- `templates/` の外部自動探索は行わない。内蔵ベンダー定義の変更は `custom_templates/` の同一セレクターで行う。汎用 RFC フォールバックは置き換えない。
- 同梱ベンダー以外の独自 CPU 等は BYO または明示的な手動 OID 設定が必要。
- LLDP/CDP の既存トポロジー探索は監視メトリクスとは分離し、従来どおり維持する。

## 免責事項

WebGUI の設定画面（SNMP セクション）にも表示する。

> TracePulse uses RFC-compliant baseline monitoring and automatically selects bundled vendor templates for recognized devices. When using proprietary vendor OIDs or third-party templates, please ensure compliance with your respective hardware vendor's EULA and support agreements. TraceWorks assumes no liability for unauthorized access to proprietary vendor metrics.

TracePulse は RFC 標準準拠の監視を基礎とし、識別した機器には同梱のベンダーテンプレートを自動選択します。ベンダー独自 OID や第三者テンプレートの利用時は、各ハードウェアベンダーの EULA およびサポート契約への準拠をご確認ください。TraceWorks は、ベンダー独自メトリクスへの無許可アクセスについて責任を負いません。
