# 03. SNMP 収集・数値 OID テンプレート仕様

## 基本方針

- SNMP v2c / v3 の数値 OID を直接 GET / GETNEXT する。
- `.mib` / `.txt` のランタイム解析、ASN.1 パーサー、IMPORTS 依存解決、シンボル名解決は実装しない。
- RFC 標準を基礎とし、Cisco・Yamaha RT・Juniper・Fortinet FortiGate・MikroTik RouterOS・Arista EOS の数値 OID テンプレートも公式リポジトリ・配布アーカイブ・Docker イメージに同梱する。これ以外の追加定義は、出典・利用条件を確認してから採用する。
- 数値 OID の記述だけで、ベンダーとの契約やアクセス権が免除されるわけではない。第三者テンプレートの説明文等にも利用条件があり得るため、適法な入手・利用を確認する。

## GUI-managed definitions (SQLite)

共通の接続・認証設定は「設定 → SNMP設定を管理」（`/settings/snmp`）に集約する。
一般設定画面には管理画面へのリンクのみを置き、接続・認証フォームは重複表示しない。
接続設定は専用ボタンから既存の設定APIで暗号化保存し、空欄では既存値を保持する。
一般設定の保存・初期値リセットではコミュニティを変更しない。
監視定義の保存は別ボタンで行い、接続設定や未保存の認証情報を含めない。
TOML入出力にも認証情報は含めない。
診断画面には編集フォームを置かず、対象入力の直後に結果を表示する。
内部のドット区切りキー名はラベルに表示せず、日本語・英語の用途別名称
（例：受信トラフィック（ifInOctets）、CPU使用率OID）を使う。
同梱・BYOのTOMLファイルはGUIから書き換えない。

| 定義元 | 永続化・適用 |
| --- | --- |
| 同梱TOML | バイナリ内の初期定義 |
| `custom_templates/*.toml` | 外部管理。起動時に読み込み、同じ識別条件の同梱定義を置換 |
| GUI編集 | SQLite `snmp_definitions` の単一行に、世代番号と構造化JSONとして保存 |

- GUIではプロファイル選択、OID・単位・倍率・計算方法・テーブル定義の編集、
  配列項目の追加・削除、機種別プロファイルの作成、リセット、TOML入出力を行える。
  スキーマ上のフィールド名はTOMLと共通。数値OID以外や不正な計算・倍率は保存できない。
- 編集画面は「CPU・メモリ」「インターフェース・光レベル」「ハードウェアセンサー」
  「プロファイル情報」の4タブに分割する。共通追加OIDの選択時は対象となる2タブだけ表示する。
  監視項目は一覧表にまとめ、設定OIDと設定済み／未設定を表示する。
  この設定状態は実機の取得成功を示すものではなく、画面内での実機問い合わせも行わない。
- 「項目追加」または行末の「編集」でモーダルを開く。編集内容はモーダル内の一時コピーに保持し、
  「編集内容を反映」で下書きへ取り込む。キャンセル・Escapeでは下書きを変更しない。
  基点OID、集約方法、メモリ計算方法、倍率、状態値マッピングなどは「詳細設定」に折りたたむ。
  CPU・インターフェース・光レベルの共通設定もカテゴリ上部の詳細設定ボタンから編集できる。
- プロファイル選択、保存、リセット、TOML入出力、再読込、プロファイル作成は画面上部の
  固定操作バーへ集約する。「保存して適用」を押すまではSQLiteと監視処理に反映しない。
  sysObjectIDによる選択プレビューと保存先・適用順の説明は一覧の下で折りたたみ表示する。
- 編集したプロファイルは**全項目を含むユーザー所有のコピー**として保存する。
  フィールドごとの暗黙的なマージは行わず、そのプロファイルに対する将来の同梱・BYO更新も
  自動取り込みしない。「リセット」で、起動時に読み込んだファイル定義へ戻す。
  GUIのみのプロファイルはリセットで削除する。いずれも保存時に確定する。
- 同梱項目をGUIで削除しても元ファイルは残るため、リセットで復元できる。
  CPU等の取得候補を削除しても、プログラムの標準RFCフォールバックそのものを無効にはしない。
