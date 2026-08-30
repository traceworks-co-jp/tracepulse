# 07. Hardware Status 詳細設計（Phase 2）

## 目的

温度・電源・ファンなどのハードウェア状態を扱う。

## 設計方針

- Device Detail は日常確認向け
- Diagnostics は根拠情報向け
- Device Detail にはネットワークエンジニアが即時判断できる項目だけ出す

## 表示対象

- temperature
- power
- fan

## 主要関数

- `SnmpClient::query_hardware_sensors`
- `SnmpClient::diagnose_device`
- `page_device_detail`
- `page_diagnostics`
- `renderHardwareStatus`
- `render_hardware_probes`
- `render_hardware_sensors`

## Device Detail 表示ルール

- 温度 / 電源 / ファンのみ表示
- 数値は短く表示
- ユーザー向け説明文は出さない

## Diagnostics 表示ルール

- hardware candidates を表示
- hardware sensors を表示
- OID / source / value / status を確認できる

## 単位

- temperature: °C
- fan: rpm
- power: V
- battery: %

## 図: 取得と表示の分離

```text
Diagnostics
 ├─ raw OID / source / status
 └─ hardware candidates / hardware sensors

Device Detail
 └─ temperature / power / fan のみ
```

## 図: 表示シーケンス

```text
SnmpClient::query_hardware_sensors
 -> hardware_sensors
 -> Device Detail renderHardwareStatus
 -> name + numeric value (+ unit)

SnmpClient::diagnose_device
 -> hardware_probes
 -> hardware_sensors
 -> Diagnostics tables
```

## 表示しないもの

- port
- module
- chassis の細かい内部コード
- OID / source / raw class

## Diagnostics で出すもの

- hardware candidates
- hardware sensors
- OID
- source
- status

## 将来拡張

- ベンダー別の温度・電源・ファン対応
- sensor unit の MIB 由来化
- 異常時の alert 連携
