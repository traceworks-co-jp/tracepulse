# 03. SNMP 収集詳細設計

## 目的

SNMP v2c での収集項目、フォールバック順、候補表示を定義する。

## 収集対象

- sysName
- sysDescr
- sysObjectID
- interface 情報
- CPU
- hardware sensor

## 実装関数

- `SnmpClient::query_device()`
- `SnmpClient::probe_device()`
- `SnmpClient::discover_interface_indexes()`
- `SnmpClient::query_interface()`
- `SnmpClient::query_hardware_sensors()`
- `SnmpClient::diagnose_device()`

## 収集フロー詳細

### 1. 基本情報

- `sysName`
- `sysDescr`
- `sysObjectID`

### 2. CPU

- vendor OID を優先
- table walk で平均値取得
- `diagnose_device` では候補一覧も返す

### 3. Interface

- `ifIndex` を列挙
- `ifName` と `linkStatus` を取得
- bandwidth と error/discard の履歴値は monitor 側で計算

### 4. Hardware sensors

- `entity-sensor` から実測値を取得
- `entity-physical` から部品名を補完
- 温度 / 電源 / ファンは unit を付けて返す

## 診断ページ向け出力

- `cpu_probes`
- `hardware_probes`
- `hardware_sensors`
- `interfaces`

## 失敗パターン

- 取得不可: N/A
- table walk 空: 候補なし
- OID 不一致: 候補から除外

## 図: SNMP 収集シーケンス

```text
Web/CLI -> SnmpClient::query_device
SnmpClient -> sysName/sysDescr/sysObjectID
SnmpClient -> query_cpu_usage
SnmpClient -> discover_interface_indexes
SnmpClient -> query_interface
SnmpClient -> query_hardware_sensors
SnmpClient -> SnmpDeviceInfo / SnmpDiagnostics
```

## 図: クラス関係

```text
SnmpClient
 ├─ query_device()
 ├─ diagnose_device()
 ├─ query_interface()
 └─ query_hardware_sensors()

SnmpDiagnostics
 ├─ cpu_probes
 ├─ hardware_probes
 ├─ hardware_sensors
 └─ interfaces
```

## 収集方針

### CPU

- ベンダー別 OID を優先
- 失敗時は標準 MIB をフォールバック
- 診断ページでは候補 OID を表示する

### interface

- ifIndex を走査して一覧化
- ifName / linkStatus / counters を取得

### hardware sensor

- ENTITY-SENSOR-MIB を優先
- ENTITY-MIB で物理部品名を補完
- 診断では候補と実測値を分けて表示

## 候補の扱い

- `probe`: 実際に試した OID
- `status`: ok / n/a / err
- `selected`: 採用された候補

## エラー処理

- 応答なしは N/A
- OID 不一致は候補落ち
- 部分取得成功時は取得済み項目のみ反映

## 将来拡張

- vendor-specific OID の追加
- sensor unit の MIB 由来化
- 新しい機器種別の判定追加