- `generic` は識別条件のない必須フォールバック。機器別定義はsysObjectIDの
  数値アーク単位で最長一致するものを選ぶ。ベンダー定義を共通定義に混ぜない。
- 共通CPU・メモリOIDは0〜100の百分率専用。**選択テンプレート → 共通追加OID →
  標準RFCフォールバック**の順で最初の有効値を採用する。
  選択テンプレートに標準OIDが含まれる場合、その候補も共通追加OIDより優先する。
  以前の手動上書き優先からの意図的な変更で、全機器にベンダー固有OIDを強制しない。
  メモリ割合のOIDを使用バイト数として再利用しない。
- 共通ハードウェアOIDは `temperature|数値OID` / `power|数値OID` /
  `fan|数値OID` / `sensor|数値OID` の形式。標準・テンプレートのセンサーに追加する。
  既に取得した同じOIDは重複追加しない。単位変換や状態値マッピングは機器別プロファイルを使う。
- 初回起動時、旧設定の `cpu_oid_override` / `memory_oid_override` /
  `hardware_oid_overrides` を共通追加OIDへ一度だけ移す。以降はDBを正本とし、
  設定ファイルに旧値が残っていても再取り込みしない。
- 保存要求は世代番号を含む。候補全体を検証してからSQLiteトランザクションで世代を
  条件付き更新し、成功後、同じ排他区間内でメモリ上のスナップショットを交換する。
  不正定義・更新競合・DB書き込み失敗では現在の定義を維持する。
  別編集者の更新に競合した場合、GUIは未保存の編集を残してエラーを表示する。
- 各SNMPクライアントはテンプレートと共通OIDを同じ世代から取得する。
  実行中の処理は旧スナップショットで完了し、新しく開始する監視・診断で新定義を使う。
  保存済み／適用済み世代をGUIに表示する。再起動時はDBから復元し、破損時は起動に失敗する。
- TOMLインポートは未保存の編集として取り込み、保存前には反映しない。
  エクスポートは選択中のプロファイルをTOMLとして出力する。共通追加OIDはDBバックアップに含む。
  DBとTOMLへの二重書き込みはしない。BYOファイルの直接編集には再起動が必要。
- sysObjectIDによる選択プレビューは保存前の定義をサーバー側の実際の選択処理で検証する。
  実機の応答確認には、保存後に同画面のSNMP診断を実行する。

API:

- `GET /api/snmp/definitions`: 有効定義、元定義、ユーザー定義、保存済み／適用済み世代。
- `POST /api/snmp/definitions`: `{generation, definitions: {common, profiles}}` を一括保存・適用。
- `POST /api/snmp/definitions/preview`: `{definitions, sys_object_id}` を検証し、選択プロファイルを返す。保存しない。
- `POST /api/snmp/definitions/convert`: `{toml}` から検証済み定義、または `{document}` からTOMLへ変換。保存しない。
- 旧 `POST /api/diagnostics/oids` は明示的な移行エラーを返す。世代確認のない旧経路では書き込まない。

既存のWeb拡張の認可・変更監査を全APIに適用する。HTTP本文は既存の64KiB上限内で使用する。
アプリを独自に組み込む場合も、DBと設定を読み込んだ後、クライアント作成前に
`snmp::management::initialize(connection, snmp_config)` を呼ぶ。
同じDBを複数の監視プロセスで共有する運用は想定しない。

## デバイス別SNMP認証と継承

- 優先順位は **デバイス個別設定 → システム共通設定**。バージョン、コミュニティ、
  v3ユーザー名、認証方式・パスワード、暗号化方式・パスワードごとに、未指定・空欄・nullを継承として扱う。
  既存デバイスのコミュニティは個別設定として引き続き使用する。
- 共通コミュニティは `[snmp] default_community`、v3設定は初回設定用の
  `[snmp.credentials]`、またはGUIから設定できる。初回読込でv3設定を暗号化した
  `credentials_encrypted` に置き換える。共通設定の空欄送信は既存値を維持する。
