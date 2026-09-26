"use strict";
var TracePulseI18n = (() => {
  var __defProp = Object.defineProperty;
  var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __hasOwnProp = Object.prototype.hasOwnProperty;
  var __export = (target, all) => {
    for (var name in all)
      __defProp(target, name, { get: all[name], enumerable: true });
  };
  var __copyProps = (to, from, except, desc) => {
    if (from && typeof from === "object" || typeof from === "function") {
      for (let key of __getOwnPropNames(from))
        if (!__hasOwnProp.call(to, key) && key !== except)
          __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
    }
    return to;
  };
  var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

  // frontend/src/shared/i18n.ts
  var i18n_exports = {};
  __export(i18n_exports, {
    applyLanguage: () => applyLanguage,
    applyTheme: () => applyTheme,
    currentLanguage: () => currentLanguage,
    currentTheme: () => currentTheme,
    currentTimeZoneSetting: () => currentTimeZoneSetting,
    formatTracePulseClock: () => formatTracePulseClock,
    formatTracePulseTimestamp: () => formatTracePulseTimestamp,
    parseTracePulseTime: () => parseTracePulseTime,
    registerTranslations: () => registerTranslations,
    setLanguage: () => setLanguage,
    setTheme: () => setTheme,
    t: () => t,
    tf: () => tf
  });

  // frontend/src/shared/i18n-data.ts
  var translations = {
    en: {
      nav_dashboard: "Dashboard",
      nav_discovery: "Discovery",
      nav_diagnostics: "Diagnostics",
      nav_settings: "Settings",
      recent_alerts_title: "Recent Alerts",
      theme_label: "Theme",
      theme_dark: "Dark",
      theme_light: "Light",
      language_label: "Language",
      english: "English",
      japanese: "Japanese",
      dashboard_title: "Dashboard",
      discovery_title: "Device Discovery",
      diagnostics_title: "SNMP Diagnostics",
      settings_title: "Settings",
      device_title_prefix: "Device",
      interface_status: "Interface Status",
      interface_filter_title: "Interface Selection",
      select_all_interfaces: "Select All",
      clear_all_interfaces: "Clear All",
      bandwidth_title: "Bandwidth Utilization (%)",
      bandwidth_line1: "Bandwidth",
      bandwidth_line2: "Utilization (%)",
      error_discard_title: "Error & Discard Counters",
      cpu_memory_title: "CPU & Memory Usage (%)",
      cpu_usage_title: "CPU Usage (%)",
      memory_used_title: "Memory Used",
      memory_usage_title: "Memory Usage (%)",
      recent_spikes_title: "Recent Interface Spikes",
      alert_history: "Alert History",
      polling: "Polling",
      snmp: "SNMP",
      display: "Display",
      time_zone: "Time Zone",
      time_zone_hint: "Controls how timestamps are shown in the web UI",
      timezone_utc: "UTC",
      timezone_jst: "JST",
      alert_thresholds: "Alert Thresholds",
      data_retention: "Data Retention",
      polling_interval_label: "Polling Interval (seconds)",
      polling_interval_hint: "Range: 5 \u2013 600 seconds (default: 30)",
      default_community: "Default Community String",
      public_placeholder: "public",
      default_community_hint: "Used when no community is specified per device",
      error_rate_threshold: "Error Rate Threshold (0.0 \u2013 1.0)",
      error_rate_hint: "e.g. 0.05 = alert when error rate exceeds 5%",
      spike_threshold: "Spike Threshold (error count delta)",
      spike_threshold_hint: "Alert when error count jumps by this amount between polls",
      warning_threshold: "Health Score \u2014 Warning threshold (0 \u2013 100)",
      warning_threshold_hint: "Score below this value turns status yellow (Warning)",
      critical_threshold: "Health Score \u2014 Critical threshold (0 \u2013 100)",
      critical_threshold_hint: "Score below this value turns status red (Critical)",
      score_preview: "Score preview",
      score_formula: "Health score (0\u2013100) = 100 \u2212 error_rate\xD71.5 \u2212 bandwidth\xD70.3 \u2212 cpu/3 \u2212 memory/3",
      history_retention: "History Retention (days)",
      history_retention_hint: "Polling history older than this is purged automatically",
      save_settings: "Save Settings",
      reset_defaults: "Reset to Defaults",
      settings_saved: "Settings saved",
      notifications: "Alert Notifications (Slack / Teams)",
      slack_enabled: "Enable Slack notifications",
      teams_enabled: "Enable Teams notifications",
      webhook_url_hint: "Leave empty to fall back to the environment variable below. ${ENV_NAME} placeholders are expanded.",
      webhook_env_hint: "Environment variable used when no URL is set",
      flap_window: "Flap Guard Window (seconds)",
      flap_window_hint: "Alerts of the same kind are aggregated within this window",
      retry_max_attempts: "Retry Attempts",
      retry_max_attempts_hint: "Number of webhook delivery attempts (1 \u2013 10)",
      save_notifications: "Save Notification Settings",
      send_test: "Send test notification",
      send_test_all: "Test all enabled channels",
      notifications_saved: "Notification settings saved",
      sending_test: "Sending test notification\u2026",
      test_sent: "Test notification sent",
      device_discovery: "Device Discovery",
      cidr_range: "CIDR Range",
      cidr_range_placeholder: "192.168.1.0/24",
      community: "Community",
      scan: "Scan",
      cancel: "Cancel",
      add_manually: "+ Add manually",
      add_device_manually: "Add Device Manually",
      ip_address: "IP Address",
      ip_address_placeholder: "192.168.1.1",
      name: "Name",
      name_placeholder: "router-01",
      add: "Add",
      registration: "Registration",
      hostname: "Hostname",
      status: "Status",
      in_errors: "In Errors",
      out_errors: "Out Errors",
      in_discards: "In Discards",
      out_discards: "Out Discards",
      late_collisions: "Late Collisions",
      diagnostic_status: "Diagnostic Status",
      diagnostic_healthy: "Healthy",
      diagnostic_l1_error: "\u26A0 L1 Error",
      diagnostic_duplex_mismatch: "\u26A0 Duplex Mismatch",
      diagnostic_congestion: "\u26A0 Congestion",
      diagnostic_down: "Down",
      registered: "Registered",
      select_all: "Select all",
      scanning: "Scanning\u2026",
      elapsed: "Elapsed",
      found: "found",
      scan_results_label: "Scan Results",
      register: "Register",
      selected: "selected",
      time: "Time",
      type: "Type",
      severity: "Severity",
      details: "Details",
      interface: "Interface",
      online: "Online",
      warning: "Warning",
      offline: "Offline",
      total: "Total",
      error_spikes: "Error Spikes",
      attention_devices: "Devices needing attention",
      registered_devices: "Registered Devices",
      updated: "Updated",
      no_data: "No data yet",
      no_devices: "No devices registered yet.",
      no_interface_data: "No interface data collected yet",
      no_interface_spikes: "No interface spikes recorded",
      no_alerts: "No alerts recorded",
      actions: "Actions",
      scan_results: "Scan Results",
      run_diagnostics: "Run Diagnostics",
      diagnostics_running: "Diagnosing\u2026",
      diagnostics_help: "Check sysObjectID, vendor presets, CPU / memory candidates, and ifIndex mappings.",
      target_device: "Target Device",
      oid_overrides: "OID Overrides",
      cpu_oid_override: "CPU OID Override",
      memory_oid_override: "Memory OID Override",
      hardware_oid_temperature: "Temperature OID",
      hardware_oid_power: "Power OID",
      hardware_oid_fan: "Fan OID",
      save_oid_overrides: "Save OID Overrides",
      unregister: "Unregister",
      unregistering: "Unregistering\u2026",
      unregister_confirm: "Remove this device from TracePulse?",
      unregister_success: "Device unregistered",
      unregister_failed: "Failed to unregister device",
      cpu_candidates: "CPU Candidates",
      hardware_candidates: "Hardware Candidates",
      hardware_sensors: "Hardware Sensors",
      hardware_status_title: "Hardware Status",
      memory_candidates: "Memory Candidates",
      if_index_list: "ifIndex List",
      sys_object_id: "sysObjectID",
      enterprise_id: "Enterprise ID",
      vendor: "Vendor",
      probe: "Probe",
      value: "Value",
      link_status: "Link",
      no_snmp_devices_found: "No SNMP-responding devices found",
      registering: "Registering\u2026",
      select_at_least_one_device: "Select at least one device.",
      request_failed: "Request failed",
      register_selected: "Register Selected",
      seed_device_ip: "Seed Device IP (LLDP/CDP)",
      draw_topology: "Draw Topology",
      topology_map: "Topology Map",
      topology_zoom_in: "Zoom in",
      topology_zoom_out: "Zoom out",
      topology_fit: "Fit",
      topology_auto_layout: "Auto layout",
      topology_export_json: "Export JSON",
      topology_export_csv: "Export CSV",
      topology_details: "Details",
      topology_close: "Close",
      topology_empty_hint: "Enter a seed device IP and run discovery to draw the network map.",
      topology_running: "Topology discovery running from {ip}\u2026",
      topology_no_neighbors: "No LLDP/CDP neighbors found from {ip}",
      topology_summary: "{nodes} nodes / {edges} links discovered from {ip}",
      topology_seed_required: "Seed device IP is required to draw the topology map.",
      topology_export_empty: "Run topology discovery before exporting.",
      topology_protocol: "Protocol",
      topology_local_side: "Local side",
      topology_remote_side: "Remote side",
      topology_device: "Device",
      topology_port: "Port",
      topology_link: "Link",
      topology_interfaces: "Interfaces",
      topology_no_interface_data: "No interface data",
      node_type_switch: "Switch / Router",
      node_type_endpoint: "Endpoint",
      edition_community: "Community Edition ({limit} Node Limit)",
      port_health: "Port Health",
      port_health_crc: "CRC Errors",
      port_health_late_collisions: "Late Collisions",
      port_health_discards: "Discards",
      error_breakdown_title: "Error Breakdown",
      error_breakdown_select_hint: "Select an interface to inspect corrupted-packet breakdown.",
      error_breakdown_no_data: "No error breakdown data collected yet.",
      error_breakdown_fcs_errors: "FCS/CRC Errors",
      error_breakdown_alignment_errors: "Alignment Errors",
      error_breakdown_frame_too_longs: "Oversized Frames",
      error_breakdown_internal_mac_receive_errors: "MAC Receive Errors",
      error_breakdown_hover_title: "Error Breakdown (since previous poll)",
      traffic_protocols_title: "Traffic & Protocols",
      traffic_protocols_empty: "No flow records received yet.",
      traffic_protocols_note: "Values are estimates based on received flows; device-side sampling is not corrected.",
      device_auto_refresh: "Device data auto-refresh: {seconds}s",
      traffic_protocols_auto_refresh: "Traffic analytics auto-refresh: {seconds}s",
      traffic_protocols_refresh_interval: "Refresh interval",
      traffic_protocols_window: "Window",
      traffic_protocols_live: "Live (5s)",
      traffic_protocols_last_5m: "Last 5m",
      traffic_protocols_last_1h: "Last 1h",
      traffic_protocols_last_24h: "Last 24h",
      traffic_protocols_pause: "Pause",
      traffic_protocols_resume: "Resume",
      traffic_protocols_summary: "Summary",
      traffic_protocols_total_bps: "Total Bps",
      traffic_protocols_packet_rate: "Packet Rate",
      traffic_protocols_active_flows: "Active Flows",
      traffic_protocols_top_protocol: "Top Protocol",
      traffic_protocols_timeseries: "Traffic Timeseries",
      traffic_protocols_time: "Time",
      traffic_protocols_applications: "Applications",
      traffic_protocols_top_sources: "Top Sources",
      traffic_protocols_top_destinations: "Top Destinations",
      traffic_protocols_source: "Source",
      traffic_protocols_destination: "Destination",
      traffic_protocols_protocol: "Protocol",
      traffic_protocols_application: "Application",
      traffic_protocols_tcp_flags: "TCP Flags",
      traffic_protocols_in_if: "In If",
      traffic_protocols_out_if: "Out If",
      traffic_protocols_share: "Protocol Share",
      traffic_protocols_top_talkers: "Top Talkers"
    },
    ja: {
      nav_dashboard: "\u30C0\u30C3\u30B7\u30E5\u30DC\u30FC\u30C9",
      nav_discovery: "\u63A2\u7D22",
      nav_diagnostics: "\u8A3A\u65AD",
      recent_alerts_title: "\u6700\u8FD1\u306E\u30A2\u30E9\u30FC\u30C8",
      nav_settings: "\u8A2D\u5B9A",
      theme_label: "\u30C6\u30FC\u30DE",
      theme_dark: "\u30C0\u30FC\u30AF",
      theme_light: "\u30E9\u30A4\u30C8",
      language_label: "Language",
      english: "\u82F1\u8A9E",
      japanese: "\u65E5\u672C\u8A9E",
      dashboard_title: "\u30C0\u30C3\u30B7\u30E5\u30DC\u30FC\u30C9",
      discovery_title: "\u30C7\u30D0\u30A4\u30B9\u63A2\u7D22",
      diagnostics_title: "SNMP \u8A3A\u65AD",
      settings_title: "\u8A2D\u5B9A",
      device_title_prefix: "\u6A5F\u5668",
      interface_status: "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u72B6\u614B",
      interface_filter_title: "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u9078\u629E",
      select_all_interfaces: "\u5168\u9078\u629E",
      clear_all_interfaces: "\u5168\u89E3\u9664",
      bandwidth_title: "\u5E2F\u57DF\u5229\u7528\u7387 (%)",
      bandwidth_line1: "\u5E2F\u57DF",
      bandwidth_line2: "\u5229\u7528\u7387 (%)",
      error_discard_title: "\u30A8\u30E9\u30FC / \u30C7\u30A3\u30B9\u30AB\u30FC\u30C9",
      cpu_memory_title: "CPU / \u30E1\u30E2\u30EA\u4F7F\u7528\u7387 (%)",
      cpu_usage_title: "CPU \u4F7F\u7528\u7387 (%)",
      memory_used_title: "\u30E1\u30E2\u30EA\u4F7F\u7528\u91CF",
      memory_usage_title: "\u30E1\u30E2\u30EA\u4F7F\u7528\u7387 (%)",
      recent_spikes_title: "\u6700\u8FD1\u306E\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u30B9\u30D1\u30A4\u30AF",
      alert_history: "\u30A2\u30E9\u30FC\u30C8\u5C65\u6B74",
      polling: "\u30DD\u30FC\u30EA\u30F3\u30B0",
      snmp: "SNMP",
      display: "\u8868\u793A",
      time_zone: "\u6642\u523B\u8A2D\u5B9A",
      time_zone_hint: "Web UI \u4E0A\u306E\u6642\u523B\u8868\u793A\u3092\u5207\u308A\u66FF\u3048\u307E\u3059",
      timezone_utc: "UTC",
      timezone_jst: "JST",
      alert_thresholds: "\u30A2\u30E9\u30FC\u30C8\u95BE\u5024",
      data_retention: "\u30C7\u30FC\u30BF\u4FDD\u6301",
      polling_interval_label: "\u30DD\u30FC\u30EA\u30F3\u30B0\u9593\u9694\uFF08\u79D2\uFF09",
      polling_interval_hint: "\u7BC4\u56F2: 5 \uFF5E 600 \u79D2\uFF08\u30C7\u30D5\u30A9\u30EB\u30C8: 30\uFF09",
      default_community: "\u30C7\u30D5\u30A9\u30EB\u30C8\u30B3\u30DF\u30E5\u30CB\u30C6\u30A3\u6587\u5B57\u5217",
      public_placeholder: "public",
      default_community_hint: "\u5404\u30C7\u30D0\u30A4\u30B9\u3067 community \u672A\u6307\u5B9A\u6642\u306B\u4F7F\u7528",
      error_rate_threshold: "\u30A8\u30E9\u30FC\u7387\u95BE\u5024 (0.0 \u2013 1.0)",
      error_rate_hint: "\u4F8B: 0.05 = \u30A8\u30E9\u30FC\u7387\u304C 5% \u3092\u8D85\u3048\u305F\u3089\u901A\u77E5",
      spike_threshold: "\u30B9\u30D1\u30A4\u30AF\u95BE\u5024\uFF08\u30A8\u30E9\u30FC\u30AB\u30A6\u30F3\u30C8\u5DEE\u5206\uFF09",
      spike_threshold_hint: "\u30DD\u30FC\u30EA\u30F3\u30B0\u9593\u306E\u30A8\u30E9\u30FC\u30AB\u30A6\u30F3\u30C8\u5897\u5206\u304C\u3053\u306E\u5024\u3092\u8D85\u3048\u308B\u3068\u901A\u77E5",
      warning_threshold: "\u30D8\u30EB\u30B9\u30B9\u30B3\u30A2 \u2014 Warning \u95BE\u5024\uFF080 \u2013 100\uFF09",
      warning_threshold_hint: "\u3053\u306E\u5024\u3092\u4E0B\u56DE\u308B\u3068\u9EC4\u8272 (Warning)",
      critical_threshold: "\u30D8\u30EB\u30B9\u30B9\u30B3\u30A2 \u2014 Critical \u95BE\u5024\uFF080 \u2013 100\uFF09",
      critical_threshold_hint: "\u3053\u306E\u5024\u3092\u4E0B\u56DE\u308B\u3068\u8D64\u8272 (Critical)",
      score_preview: "\u30B9\u30B3\u30A2\u8868\u793A",
      score_formula: "\u30D8\u30EB\u30B9\u30B9\u30B3\u30A2 (0\u2013100) = 100 \u2212 error_rate\xD71.5 \u2212 bandwidth\xD70.3 \u2212 cpu/3 \u2212 memory/3",
      history_retention: "\u5C65\u6B74\u4FDD\u6301\u671F\u9593\uFF08\u65E5\uFF09",
      history_retention_hint: "\u6307\u5B9A\u65E5\u6570\u3088\u308A\u53E4\u3044\u30DD\u30FC\u30EA\u30F3\u30B0\u5C65\u6B74\u306F\u81EA\u52D5\u524A\u9664",
      save_settings: "\u8A2D\u5B9A\u3092\u4FDD\u5B58",
      reset_defaults: "\u30C7\u30D5\u30A9\u30EB\u30C8\u306B\u623B\u3059",
      settings_saved: "\u8A2D\u5B9A\u3092\u4FDD\u5B58\u3057\u307E\u3057\u305F",
      notifications: "\u30A2\u30E9\u30FC\u30C8\u901A\u77E5\uFF08Slack / Teams\uFF09",
      slack_enabled: "Slack \u901A\u77E5\u3092\u6709\u52B9\u306B\u3059\u308B",
      teams_enabled: "Teams \u901A\u77E5\u3092\u6709\u52B9\u306B\u3059\u308B",
      webhook_url_hint: "\u7A7A\u306E\u5834\u5408\u306F\u4E0B\u306E\u74B0\u5883\u5909\u6570\u304B\u3089\u53D6\u5F97\u3057\u307E\u3059\u3002${\u74B0\u5883\u5909\u6570\u540D} \u5F62\u5F0F\u306E\u5C55\u958B\u306B\u3082\u5BFE\u5FDC\u3057\u307E\u3059\u3002",
      webhook_env_hint: "URL \u672A\u8A2D\u5B9A\u6642\u306B\u53C2\u7167\u3059\u308B\u74B0\u5883\u5909\u6570\u540D",
      flap_window: "\u30D5\u30E9\u30C3\u30D7\u6291\u5236\u30A6\u30A3\u30F3\u30C9\u30A6\uFF08\u79D2\uFF09",
      flap_window_hint: "\u540C\u7A2E\u306E\u30A2\u30E9\u30FC\u30C8\u3092\u3053\u306E\u671F\u9593\u5185\u3067\u96C6\u7D04\u3057\u3066\u901A\u77E5\u3057\u307E\u3059",
      retry_max_attempts: "\u30EA\u30C8\u30E9\u30A4\u56DE\u6570",
      retry_max_attempts_hint: "Webhook \u9001\u4FE1\u306E\u8A66\u884C\u56DE\u6570\uFF081\u301C10\uFF09",
      save_notifications: "\u901A\u77E5\u8A2D\u5B9A\u3092\u4FDD\u5B58",
      send_test: "\u30C6\u30B9\u30C8\u901A\u77E5\u3092\u9001\u4FE1",
      send_test_all: "\u6709\u52B9\u306A\u30C1\u30E3\u30F3\u30CD\u30EB\u3092\u4E00\u62EC\u30C6\u30B9\u30C8",
      notifications_saved: "\u901A\u77E5\u8A2D\u5B9A\u3092\u4FDD\u5B58\u3057\u307E\u3057\u305F",
      sending_test: "\u30C6\u30B9\u30C8\u901A\u77E5\u3092\u9001\u4FE1\u4E2D\u2026",
      test_sent: "\u30C6\u30B9\u30C8\u901A\u77E5\u3092\u9001\u4FE1\u3057\u307E\u3057\u305F",
      device_discovery: "\u30C7\u30D0\u30A4\u30B9\u63A2\u7D22",
      cidr_range: "CIDR \u7BC4\u56F2",
      cidr_range_placeholder: "192.168.1.0/24",
      error_breakdown_title: "\u30A8\u30E9\u30FC\u8A73\u7D30\uFF08Error Breakdown\uFF09",
      error_breakdown_select_hint: "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u3092\u9078\u629E\u3059\u308B\u3068\u7834\u640D\u30D1\u30B1\u30C3\u30C8\u306E\u5185\u8A33\u3092\u8868\u793A\u3057\u307E\u3059\u3002",
      error_breakdown_no_data: "\u307E\u3060\u30A8\u30E9\u30FC\u5185\u8A33\u30C7\u30FC\u30BF\u304C\u53CE\u96C6\u3055\u308C\u3066\u3044\u307E\u305B\u3093\u3002",
      error_breakdown_fcs_errors: "FCS/CRC \u30A8\u30E9\u30FC",
      error_breakdown_alignment_errors: "\u30A2\u30E9\u30A4\u30E1\u30F3\u30C8\u30A8\u30E9\u30FC",
      error_breakdown_frame_too_longs: "\u30D5\u30EC\u30FC\u30E0\u8D85\u904E\uFF08\u30B8\u30E3\u30A4\u30A2\u30F3\u30C8\uFF09",
      error_breakdown_internal_mac_receive_errors: "MAC \u5C64\u53D7\u4FE1\u30A8\u30E9\u30FC",
      error_breakdown_hover_title: "\u30A8\u30E9\u30FC\u8A73\u7D30\uFF08\u76F4\u8FD1\u30DD\u30FC\u30EA\u30F3\u30B0\u5DEE\u5206\uFF09",
      community: "\u30B3\u30DF\u30E5\u30CB\u30C6\u30A3",
      scan: "\u30B9\u30AD\u30E3\u30F3",
      cancel: "\u30AD\u30E3\u30F3\u30BB\u30EB",
      add_manually: "+ \u624B\u52D5\u8FFD\u52A0",
      add_device_manually: "\u30C7\u30D0\u30A4\u30B9\u3092\u624B\u52D5\u8FFD\u52A0",
      ip_address: "IP \u30A2\u30C9\u30EC\u30B9",
      ip_address_placeholder: "192.168.1.1",
      name: "\u540D\u524D",
      name_placeholder: "router-01",
      add: "\u8FFD\u52A0",
      registration: "\u767B\u9332",
      hostname: "\u30DB\u30B9\u30C8\u540D",
      status: "\u72B6\u614B",
      in_errors: "\u5165\u529B\u30A8\u30E9\u30FC",
      out_errors: "\u51FA\u529B\u30A8\u30E9\u30FC",
      in_discards: "\u5165\u529B\u30C7\u30A3\u30B9\u30AB\u30FC\u30C9",
      out_discards: "\u51FA\u529B\u30C7\u30A3\u30B9\u30AB\u30FC\u30C9",
      late_collisions: "\u9045\u5EF6\u885D\u7A81",
      diagnostic_status: "\u8A3A\u65AD\u30B9\u30C6\u30FC\u30BF\u30B9",
      diagnostic_healthy: "\u6B63\u5E38",
      diagnostic_l1_error: "\u26A0 L1\u30A8\u30E9\u30FC",
      diagnostic_duplex_mismatch: "\u26A0 Duplex\u4E0D\u6574\u5408",
      diagnostic_congestion: "\u26A0 \u5E2F\u57DF\u903C\u8FEB",
      diagnostic_down: "Down",
      registered: "\u767B\u9332\u6E08\u307F",
      select_all: "\u5168\u9078\u629E",
      scanning: "\u30B9\u30AD\u30E3\u30F3\u4E2D\u2026",
      elapsed: "\u7D4C\u904E",
      found: "\u4EF6\u691C\u51FA",
      scan_results_label: "\u30B9\u30AD\u30E3\u30F3\u7D50\u679C",
      register: "\u767B\u9332",
      selected: "\u9078\u629E",
      time: "\u6642\u523B",
      type: "\u7A2E\u5225",
      severity: "\u91CD\u8981\u5EA6",
      details: "\u8A73\u7D30",
      interface: "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9",
      online: "Online",
      warning: "Warning",
      offline: "Offline",
      total: "\u5408\u8A08",
      error_spikes: "\u30A8\u30E9\u30FC\u6025\u5897",
      attention_devices: "\u7570\u5E38\u30FB\u8B66\u544A\u306E\u3042\u308B\u7AEF\u672B",
      registered_devices: "\u767B\u9332\u7AEF\u672B",
      updated: "\u66F4\u65B0",
      no_data: "\u30C7\u30FC\u30BF\u306F\u307E\u3060\u3042\u308A\u307E\u305B\u3093",
      no_devices: "\u307E\u3060\u30C7\u30D0\u30A4\u30B9\u304C\u767B\u9332\u3055\u308C\u3066\u3044\u307E\u305B\u3093\u3002",
      no_interface_data: "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u30C7\u30FC\u30BF\u306F\u307E\u3060\u53CE\u96C6\u3055\u308C\u3066\u3044\u307E\u305B\u3093",
      no_interface_spikes: "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u30B9\u30D1\u30A4\u30AF\u306F\u307E\u3060\u8A18\u9332\u3055\u308C\u3066\u3044\u307E\u305B\u3093",
      no_alerts: "\u30A2\u30E9\u30FC\u30C8\u306F\u307E\u3060\u3042\u308A\u307E\u305B\u3093",
      actions: "\u64CD\u4F5C",
      scan_results: "\u30B9\u30AD\u30E3\u30F3\u7D50\u679C",
      run_diagnostics: "\u8A3A\u65AD\u3092\u5B9F\u884C",
      diagnostics_running: "\u8A3A\u65AD\u4E2D\u2026",
      diagnostics_help: "sysObjectID\u3001\u30D9\u30F3\u30C0\u30FC\u5019\u88DC\u3001CPU / \u30E1\u30E2\u30EA\u5019\u88DC\u3001ifIndex \u30DE\u30C3\u30D4\u30F3\u30B0\u3092\u78BA\u8A8D\u3057\u307E\u3059\u3002",
      target_device: "\u5BFE\u8C61\u30C7\u30D0\u30A4\u30B9",
      oid_overrides: "OID \u4E0A\u66F8\u304D",
      cpu_oid_override: "CPU OID \u4E0A\u66F8\u304D",
      memory_oid_override: "\u30E1\u30E2\u30EA OID \u4E0A\u66F8\u304D",
      hardware_oid_temperature: "\u6E29\u5EA6 OID \u4E0A\u66F8\u304D",
      hardware_oid_power: "\u96FB\u6E90 OID \u4E0A\u66F8\u304D",
      hardware_oid_fan: "\u30D5\u30A1\u30F3 OID \u4E0A\u66F8\u304D",
      save_oid_overrides: "OID \u306E\u624B\u52D5\u4FDD\u5B58",
      unregister: "\u767B\u9332\u89E3\u9664",
      unregistering: "\u89E3\u9664\u4E2D\u2026",
      unregister_confirm: "\u3053\u306E\u30C7\u30D0\u30A4\u30B9\u3092 TracePulse \u304B\u3089\u524A\u9664\u3057\u307E\u3059\u304B\uFF1F",
      unregister_success: "\u30C7\u30D0\u30A4\u30B9\u3092\u524A\u9664\u3057\u307E\u3057\u305F",
      unregister_failed: "\u30C7\u30D0\u30A4\u30B9\u306E\u524A\u9664\u306B\u5931\u6557\u3057\u307E\u3057\u305F",
      cpu_candidates: "CPU \u5019\u88DC",
      hardware_candidates: "Hardware \u5019\u88DC",
      hardware_sensors: "Hardware \u30BB\u30F3\u30B5\u30FC",
      hardware_status_title: "Hardware \u30B9\u30C6\u30FC\u30BF\u30B9",
      memory_candidates: "\u30E1\u30E2\u30EA\u5019\u88DC",
      if_index_list: "ifIndex \u4E00\u89A7",
      sys_object_id: "sysObjectID",
      enterprise_id: "Enterprise ID",
      vendor: "\u30D9\u30F3\u30C0\u30FC",
      probe: "\u5019\u88DC",
      value: "\u5024",
      link_status: "\u30EA\u30F3\u30AF",
      no_snmp_devices_found: "SNMP \u5FDC\u7B54\u306E\u3042\u308B\u30C7\u30D0\u30A4\u30B9\u306F\u898B\u3064\u304B\u308A\u307E\u305B\u3093\u3067\u3057\u305F",
      registering: "\u767B\u9332\u4E2D\u2026",
      select_at_least_one_device: "\u5C11\u306A\u304F\u3068\u30821\u53F0\u306E\u6A5F\u5668\u3092\u9078\u629E\u3057\u3066\u304F\u3060\u3055\u3044\u3002",
      request_failed: "\u30EA\u30AF\u30A8\u30B9\u30C8\u306B\u5931\u6557\u3057\u307E\u3057\u305F",
      register_selected: "\u9078\u629E\u3057\u305F\u6A5F\u5668\u3092\u767B\u9332",
      seed_device_ip: "\u30B7\u30FC\u30C9\u6A5F\u5668 IP (LLDP/CDP)",
      draw_topology: "\u30C8\u30DD\u30ED\u30B8\u30FC\u63CF\u753B",
      topology_map: "\u30C8\u30DD\u30ED\u30B8\u30FC\u30DE\u30C3\u30D7",
      topology_zoom_in: "\u62E1\u5927",
      topology_zoom_out: "\u7E2E\u5C0F",
      topology_fit: "\u5168\u4F53\u8868\u793A",
      topology_auto_layout: "\u81EA\u52D5\u6574\u5217",
      topology_export_json: "JSON \u3067\u51FA\u529B",
      topology_export_csv: "CSV \u3067\u51FA\u529B",
      topology_details: "\u8A73\u7D30",
      topology_close: "\u9589\u3058\u308B",
      topology_empty_hint: "\u30B7\u30FC\u30C9\u6A5F\u5668\u306E IP \u3092\u5165\u529B\u3057\u3066\u63A2\u7D22\u3092\u5B9F\u884C\u3059\u308B\u3068\u69CB\u6210\u56F3\u3092\u63CF\u753B\u3057\u307E\u3059\u3002",
      topology_running: "{ip} \u3092\u8D77\u70B9\u306B\u30C8\u30DD\u30ED\u30B8\u30FC\u3092\u63A2\u7D22\u4E2D\u2026",
      topology_no_neighbors: "{ip} \u304B\u3089 LLDP/CDP \u96A3\u63A5\u6A5F\u5668\u306F\u691C\u51FA\u3055\u308C\u307E\u305B\u3093\u3067\u3057\u305F",
      topology_summary: "{ip} \u304B\u3089 {nodes} \u30CE\u30FC\u30C9 / {edges} \u30EA\u30F3\u30AF\u3092\u691C\u51FA\u3057\u307E\u3057\u305F",
      topology_seed_required: "\u30C8\u30DD\u30ED\u30B8\u30FC\u63CF\u753B\u306B\u306F\u30B7\u30FC\u30C9\u6A5F\u5668\u306E IP \u304C\u5FC5\u8981\u3067\u3059\u3002",
      topology_export_empty: "\u30A8\u30AF\u30B9\u30DD\u30FC\u30C8\u524D\u306B\u30C8\u30DD\u30ED\u30B8\u30FC\u63A2\u7D22\u3092\u5B9F\u884C\u3057\u3066\u304F\u3060\u3055\u3044\u3002",
      topology_protocol: "\u30D7\u30ED\u30C8\u30B3\u30EB",
      topology_local_side: "\u63A5\u7D9A\u5143",
      topology_remote_side: "\u63A5\u7D9A\u5148",
      topology_device: "\u6A5F\u5668\u540D",
      topology_port: "\u30DD\u30FC\u30C8",
      topology_link: "\u30EA\u30F3\u30AF",
      topology_interfaces: "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9",
      topology_no_interface_data: "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u60C5\u5831\u306F\u3042\u308A\u307E\u305B\u3093",
      node_type_switch: "\u30B9\u30A4\u30C3\u30C1 / \u30EB\u30FC\u30BF\u30FC",
      node_type_endpoint: "\u7AEF\u672B\u30FB\u305D\u306E\u4ED6",
      edition_community: "Community Edition\uFF08\u4E0A\u9650 {limit} \u30CE\u30FC\u30C9\uFF09",
      port_health: "\u30DD\u30FC\u30C8\u5065\u5168\u6027",
      port_health_crc: "CRC \u30A8\u30E9\u30FC",
      port_health_late_collisions: "Late Collision",
      port_health_discards: "\u30C7\u30A3\u30B9\u30AB\u30FC\u30C9",
      traffic_protocols_title: "\u30C8\u30E9\u30D5\u30A3\u30C3\u30AF\u3068\u30D7\u30ED\u30C8\u30B3\u30EB",
      traffic_protocols_empty: "\u30D5\u30ED\u30FC\u30C7\u30FC\u30BF\u3092\u307E\u3060\u53D7\u4FE1\u3057\u3066\u3044\u307E\u305B\u3093\u3002",
      traffic_protocols_note: "\u203B\u8868\u793A\u5024\u306F\u53D7\u4FE1\u30D5\u30ED\u30FC\u306B\u57FA\u3065\u304F\u6982\u7B97\u5024\u3067\u3059\u3002\u6A5F\u5668\u5074\u306E\u30B5\u30F3\u30D7\u30EA\u30F3\u30B0\u8A2D\u5B9A\u7B49\u306B\u3088\u308B\u88DC\u6B63\u306F\u542B\u307E\u308C\u307E\u305B\u3093\u3002",
      device_auto_refresh: "\u6A5F\u5668\u60C5\u5831\u306E\u81EA\u52D5\u66F4\u65B0: {seconds}\u79D2",
      traffic_protocols_auto_refresh: "\u30C8\u30E9\u30D5\u30A3\u30C3\u30AF\u5206\u6790\u306E\u81EA\u52D5\u66F4\u65B0: {seconds}\u79D2",
      traffic_protocols_refresh_interval: "\u66F4\u65B0\u9593\u9694",
      traffic_protocols_window: "\u671F\u9593",
      traffic_protocols_live: "\u30E9\u30A4\u30D6\uFF085\u79D2\uFF09",
      traffic_protocols_last_5m: "\u76F4\u8FD15\u5206",
      traffic_protocols_last_1h: "\u76F4\u8FD11\u6642\u9593",
      traffic_protocols_last_24h: "\u76F4\u8FD124\u6642\u9593",
      traffic_protocols_pause: "\u4E00\u6642\u505C\u6B62",
      traffic_protocols_resume: "\u518D\u958B",
      traffic_protocols_summary: "\u30B5\u30DE\u30EA\u30FC",
      traffic_protocols_total_bps: "\u5408\u8A08Bps",
      traffic_protocols_packet_rate: "\u30D1\u30B1\u30C3\u30C8\u30EC\u30FC\u30C8",
      traffic_protocols_active_flows: "\u30A2\u30AF\u30C6\u30A3\u30D6\u30D5\u30ED\u30FC",
      traffic_protocols_top_protocol: "\u6700\u591A\u30D7\u30ED\u30C8\u30B3\u30EB",
      traffic_protocols_timeseries: "\u30C8\u30E9\u30D5\u30A3\u30C3\u30AF\u6642\u7CFB\u5217",
      traffic_protocols_time: "\u6642\u523B",
      traffic_protocols_applications: "\u30A2\u30D7\u30EA\u30B1\u30FC\u30B7\u30E7\u30F3",
      traffic_protocols_top_sources: "\u4E0A\u4F4D\u9001\u4FE1\u5143",
      traffic_protocols_top_destinations: "\u4E0A\u4F4D\u5B9B\u5148",
      traffic_protocols_source: "\u9001\u4FE1\u5143",
      traffic_protocols_destination: "\u5B9B\u5148",
      traffic_protocols_protocol: "\u30D7\u30ED\u30C8\u30B3\u30EB",
      traffic_protocols_application: "\u30A2\u30D7\u30EA\u30B1\u30FC\u30B7\u30E7\u30F3",
      traffic_protocols_tcp_flags: "TCP\u30D5\u30E9\u30B0",
      traffic_protocols_in_if: "\u5165\u529BIF",
      traffic_protocols_out_if: "\u51FA\u529BIF",
      traffic_protocols_share: "\u30D7\u30ED\u30C8\u30B3\u30EB\u69CB\u6210\u6BD4",
      traffic_protocols_top_talkers: "Top Talkers"
    }
  };

  // frontend/src/shared/i18n.ts
  function translationData() {
    return translations;
  }
  function registerTranslations(extension) {
    const data = translations;
    Object.entries(extension).forEach(([language, entries]) => {
      if (!entries) return;
      Object.assign(data[language] || (data[language] = {}), entries);
    });
  }
  function t(key, lang = currentLanguage()) {
    const data = translationData();
    return data[lang]?.[key] || data.en?.[key] || key;
  }
  function tf(key, params = {}, lang = currentLanguage()) {
    let text = t(key, lang);
    Object.entries(params).forEach(([name, value]) => {
      text = text.split(`{${name}}`).join(String(value));
    });
    return text;
  }
  function currentLanguage() {
    try {
      return localStorage.getItem("tracepulse-lang") || "en";
    } catch {
      return "en";
    }
  }
  function setLanguage(lang) {
    try {
      localStorage.setItem("tracepulse-lang", lang);
    } catch {
    }
    applyLanguage(lang);
  }
  function currentTheme() {
    try {
      return localStorage.getItem("tracepulse-theme") || "dark";
    } catch {
      return "dark";
    }
  }
  function setTheme(theme) {
    try {
      localStorage.setItem("tracepulse-theme", theme);
    } catch {
    }
    applyTheme(theme);
  }
  function applyTheme(theme = currentTheme()) {
    document.documentElement.dataset.theme = theme;
    const select = document.getElementById("theme-select");
    if (select && select.value !== theme) select.value = theme;
    window.dispatchEvent(new CustomEvent("tracepulse-theme-change", { detail: { theme } }));
  }
  function applyLanguage(lang = currentLanguage()) {
    document.documentElement.lang = lang;
    const languageSelect = document.getElementById("lang-select");
    if (languageSelect && languageSelect.value !== lang) languageSelect.value = lang;
    document.querySelectorAll("[data-i18n]").forEach((element) => {
      element.textContent = t(element.dataset.i18n || "", lang);
    });
    document.querySelectorAll("[data-i18n-placeholder]").forEach((element) => {
      element.placeholder = t(element.dataset.i18nPlaceholder || "", lang);
    });
    document.querySelectorAll("[data-i18n-title]").forEach((element) => {
      element.title = t(element.dataset.i18nTitle || "", lang);
    });
    const titleKey = document.body?.dataset.titleKey;
    if (titleKey) document.title = `TracePulse - ${t(titleKey, lang)}`;
    window.applyPageLanguage?.(lang);
    window.dispatchEvent(new CustomEvent("tracepulse-language-change", { detail: { lang } }));
  }
  function currentTimeZoneSetting() {
    try {
      return localStorage.getItem("tracepulse-timezone") || "utc";
    } catch {
      return "utc";
    }
  }
  function formatTracePulseTimestamp(value) {
    const date = value instanceof Date ? value : parseTracePulseTime(value);
    if (Number.isNaN(date.getTime())) return value ? String(value) : "-";
    const parts = new Intl.DateTimeFormat("en-US", { timeZone: currentTimeZoneSetting() === "jst" ? "Asia/Tokyo" : "UTC", month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false }).formatToParts(date);
    const map = {};
    parts.forEach((part) => {
      if (part.type !== "literal") map[part.type] = part.value;
    });
    return `${map.month}/${map.day} ${map.hour}:${map.minute}:${map.second}`;
  }
  function parseTracePulseTime(value) {
    if (typeof value !== "string" || !value) return /* @__PURE__ */ new Date(NaN);
    if (/Z$|[+-]\d\d:\d\d$/.test(value)) return new Date(value);
    return /* @__PURE__ */ new Date(`${value.replace(" ", "T")}Z`);
  }
  function formatTracePulseClock(value) {
    const date = value instanceof Date ? value : parseTracePulseTime(value);
    if (Number.isNaN(date.getTime())) return "-";
    const parts = new Intl.DateTimeFormat("en-US", { timeZone: currentTimeZoneSetting() === "jst" ? "Asia/Tokyo" : "UTC", hour: "2-digit", minute: "2-digit", hour12: false }).formatToParts(date);
    const map = {};
    parts.forEach((part) => {
      if (part.type !== "literal") map[part.type] = part.value;
    });
    return `${map.hour}:${map.minute}`;
  }
  Object.assign(window, { t, tf, registerTranslations, currentLanguage, setLanguage, currentTheme, setTheme, applyTheme, applyLanguage, currentTimeZoneSetting, formatTracePulseTimestamp, formatTracePulseClock });
  document.addEventListener("DOMContentLoaded", () => {
    applyTheme();
    applyLanguage();
    document.querySelectorAll("[data-timestamp]").forEach((element) => {
      element.textContent = formatTracePulseTimestamp(element.dataset.timestamp);
    });
  });
  return __toCommonJS(i18n_exports);
})();
