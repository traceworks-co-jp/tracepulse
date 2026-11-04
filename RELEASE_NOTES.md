TracePulse is portable SNMP network monitoring for network teams and homelabs,
with an embedded Web UI and SQLite storage.

## Highlights

- **Multi-vendor monitoring:** Bundled RFC and vendor templates for Cisco,
  Yamaha, Juniper, Fortinet, MikroTik, Arista, Palo Alto, Ubiquiti,
  HPE ProCurve AOS-S, Aruba AOS-CX, and Allied Telesis devices.
- **Protected SNMP credentials:** Per-device SNMPv2c/SNMPv3 credentials with
  encrypted storage, per-field global fallback, and masked Web UI forms.
- **Live monitoring definitions:** Manage vendor OID definitions in the Web UI
  with SQLite-backed persistence and apply updates to new polling snapshots
  without restarting the service.
- **Topology and port diagnostics:** Discover LLDP/CDP neighbors, inspect
  interface error breakdowns, and run active network diagnostics.
- **Optical monitoring:** Collect supported transceiver Tx/Rx optical power
  readings and visually flag readings at or below -20 dBm.
- **Flow analytics:** Ingest NetFlow v5/v9, IPFIX, and sFlow v5 for protocol,
  application, endpoint, and top-talker summaries.

## Quick Start

Run the Web UI with Docker:

```bash
docker run -d --name tracepulse -p 8080:8080 \
  -v tracepulse-data:/data \
  ghcr.io/traceworks-co-jp/tracepulse:latest
```

Then open <http://localhost:8080>. See the [README](README.md) for native
installation, configuration, supported devices, and security guidance.