- 手動登録で個別設定を入力できる。ダッシュボードの機器操作・一括操作から
  「SNMP認証情報」を開き、選択機器の個別設定を置き換える。
  単一機器の編集では保存済みバージョン・v3ユーザー名・方式を読み込む。
  コミュニティ・認証／暗号化パスワードは返さず、「設定済み」と表示する。
  単一機器の通常保存では空欄の秘密値を維持し、一括編集では従来どおり個別設定を置き換える。
  「共通設定へ戻す」は確認後に個別設定の解除を保存する。別途「保存」を押す必要はない。
  失敗時はエラーを表示し、元の設定を維持する。キャンセルでは変更しない。
- 個別認証は `device_snmp_credentials(device_id, envelope)` に保存する。
  JSON全体を既存のChaCha20-Poly1305で暗号化し、デバイスIPをAADに含める。
  コミュニティ・v3パスワードだけでなくユーザー名も暗号文内に置く。
  一括変更は選択IDを全件検証した後、単一トランザクションで適用する。
  認証情報は機器の登録解除に連動して削除する。
- 設定ファイル内の共通認証も暗号化する。DB・設定ファイルと対応する `secret.key`
  を一緒に安全にバックアップする。鍵の欠落・不一致・暗号文破損では失敗し、
  平文や初期値へ切り替えて監視を続けることはない。
- 秘密値のGUI入力はパスワード型で、保存後にクリアする。GET API・HTML・Debugには
  保存済みコミュニティ・パスワードを含めない。個別設定取得APIはv3ユーザー名・方式と
  秘密値の設定有無だけを返す。`community_configured` / `snmp_individual_configured`
  は設定の有無だけを示す。マスク文字列 `********` の書込みは拒否する。
- v3は `noAuthNoPriv`、`authNoPriv`、`authPriv` をサポートする。
  認証は `sha1` / `sha256` / `sha512`、暗号化は `aes128`。
  認証なし・暗号化なしを選ぶ場合は対応する方式を `none` にする。
  既定はSHA-256 + AES-128、パスワードは8バイト以上、ユーザー名は32バイト以内。
  UDP/161でエンジンID・時刻を取得してから認証付きの問い合わせを開始する。
- ポーリング、DOM/WALK、登録済み機器の詳細取得・CLI隣接取得は解決済み認証を使う。
  認証エラーが起きても共通キーやv2cへ再試行しない。CIDR探索の入力は従来どおりv2c用で、
  v3機器は手動登録する。単体の診断ページは登録済み個別設定を自動参照しない。

初回設定例（起動後に認証情報は暗号文へ置き換わる）:

```toml
[snmp]
default_community = "replace-with-your-community"

[snmp.credentials]
version = "3"
v3_username = "monitor"
v3_auth_protocol = "sha256"
v3_auth_password = "replace-with-auth-password"
v3_priv_protocol = "aes128"
v3_priv_password = "replace-with-privacy-password"
```

`POST /api/devices/snmp-credentials` は `{device_ids, credentials}` を受け取り、
個別値を一括置換する。`credentials: {}` は全項目の継承、未指定／null／空文字の項目は
当該項目の継承を表す。共通値の更新は `POST /api/settings` の `snmp_credentials` を使い、
こちらの空欄は共通値を消さず維持する。手動登録APIの各機器にも `snmp_credentials` を指定できる。

## RFC標準定義の内容

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

標準定義と次の 11 定義をバイナリに埋め込む。外部 `templates/` の配置は不要で、編集しても内蔵定義は変わらない。

| 定義 | 自動選択 | CPU | メモリ | センサー |
| --- | --- | --- | --- | --- |
| [`cisco.toml`](../../templates/cisco.toml) | enterprise 9 | 1分平均: Total1minRev → Core1min → avgBusy1 | Enhanced MemPool → 旧 MemPool | FRU 電源状態、ENTITY の名前 |
| [`yamaha.toml`](../../templates/yamaha.toml) | enterprise 1182 | 5秒 → 1分 → 5分 | yrhMemoryUtil の利用率 | 標準センサーのみ |
| [`juniper.toml`](../../templates/juniper.toml) | enterprise 2636 | jnxOperatingCPU | jnxOperatingBuffer の利用率 | Operating 温度、名前で分類した FRU 電源・ファン |
| [`fortinet.toml`](../../templates/fortinet.toml) | sysObjectID prefix `1.3.6.1.4.1.12356.101` | `fgSysCpuUsage` | `fgSysMemUsage` | 標準センサー収集、`fgSysSesCount` を追加メトリクス表示 |
| [`mikrotik.toml`](../../templates/mikrotik.toml) | enterprise 14988 | HOST-RESOURCES `hrProcessorLoad` | 標準フォールバック | MIBの電圧・CPU温度を0.1単位の追加メトリクス表示 |
| [`arista.toml`](../../templates/arista.toml) | sysObjectID prefix `1.3.6.1.4.1.30065.1` | HOST-RESOURCES `hrProcessorLoad` | 標準フォールバック | 標準ENTITY-SENSOR-MIB収集を使用 |
| [`paloalto.toml`](../../templates/paloalto.toml) | enterprise 25461 | MP scalar + DPテーブル最大値 | 標準フォールバック | Active / Maximum Sessionsを追加メトリクス表示 |
| [`ubiquiti.toml`](../../templates/ubiquiti.toml) | enterprise 41112 | Ubiquiti CPU → HOST-RESOURCES | Total - Freeから算出し、使用bytesも記録 | 温度OIDを追加メトリクス表示 |
| [`hp_aos_s.toml`](../../templates/hp_aos_s.toml) | enterprise 11 | `hpSwitchCpuStat` | Total - Freeから算出し、使用bytesも記録 | 標準センサー収集 |
| [`aruba_cx.toml`](../../templates/aruba_cx.toml) | enterprise 47196 | AOS-CX CPU → HOST-RESOURCES | AOS-CX直接割合OID | 標準センサー収集 |
| [`allied_telesis.toml`](../../templates/allied_telesis.toml) | enterprise 207 | 5秒平均CPU | メモリ使用率OID | Fan / PSU状態テーブル |

- CPU は候補順で最初に有効値がある OID を採用する。Cisco・Juniper のテーブル内は 0〜100 の有効行の最大値を使い、未対応コンポーネントの先頭 0 行だけで全体を判定しない。これは全 CPU の平均ではない。
- Cisco メモリは各ソース内で used と free を**完全な行キー**で結合し、有効な最大容量プールを選ぶ。Enhanced MemPool は 2 要素インデックスを保持する。該当ソースから利用率と used bytes を計算し、取得不可・容量ゼロなら旧 MemPool、さらに HOST-RESOURCES へフォールバックする。プールを合算した総メモリ量ではない。
- Yamaha の各 CPU・メモリ gauge は公式定義で 0〜100 の百分率。snmp_exporter の `scale: 0.01` を転記すると割合になるため適用しない。enterprise 1182 全体を識別対象とするので、RT hardware OID 未対応の機器では標準監視へフォールバックする。
- Juniper CPU・メモリは各 Operating テーブルの最大有効利用率で、同じコンポーネントの値とは限らない。メモリに heap は混在させず、Buffer 利用率をバイト数に変換しない。使用バイト数は独立に取得可能な標準 hrStorage のみを使う。Operating・FRU の 4 要素インデックスを保持する。
- Juniper の FRU は `power / pem / psu` または `fan` を名前に含む行だけ分類する。Routing Engine 等を電源として表示しない。名前の取得失敗・未一致行はこの分類から除外する。Online(6) は正常、Offline(8) はアラーム、それ以外は注意表示とする。
- Cisco FRU の On(2) は正常。Administratively off(3) は故障扱いにせず注意表示、電力拒否・環境停止・故障等はアラームとする。状態だけのセンサーに V / rpm 等の測定単位を付けない。
- Fortinet の CPU・メモリは0〜100の直接割合値として扱い、セッション数は追加数値メトリクスとして表示する。FortiGateの機種・FortiOS・SNMP viewによって公開OIDや応答は異なる。
- MikroTik の電圧・CPU温度OIDは0.1単位の生値を返す。メトリクスscale 0.1を掛け、V / Cへ換算する。CPU負荷は標準HOST-RESOURCES-MIBを使う。
- Arista はCPUに標準HOST-RESOURCES-MIBを選択し、温度・ファン・電源などは全機器共通の標準ENTITY-MIB / ENTITY-SENSOR-MIB収集に任せる。テンプレート側でセンサー行・列の独自対応付けは追加しない。
- PAN-OS CPUはMP scalarを通常CPUとして取得し、DP CPUテーブルはpercent追加メトリクスとしてscale後に最大値を表示する。Active / Maximum Sessionsは割合値に変換しない。
- Ubiquiti / HPE AOS-Sのメモリ計算は行キーごとに`(Total - Free) / Total * 100`を行い、使用bytesは`Total - Free`として記録する。対応するTotal / Freeメトリクスも個別に表示する。
- 数値追加メトリクスの`scale`は生値に乗算してから表示・保存する。既定値は1。percent単位はscale適用後に0〜100の範囲を検証し、指定aggregation（first / max）で集約する。Web/TUI共通のポーリングで取得できた値は`template_metric_samples`へ機器・metric key・値・単位・取得時刻とともに保存し、通常の履歴保持日数で削除する。欠測値・取得エラーは保存しない。
- HPE AOS-SのCPUは百分率、メモリTotal / Freeはbyte値。enterprise 11の選択は配下の他製品にも一致し得るため、対象外機器にはより具体的な`sysObjectID_prefix`を用意する。Aruba AOS-CXはPEN 47196で別に選択し、指定されたCPU・メモリ直接割合OIDを使う。CPUにはHOST-RESOURCES-MIBの候補フォールバックを残す。
- Allied Telesisは提示されたCPU・メモリOIDとFan / PSUの名前・状態列を使用する。状態値2をNormalとし、その他は正常と決めつけずwarning表示する。
- 新規定義は模擬 SNMP エージェントで検証する。機種・OS・SNMP view ごとの公開範囲は異なるため、実機で確認し、必要なら BYO で機種別に上書きする。

#### OID の出典

既存Cisco・Yamaha・Juniper定義のOID・列挙値はユーザー提供の生成済み `snmp.yml` を参照した。Fortinet、MikroTik、Arista、PAN-OS、Ubiquiti、HPE AOS-S / Aruba CX、Allied Telesisの追加OIDは依頼時に提示された数値OIDを使い、未提示の取得項目は標準MIBへフォールバックするか追加していない。追加定義ではMIBファイル本文や説明文を再配布しない。実機モデル・OS・SNMP viewごとの公開範囲は異なるため、正式運用前に対象機器で値と単位を確認する。数値OIDの利用だけでベンダー契約への準拠やアクセス許諾が保証されるものではない。参考プロジェクトは [Prometheus snmp_exporter](https://github.com/prometheus/snmp_exporter)（[Apache-2.0](https://github.com/prometheus/snmp_exporter/blob/main/LICENSE)、[生成済み定義](https://github.com/prometheus/snmp_exporter/blob/main/snmp.yml)）。

## Optical transceiver power (DOM / DDM)

DOM is optional and template-driven. The Web and TUI pollers walk each configured
DOM column once per device/cycle, then join the result to interface samples by
`ifIndex`. No DOM table is assumed for the generic RFC template: DOM is not a
uniform IF-MIB column. Copper ports, missing SNMP instances, nonnumeric values,
and unavailable sensor readings remain unmeasured (`null` / `N/A`); they do not
interrupt traffic collection. Transport/walk errors are logged and the optional
DOM collection is omitted for that cycle, rather than reusing stale values.

### Direct dBm tables

```toml
[tables.dom]
base_oid = "1.3.6.1.4.1.2636.3.60.1.1.1.1"
# Omit index_oid when the row suffix is already ifIndex, as on Juniper.
# Otherwise supply a numeric column whose VALUE is the corresponding ifIndex:
# index_oid = "1.3.6.1.2.1.2.2.1.1"

[[tables.dom.metrics]]
key = "rx_power"
oid = "1.3.6.1.4.1.2636.3.60.1.1.1.1.5"
unit = "dBm"
scale = 0.01
label = "Rx Power"
invalid_values = [] # Raw, BEFORE scaling; add only documented device sentinels.

[[tables.dom.metrics]]
key = "tx_power"
oid = "1.3.6.1.4.1.2636.3.60.1.1.1.1.7"
unit = "dBm"
scale = 0.01
label = "Tx Power"
```

`base_oid` and metric column OIDs are required numeric OIDs; metric columns must
be beneath the base. `key` is `rx_power` or `tx_power`, each at most once, with
`unit = "dBm"` and a nonempty label. `scale` defaults to 1 and must be finite and
positive. `invalid_values` defaults to empty and must contain finite numbers.
An explicit `index_oid` joins by the complete row suffix, not by row order. If it
is omitted, a single positive suffix component is the ifIndex. Composite/lane
rows are not silently truncated to a port index.

The bundled Juniper definition uses Rx column `.5` and **Tx column `.7`**.
Column `.6` is laser bias current, not Tx power. Both optical power columns
report hundredths of dBm. References:
[Juniper MIB Explorer](https://apps.juniper.net/mib-explorer/),
[Rx numeric OID](https://oidref.com/1.3.6.1.4.1.2636.3.60.1.1.1.1.5),
[Tx numeric OID](https://oidref.com/1.3.6.1.4.1.2636.3.60.1.1.1.1.7).

### Cisco ENTITY sensors

The bundled Cisco definition additionally supplies `[tables.dom.entity]` with
numeric `name_oid`, `contained_in_oid`, `alias_oid`, `type_oid`, `scale_oid`,
`precision_oid`, and `status_oid` columns. It is mutually exclusive with
`index_oid`. Each metric has case-insensitive `match_names` filters for
receive/transmit power sensors. All mapping and measurement OIDs remain in TOML.

The collector follows ENTITY-MIB containment from sensor to physical port,
resolves `entAliasMappingIdentifier` values referencing IF-MIB `ifIndex`, and
requires an unambiguous mapping. It never assumes `entPhysicalIndex == ifIndex`
or matches interface names by substring. Sensor status must be OK(1), and the
sensor type must explicitly be watts(6). SI scale, fractional precision, and the
optional metric `scale` multiplier are applied before conversion:
`dBm = 10 * log10(watts * 1000)`. Non-positive precision
describes accuracy and does not add fractional digits. Unsupported unit types
(including vendor-specific `other(1)` readings that may or may not be dBm),
missing metadata, zero/negative watts, underflow/overflow sentinels, conflicting
aliases, and multiple same-direction readings for one port remain N/A.

The Cisco template uses the
[public numeric CISCO-ENTITY-SENSOR-MIB definitions](https://oidref.com/1.3.6.1.4.1.9.9.91.1.1.1.1)
and ENTITY-MIB mapping; this is not certification of every Cisco series.
Devices without ENTITY alias mappings or with proprietary native-dBm formats
need a model-specific template with a verified index column and units.
MIB source text is not distributed or parsed.

### Storage, display, and scope

SQLite retains the existing `rx_optical_power_dbm` column for compatibility and
adds nullable `tx_power_dbm` using an additive migration. All interface history
and latest-sample read paths retain both values. The device API exposes
`interfaces[].rx_power_dbm` and `interfaces[].tx_power_dbm` as dBm numbers or
`null`. WebGUI and TUI interface tables show Tx/Rx columns with one decimal
place. Either value **<= -20 dBm** is yellow as a display-only advisory, not a
device alarm or a transceiver-specific operating limit.

**-40 dBm is not globally discarded**: it can be a valid low-power reading.
Declare a documented raw sentinel in the relevant metric's `invalid_values`
(for example `[-4000]` with `scale = 0.01`) only for models where it is invalid.
Explicit null readings are not replaced by historical measurements.

This implementation is port-level, not per-lane telemetry. Juniper lane tables,
Cisco names containing lane/channel, and ambiguous multiple sensors are not
aggregated into a fabricated port value. Per-lane storage/UI is outside this
scope. Tests use mock SNMP agents and database migration fixtures; real device
model/firmware validation is still required.

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

> TracePulse uses RFC-compliant baseline monitoring and automatically selects bundled vendor templates for recognized devices. When using proprietary vendor OIDs or third-party templates, please ensure compliance with your respective hardware vendor's EULA and support agreements. TraceWorks Co., Ltd. assumes no liability for unauthorized access to proprietary vendor metrics.

TracePulse は RFC 標準準拠の監視を基礎とし、識別した機器には同梱のベンダーテンプレートを自動選択します。ベンダー独自 OID や第三者テンプレートの利用時は、各ハードウェアベンダーの EULA およびサポート契約への準拠をご確認ください。TraceWorks株式会社 は、ベンダー独自メトリクスへの無許可アクセスについて責任を負いません。
