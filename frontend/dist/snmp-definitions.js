"use strict";
var TracePulseSnmpDefinitions = (() => {
  // frontend/src/shared/operation-progress.ts
  function showOperationProgress(container, label, message) {
    if (!document.getElementById("operation-progress-styles")) {
      const style = document.createElement("style");
      style.id = "operation-progress-styles";
      style.textContent = `.operation-progress{box-sizing:border-box;flex-basis:100%;width:100%;padding:12px;border:1px solid var(--tp-border,#475569);border-radius:4px;background:var(--tp-raised,#1a2b40);color:var(--tp-text,#e2e8f0)}.operation-progress progress{display:block;width:100%;height:8px;accent-color:var(--tp-accent,#38c5b2)}.operation-progress p{margin:8px 0 0;overflow-wrap:anywhere}`;
      document.head.append(style);
    }
    const element2 = document.createElement("div");
    element2.className = "operation-progress";
    const bar = document.createElement("progress");
    bar.setAttribute("aria-label", label);
    const status = document.createElement("p");
    status.setAttribute("role", "status");
    status.setAttribute("aria-live", "polite");
    element2.append(bar, status);
    container.append(element2);
    const started = performance.now();
    const update = () => {
      status.textContent = message(Math.floor((performance.now() - started) / 1e3));
    };
    update();
    const timer = window.setInterval(update, 1e3);
    return () => {
      window.clearInterval(timer);
      element2.remove();
    };
  }

  // frontend/src/shared/template-manager.ts
  var messages = {
    en: {
      title: "OID additions / SNMP monitoring definitions",
      help: "Templates define device-specific monitoring. Common OIDs are additional candidates for all devices: CPU/memory use the template first, then the common OID, then standard fallback. CPU/memory common values must be percentages (0\u2013100). Hardware entries use type|numeric OID.",
      storage: "GUI changes are stored in SQLite, not written to bundled or BYO TOML files. Editing a profile creates a complete user-owned copy; Reset restores its current file-backed definition. Saves apply to new polling and diagnostic operations without restarting.",
      profile: "Profile",
      common: "Common additional OIDs (all devices)",
      save: "Save and apply",
      reload: "Reload saved definitions",
      reset: "Reset / remove user profile",
      add: "Add",
      remove: "Remove",
      import: "Import TOML",
      export: "Export TOML",
      newProfile: "New profile sysObjectID prefix",
      create: "Create profile",
      saved: "Saved and applied",
      savedToast: "Saved and applied (Generation {generation})",
      closeToast: "Dismiss notification",
      dirty: "Unsaved changes",
      source: "Definition source",
      version: "Saved / applied generation",
      property: "Optional section",
      preview: "Preview profile selection",
      sysObject: "sysObjectID",
      noSelector: "Enter a numeric sysObjectID prefix.",
      discard: "Discard all unsaved changes?",
      resetConfirm: "Reset this profile? A file-backed profile will be restored; a GUI-only profile will be removed. Save to apply.",
      disclaimer: "When configuring proprietary vendor OIDs or importing third-party templates, ensure compliance with your hardware vendor's EULA and support agreements. TraceWorks assumes no liability for unauthorized access to proprietary vendor metrics.",
      systemTab: "CPU / memory",
      interfacesTab: "Interfaces / optical power",
      hardwareTab: "Hardware sensors",
      profileTab: "Profile information",
      item: "Monitoring item",
      oids: "Configured OIDs",
      configuration: "Configuration",
      configured: "Configured",
      unconfigured: "Not configured",
      edit: "Edit",
      addItem: "+ Add item",
      itemType: "Item type",
      advanced: "Advanced settings",
      applyItem: "Apply to draft",
      cancel: "Cancel",
      empty: "No items configured",
      cpuCandidate: "CPU utilization OID",
      memorySource: "Memory source",
      customMetric: "Additional metric",
      domMetric: "Optical power metric",
      interfaceSettings: "Interface settings",
      cpuSettings: "CPU settings",
      domSettings: "Optical table settings",
      newProfileTitle: "Create profile",
      notes: "Definition storage and priority",
      invalidNumber: "Enter a valid number.",
      confirmRemove: "Remove this item from the draft?",
      loading: "Loading monitoring definitions",
      saving: "Saving and applying monitoring definitions",
      importing: "Importing TOML",
      exporting: "Exporting TOML",
      previewing: "Previewing profile selection",
      profileGuide: "Create monitoring rules that apply automatically to a specific vendor or model. The profile with the longest matching sysObjectID prefix is selected.",
      profileName: "Profile name",
      profileNameExample: "Cisco Catalyst 9200 / Yamaha RTX1220",
      registeredDevice: "Select a registered device",
      chooseDevice: "Select a device to use its sysObjectID",
      unknownOid: "sysObjectID not collected yet",
      probeIp: "Device IP address",
      community: "Community (optional; blank uses device or system credentials)",
      probe: "Fetch",
      probing: "Fetching sysObjectID\u2026",
      vendorScope: "Apply to the entire vendor",
      modelScope: "Apply to this model (full OID)",
      scopePreview: "Automatically applies to devices whose sysObjectID starts with",
      longestHint: "A more specific matching profile takes priority.",
      baseTemplate: "Initial settings (base template)",
      emptyTemplate: "Start empty",
      duplicateProfile: "A profile with this prefix already exists. Select it from the profile list.",
      deviceProfiles: "Device profiles (automatic sysObjectID selection)",
      commonProfiles: "Shared settings for all devices",
      condition: "Applies when sysObjectID starts with",
      fallbackCondition: "Fallback when no device-specific profile matches",
      matchingDevices: "Matching registered devices",
      commonCondition: "All registered devices (additional OID candidates)",
      scalarHint: "Single-value (scalar) OIDs must end in .0.",
      scaleHint: "Use 0.1 to convert integers in tenths (for example, 455 = 45.5\xB0C)."
    },
    ja: {
      title: "OID\u8FFD\u52A0\u30FBSNMP\u76E3\u8996\u5B9A\u7FA9",
      help: "\u30C6\u30F3\u30D7\u30EC\u30FC\u30C8\u3067\u6A5F\u5668\u5225\u306E\u76E3\u8996\u3092\u5B9A\u7FA9\u3057\u307E\u3059\u3002\u5171\u901AOID\u306F\u5168\u30C7\u30D0\u30A4\u30B9\u3078\u306E\u8FFD\u52A0\u5019\u88DC\u3067\u3059\u3002CPU\u30FB\u30E1\u30E2\u30EA\u306F\u30C6\u30F3\u30D7\u30EC\u30FC\u30C8\u2192\u5171\u901AOID\u2192\u6A19\u6E96\u30D5\u30A9\u30FC\u30EB\u30D0\u30C3\u30AF\u306E\u9806\u3067\u63A1\u7528\u3057\u307E\u3059\u3002\u5171\u901ACPU\u30FB\u30E1\u30E2\u30EA\u306F\u5272\u5408\u5024\uFF080\u301C100\uFF09\u5C02\u7528\u3067\u3059\u3002\u30CF\u30FC\u30C9\u30A6\u30A7\u30A2\u9805\u76EE\u306F \u7A2E\u5225|\u6570\u5024OID \u306E\u5F62\u5F0F\u3067\u3059\u3002",
      storage: "GUI\u306E\u5909\u66F4\u306FSQLite\u306B\u4FDD\u5B58\u3057\u3001\u540C\u68B1\u30FBBYO\u306ETOML\u306F\u66F8\u304D\u63DB\u3048\u307E\u305B\u3093\u3002\u7DE8\u96C6\u3057\u305F\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\u306F\u5168\u9805\u76EE\u3092\u542B\u3080\u30E6\u30FC\u30B6\u30FC\u5B9A\u7FA9\u3068\u306A\u308A\u3001\u30EA\u30BB\u30C3\u30C8\u3067\u73FE\u5728\u306E\u30D5\u30A1\u30A4\u30EB\u5B9A\u7FA9\u306B\u623B\u308A\u307E\u3059\u3002\u4FDD\u5B58\u5F8C\u3001\u65B0\u3057\u3044\u30DD\u30FC\u30EA\u30F3\u30B0\u30FB\u8A3A\u65AD\u304B\u3089\u518D\u8D77\u52D5\u306A\u3057\u3067\u53CD\u6620\u3057\u307E\u3059\u3002",
      profile: "\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB",
      common: "\u5171\u901A\u8FFD\u52A0OID\uFF08\u5168\u30C7\u30D0\u30A4\u30B9\uFF09",
      save: "\u4FDD\u5B58\u3057\u3066\u9069\u7528",
      reload: "\u4FDD\u5B58\u6E08\u307F\u5B9A\u7FA9\u3092\u518D\u8AAD\u8FBC",
      reset: "\u30EA\u30BB\u30C3\u30C8\u30FB\u30E6\u30FC\u30B6\u30FC\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\u524A\u9664",
      add: "\u8FFD\u52A0",
      remove: "\u524A\u9664",
      import: "TOML\u3092\u30A4\u30F3\u30DD\u30FC\u30C8",
      export: "TOML\u3092\u30A8\u30AF\u30B9\u30DD\u30FC\u30C8",
      newProfile: "\u65B0\u898F\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\u306EsysObjectID\u30D7\u30EC\u30D5\u30A3\u30C3\u30AF\u30B9",
      create: "\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\u4F5C\u6210",
      saved: "\u4FDD\u5B58\u30FB\u9069\u7528\u3057\u307E\u3057\u305F",
      savedToast: "\u4FDD\u5B58\u30FB\u9069\u7528\u3057\u307E\u3057\u305F\uFF08\u4E16\u4EE3 {generation}\uFF09\u2014 \u6B21\u56DE\u306E\u30DD\u30FC\u30EA\u30F3\u30B0\u30FB\u8A3A\u65AD\u304B\u3089\u53CD\u6620\u3055\u308C\u307E\u3059",
      closeToast: "\u901A\u77E5\u3092\u9589\u3058\u308B",
      dirty: "\u672A\u4FDD\u5B58\u306E\u5909\u66F4\u304C\u3042\u308A\u307E\u3059",
      source: "\u5B9A\u7FA9\u5143",
      version: "\u4FDD\u5B58\u6E08\u307F\uFF0F\u9069\u7528\u6E08\u307F\u4E16\u4EE3",
      property: "\u8FFD\u52A0\u53EF\u80FD\u306A\u30BB\u30AF\u30B7\u30E7\u30F3",
      preview: "\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\u9078\u629E\u3092\u30D7\u30EC\u30D3\u30E5\u30FC",
      sysObject: "sysObjectID",
      noSelector: "\u6570\u5024\u306EsysObjectID\u30D7\u30EC\u30D5\u30A3\u30C3\u30AF\u30B9\u3092\u5165\u529B\u3057\u3066\u304F\u3060\u3055\u3044\u3002",
      discard: "\u672A\u4FDD\u5B58\u306E\u5909\u66F4\u3092\u3059\u3079\u3066\u7834\u68C4\u3057\u307E\u3059\u304B\uFF1F",
      resetConfirm: "\u30EA\u30BB\u30C3\u30C8\u3057\u307E\u3059\u304B\uFF1F\u30D5\u30A1\u30A4\u30EB\u7531\u6765\u306E\u5B9A\u7FA9\u306F\u5143\u306B\u623B\u3057\u3001GUI\u306E\u307F\u306E\u5B9A\u7FA9\u306F\u524A\u9664\u3057\u307E\u3059\u3002\u4FDD\u5B58\u3059\u308B\u307E\u3067\u76E3\u8996\u306B\u306F\u53CD\u6620\u3055\u308C\u307E\u305B\u3093\u3002",
      disclaimer: "\u30D9\u30F3\u30C0\u30FC\u72EC\u81EAOID\u3084\u7B2C\u4E09\u8005\u30C6\u30F3\u30D7\u30EC\u30FC\u30C8\u3092\u5229\u7528\u3059\u308B\u969B\u306F\u3001\u6A5F\u5668\u30D9\u30F3\u30C0\u30FC\u306EEULA\u3068\u30B5\u30DD\u30FC\u30C8\u5951\u7D04\u3078\u306E\u6E96\u62E0\u3092\u78BA\u8A8D\u3057\u3066\u304F\u3060\u3055\u3044\u3002TraceWorks\u306F\u30D9\u30F3\u30C0\u30FC\u72EC\u81EA\u30E1\u30C8\u30EA\u30AF\u30B9\u3078\u306E\u7121\u8A31\u53EF\u30A2\u30AF\u30BB\u30B9\u306B\u3064\u3044\u3066\u8CAC\u4EFB\u3092\u8CA0\u3044\u307E\u305B\u3093\u3002",
      systemTab: "CPU\u30FB\u30E1\u30E2\u30EA",
      interfacesTab: "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u30FB\u5149\u30EC\u30D9\u30EB",
      hardwareTab: "\u30CF\u30FC\u30C9\u30A6\u30A7\u30A2\u30BB\u30F3\u30B5\u30FC",
      profileTab: "\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\u60C5\u5831",
      item: "\u76E3\u8996\u9805\u76EE",
      oids: "\u8A2D\u5B9AOID",
      configuration: "\u8A2D\u5B9A\u72B6\u614B",
      configured: "\u8A2D\u5B9A\u6E08\u307F",
      unconfigured: "\u672A\u8A2D\u5B9A",
      edit: "\u7DE8\u96C6",
      addItem: "\uFF0B\u9805\u76EE\u8FFD\u52A0",
      itemType: "\u9805\u76EE\u306E\u7A2E\u985E",
      advanced: "\u8A73\u7D30\u8A2D\u5B9A",
      applyItem: "\u7DE8\u96C6\u5185\u5BB9\u3092\u53CD\u6620",
      cancel: "\u30AD\u30E3\u30F3\u30BB\u30EB",
      empty: "\u76E3\u8996\u9805\u76EE\u306F\u767B\u9332\u3055\u308C\u3066\u3044\u307E\u305B\u3093",
      cpuCandidate: "CPU\u4F7F\u7528\u7387OID",
      memorySource: "\u30E1\u30E2\u30EA\u53D6\u5F97\u5143",
      customMetric: "\u8FFD\u52A0\u76E3\u8996\u9805\u76EE",
      domMetric: "\u5149\u30EC\u30D9\u30EB\u76E3\u8996\u9805\u76EE",
      interfaceSettings: "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u306E\u8A73\u7D30\u8A2D\u5B9A",
      cpuSettings: "CPU\u306E\u8A73\u7D30\u8A2D\u5B9A",
      domSettings: "\u5149\u30EC\u30D9\u30EB\u30C6\u30FC\u30D6\u30EB\u306E\u8A73\u7D30\u8A2D\u5B9A",
      newProfileTitle: "\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\u4F5C\u6210",
      notes: "\u5B9A\u7FA9\u306E\u4FDD\u5B58\u5148\u30FB\u9069\u7528\u9806",
      invalidNumber: "\u6709\u52B9\u306A\u6570\u5024\u3092\u5165\u529B\u3057\u3066\u304F\u3060\u3055\u3044\u3002",
      confirmRemove: "\u3053\u306E\u9805\u76EE\u3092\u4E0B\u66F8\u304D\u304B\u3089\u524A\u9664\u3057\u307E\u3059\u304B\uFF1F",
      loading: "\u76E3\u8996\u5B9A\u7FA9\u3092\u8AAD\u307F\u8FBC\u3093\u3067\u3044\u307E\u3059",
      saving: "\u76E3\u8996\u5B9A\u7FA9\u3092\u4FDD\u5B58\u30FB\u9069\u7528\u3057\u3066\u3044\u307E\u3059",
      importing: "TOML\u3092\u30A4\u30F3\u30DD\u30FC\u30C8\u3057\u3066\u3044\u307E\u3059",
      exporting: "TOML\u3092\u30A8\u30AF\u30B9\u30DD\u30FC\u30C8\u3057\u3066\u3044\u307E\u3059",
      previewing: "\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\u9078\u629E\u3092\u30D7\u30EC\u30D3\u30E5\u30FC\u3057\u3066\u3044\u307E\u3059",
      profileGuide: "\u7279\u5B9A\u306E\u30D9\u30F3\u30C0\u30FC\u3084\u6A5F\u7A2E\u306B\u81EA\u52D5\u9069\u7528\u3059\u308B\u76E3\u8996\u30EB\u30FC\u30EB\u3092\u4F5C\u6210\u3057\u307E\u3059\u3002\u6A5F\u5668\u306E sysObjectID \u306E\u5148\u982D\u4E00\u81F4\uFF08\u6700\u9577\u4E00\u81F4\uFF09\u3067\u81EA\u52D5\u7684\u306B\u9078\u3070\u308C\u307E\u3059\u3002",
      profileName: "\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\u540D",
      profileNameExample: "Cisco Catalyst 9200 / Yamaha RTX1220",
      registeredDevice: "\u767B\u9332\u6E08\u307F\u30C7\u30D0\u30A4\u30B9\u304B\u3089\u9078\u629E",
      chooseDevice: "sysObjectID\u3092\u53D6\u5F97\u3059\u308B\u30C7\u30D0\u30A4\u30B9\u3092\u9078\u629E",
      unknownOid: "sysObjectID\u672A\u53D6\u5F97",
      probeIp: "\u6A5F\u5668\u306EIP\u30A2\u30C9\u30EC\u30B9",
      community: "\u30B3\u30DF\u30E5\u30CB\u30C6\u30A3\uFF08\u4EFB\u610F\u30FB\u7A7A\u6B04\u306F\u500B\u5225\uFF0F\u30B7\u30B9\u30C6\u30E0\u5171\u901A\u8A2D\u5B9A\u3092\u4F7F\u7528\uFF09",
      probe: "\u53D6\u5F97",
      probing: "sysObjectID\u3092\u53D6\u5F97\u3057\u3066\u3044\u307E\u3059\u2026",
      vendorScope: "\u30D9\u30F3\u30C0\u30FC\u5168\u4F53\u306B\u9069\u7528",
      modelScope: "\u3053\u306E\u6A5F\u7A2E\u5C02\u7528\u306B\u9069\u7528\uFF08\u30D5\u30EBOID\uFF09",
      scopePreview: "\u3067\u59CB\u307E\u308B\u3059\u3079\u3066\u306E\u6A5F\u5668\u306B\u81EA\u52D5\u9069\u7528\u3055\u308C\u307E\u3059",
      longestHint: "\u3088\u308A\u5177\u4F53\u7684\u306A\u6761\u4EF6\u306E\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\u304C\u3042\u308B\u5834\u5408\u306F\u3001\u305D\u3061\u3089\u304C\u512A\u5148\u3055\u308C\u307E\u3059\u3002",
      baseTemplate: "\u521D\u671F\u8A2D\u5B9A\uFF08\u30D9\u30FC\u30B9\u30C6\u30F3\u30D7\u30EC\u30FC\u30C8\uFF09",
      emptyTemplate: "\u7A7A\u304B\u3089\u4F5C\u6210",
      duplicateProfile: "\u3053\u306E\u30D7\u30EC\u30D5\u30A3\u30C3\u30AF\u30B9\u306E\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\u306F\u65E2\u306B\u3042\u308A\u307E\u3059\u3002\u4E00\u89A7\u304B\u3089\u9078\u629E\u3057\u3066\u304F\u3060\u3055\u3044\u3002",
      deviceProfiles: "\u6A5F\u5668\u5225\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\uFF08sysObjectID \u81EA\u52D5\u9078\u629E\uFF09",
      commonProfiles: "\u5168\u30C7\u30D0\u30A4\u30B9\u5171\u901A\u8A2D\u5B9A",
      condition: "\u9069\u7528\u6761\u4EF6\uFF08sysObjectID\u30D7\u30EC\u30D5\u30A3\u30C3\u30AF\u30B9\uFF09",
      fallbackCondition: "\u6A5F\u5668\u5225\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\u306B\u4E00\u81F4\u3057\u306A\u3044\u6A5F\u5668\u306B\u9069\u7528",
      matchingDevices: "\u4E00\u81F4\u3059\u308B\u767B\u9332\u6A5F\u5668",
      commonCondition: "\u3059\u3079\u3066\u306E\u767B\u9332\u6A5F\u5668\uFF08\u8FFD\u52A0OID\u5019\u88DC\uFF09",
      scalarHint: "\u203B\u5358\u4E00\u5024\uFF08\u30B9\u30AB\u30E9\u30FC\uFF09\u306EOID\u306F\u672B\u5C3E\u306B .0 \u3092\u4ED8\u3051\u3066\u304F\u3060\u3055\u3044",
      scaleHint: "\u203B0.1\u5358\u4F4D\u306E\u6574\u6570\uFF08\u4F8B: 455 = 45.5\u2103\uFF09\u3092\u63DB\u7B97\u3059\u308B\u5834\u5408\u306F 0.1 \u3092\u6307\u5B9A"
    }
  };
  var text = (key) => messages[localStorage.getItem("tracepulse-lang") === "ja" ? "ja" : "en"][key];
  var clone = (value) => JSON.parse(JSON.stringify(value));
  function validOid(value) {
    const parts = value.replace(/^\./, "").split(".");
    return parts.length >= 2 && parts.length <= 128 && parts.every((part) => /^\d+$/.test(part) && Number(part) <= 4294967295) && Number(parts[0]) <= 2 && (Number(parts[0]) === 2 || Number(parts[1]) <= 39);
  }
  function selector(doc) {
    return typeof doc.sysObjectID_prefix === "string" ? doc.sysObjectID_prefix.replace(/^\.+|\.+$/g, "") : Number(doc.enterprise_id) ? `1.3.6.1.4.1.${doc.enterprise_id}` : "";
  }
  function selectedProfile(oid, profiles) {
    let selected = "generic", segments = 0;
    if (!oid || !validOid(oid)) return selected;
    oid = oid.replace(/^\.+/, "");
    for (const profile of profiles.values()) {
      const prefix = selector(profile.document), length = prefix.split(".").length;
      if (prefix && (oid === prefix || oid.startsWith(`${prefix}.`)) && length >= segments) {
        selected = profile.id;
        segments = length;
      }
    }
    return selected;
  }
  async function registeredDevices() {
    const response = await fetch("/api/devices");
    const result = await response.json();
    if (!response.ok || !Array.isArray(result)) {
      throw new Error(!Array.isArray(result) && result.error || `HTTP ${response.status}`);
    }
    return result;
  }
  var fieldLabels = {
    profile: ["Monitoring profile", "\u76E3\u8996\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB"],
    common: ["Common additional OIDs", "\u5171\u901A\u8FFD\u52A0OID"],
    name: ["Profile name", "\u30D7\u30ED\u30D5\u30A1\u30A4\u30EB\u540D"],
    description: ["Description", "\u8AAC\u660E"],
    sysObjectID_prefix: ["Device identification (sysObjectID prefix)", "\u6A5F\u5668\u8B58\u5225\u6761\u4EF6\uFF08sysObjectID\u30D7\u30EC\u30D5\u30A3\u30C3\u30AF\u30B9\uFF09"],
    enterprise_id: ["Vendor enterprise number", "\u30D9\u30F3\u30C0\u30FC\u4F01\u696D\u756A\u53F7"],
    cpu: ["CPU monitoring", "CPU\u76E3\u8996"],
    candidates: ["CPU utilization OIDs (priority order)", "CPU\u4F7F\u7528\u7387OID\uFF08\u512A\u5148\u9806\uFF09"],
    cpu_oid: ["Additional CPU utilization OID", "\u8FFD\u52A0CPU\u4F7F\u7528\u7387OID"],
    memory_oid: ["Additional memory utilization OID", "\u8FFD\u52A0\u30E1\u30E2\u30EA\u4F7F\u7528\u7387OID"],
    hardware_oids: ["Additional hardware sensor OIDs", "\u8FFD\u52A0\u30CF\u30FC\u30C9\u30A6\u30A7\u30A2\u30BB\u30F3\u30B5\u30FCOID"],
    memory: ["Memory monitoring", "\u30E1\u30E2\u30EA\u76E3\u8996"],
    mode: ["Memory calculation method", "\u30E1\u30E2\u30EA\u4F7F\u7528\u7387\u306E\u8A08\u7B97\u65B9\u6CD5"],
    aggregation: ["Table value selection", "\u30C6\u30FC\u30D6\u30EB\u5024\u306E\u63A1\u7528\u65B9\u6CD5"],
    used_prefix: ["Used memory OID", "\u4F7F\u7528\u30E1\u30E2\u30EAOID"],
    free_prefix: ["Free memory OID", "\u7A7A\u304D\u30E1\u30E2\u30EAOID"],
    total_prefix: ["Total memory OID", "\u7DCF\u30E1\u30E2\u30EAOID"],
    used_is_bytes: ["Used memory is in bytes", "\u4F7F\u7528\u30E1\u30E2\u30EA\u306E\u5358\u4F4D\u306F\u30D0\u30A4\u30C8"],
    fallbacks: ["Alternative memory sources (priority order)", "\u4EE3\u66FF\u30E1\u30E2\u30EA\u53D6\u5F97\u5143\uFF08\u512A\u5148\u9806\uFF09"],
    tables: ["Interface and optical monitoring", "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u30FB\u5149\u30EC\u30D9\u30EB\u76E3\u8996"],
    interfaces: ["Interface traffic (IF-MIB)", "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u30C8\u30E9\u30D5\u30A3\u30C3\u30AF\uFF08IF-MIB\uFF09"],
    base_oid: ["Table base OID", "\u30C6\u30FC\u30D6\u30EB\u306E\u57FA\u70B9OID"],
    index: ["Interface index (ifIndex)", "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u756A\u53F7\uFF08ifIndex\uFF09"],
    in_octets: ["Received traffic (ifInOctets)", "\u53D7\u4FE1\u30C8\u30E9\u30D5\u30A3\u30C3\u30AF\uFF08ifInOctets\uFF09"],
    out_octets: ["Transmitted traffic (ifOutOctets)", "\u9001\u4FE1\u30C8\u30E9\u30D5\u30A3\u30C3\u30AF\uFF08ifOutOctets\uFF09"],
    dom: ["Optical power (DOM / DDM)", "\u5149\u30EC\u30D9\u30EB\uFF08DOM / DDM\uFF09"],
    index_oid: ["Interface mapping OID (ifIndex)", "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u5BFE\u5FDC\u4ED8\u3051OID\uFF08ifIndex\uFF09"],
    entity: ["ENTITY sensor mapping", "ENTITY\u30BB\u30F3\u30B5\u30FC\u5BFE\u5FDC\u4ED8\u3051"],
    metrics: ["Monitoring metrics", "\u76E3\u8996\u9805\u76EE"],
    key: ["Metric identifier", "\u76E3\u8996\u9805\u76EEID"],
    oid: ["Numeric OID", "\u6570\u5024OID"],
    unit: ["Unit", "\u5358\u4F4D"],
    label: ["Display label", "\u8868\u793A\u540D"],
    data_type: ["Value type", "\u5024\u306E\u7A2E\u985E"],
    scale: ["Multiplier", "\u500D\u7387"],
    offset: ["Value offset", "\u5024\u306E\u88DC\u6B63\u91CF"],
    invalid_values: ["Unmeasured sentinel values (before scaling)", "\u672A\u8A08\u6E2C\u3068\u3057\u3066\u6271\u3046\u5024\uFF08\u500D\u7387\u9069\u7528\u524D\uFF09"],
    match_names: ["Sensor name filters", "\u30BB\u30F3\u30B5\u30FC\u540D\u306E\u7D5E\u308A\u8FBC\u307F"],
    sensors: ["Hardware sensors", "\u30CF\u30FC\u30C9\u30A6\u30A7\u30A2\u30BB\u30F3\u30B5\u30FC"],
    temperature: ["Temperature sensors", "\u6E29\u5EA6\u30BB\u30F3\u30B5\u30FC"],
    power: ["Power sensors", "\u96FB\u6E90\u30BB\u30F3\u30B5\u30FC"],
    fan: ["Fan sensors", "\u30D5\u30A1\u30F3\u30BB\u30F3\u30B5\u30FC"],
    descr_prefix: ["Sensor name OID", "\u30BB\u30F3\u30B5\u30FC\u540DOID"],
    value_prefix: ["Sensor reading OID", "\u30BB\u30F3\u30B5\u30FC\u6E2C\u5B9A\u5024OID"],
    state_prefix: ["Sensor status OID", "\u30BB\u30F3\u30B5\u30FC\u72B6\u614BOID"],
    fallback_builtin_power: ["Synthetic power fallback (unsupported)", "\u4EEE\u60F3\u96FB\u6E90\u5024\u306E\u88DC\u5B8C\uFF08\u975E\u5BFE\u5FDC\uFF09"],
    alarm_states: ["Alarm status values", "\u7570\u5E38\u3068\u3059\u308B\u72B6\u614B\u5024"],
    normal_states: ["Normal status values", "\u6B63\u5E38\u3068\u3059\u308B\u72B6\u614B\u5024"],
    state_labels: ["Status display names", "\u72B6\u614B\u5024\u306E\u8868\u793A\u540D"],
    name_oid: ["Sensor name OID", "\u30BB\u30F3\u30B5\u30FC\u540DOID"],
    contained_in_oid: ["Parent entity OID", "\u89AA\u30A8\u30F3\u30C6\u30A3\u30C6\u30A3OID"],
    alias_oid: ["Interface alias mapping OID", "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u5225\u540D\u5BFE\u5FDCOID"],
    type_oid: ["Sensor type OID", "\u30BB\u30F3\u30B5\u30FC\u7A2E\u5225OID"],
    scale_oid: ["Sensor scale OID", "\u30BB\u30F3\u30B5\u30FC\u500D\u7387OID"],
    precision_oid: ["Sensor precision OID", "\u30BB\u30F3\u30B5\u30FC\u7CBE\u5EA6OID"],
    status_oid: ["Sensor availability OID", "\u30BB\u30F3\u30B5\u30FC\u7A3C\u50CD\u72B6\u614BOID"],
    category: ["Metric category", "\u76E3\u8996\u9805\u76EE\u306E\u5206\u985E"],
    rule: ["Trend evaluation rule", "\u50BE\u5411\u8A55\u4FA1\u30EB\u30FC\u30EB"],
    instance: ["Scalar or table", "\u5358\u4E00\u5024\u30FB\u30C6\u30FC\u30D6\u30EB"],
    match_name: ["Sensor name match", "\u5BFE\u8C61\u30BB\u30F3\u30B5\u30FC\u540D"],
    unit_oid: ["Sensor unit OID", "\u30BB\u30F3\u30B5\u30FC\u5358\u4F4DOID"],
    match_unit: ["Required sensor unit", "\u5BFE\u8C61\u30BB\u30F3\u30B5\u30FC\u5358\u4F4D"],
    warning_value: ["Warning threshold", "\u8B66\u544A\u3057\u304D\u3044\u5024"],
    min_slope_per_day: ["Minimum daily change", "1\u65E5\u3042\u305F\u308A\u306E\u6700\u5C0F\u5909\u5316\u91CF"],
    forecast_days: ["Forecast horizon (days)", "\u4E88\u6E2C\u671F\u9593\uFF08\u65E5\uFF09"],
    min_samples: ["Minimum sample count", "\u6700\u5C0F\u30B5\u30F3\u30D7\u30EB\u6570"],
    min_period_days: ["Minimum observation period (days)", "\u6700\u5C0F\u89B3\u6E2C\u671F\u9593\uFF08\u65E5\uFF09"]
  };
  function fieldLabel(path) {
    const parts = path.split(".");
    const key = parts[parts.length - 1];
    const language = localStorage.getItem("tracepulse-lang") === "ja" ? 1 : 0;
    if (/^\d+$/.test(key)) {
      const parent = parts[parts.length - 2];
      if (parent === "state_labels") return `${language ? "\u72B6\u614B\u5024" : "Status value"} ${key}`;
      const label = parent === "candidates" ? ["CPU utilization OID", "CPU\u4F7F\u7528\u7387OID"] : fieldLabels[parent] || ["Entry", "\u9805\u76EE"];
      return `${label[language]} ${Number(key) + 1}`;
    }
    if (path === "profile.tables.interfaces.name") return language ? "\u30A4\u30F3\u30BF\u30FC\u30D5\u30A7\u30FC\u30B9\u540D\uFF08ifDescr\uFF09" : "Interface name (ifDescr)";
    if (key === "name" && parts.includes("metrics")) return language ? "\u76E3\u8996\u9805\u76EE\u540D" : "Metric name";
    if (key === "oid" && parts.includes("memory")) return language ? "\u30E1\u30E2\u30EA\u4F7F\u7528\u7387OID" : "Memory utilization OID";
    return fieldLabels[key]?.[language] || (language ? "\u8FFD\u52A0\u9805\u76EE" : "Additional field");
  }
  async function api(path, body) {
    const response = await fetch(path, body === void 0 ? {} : {
      method: "POST",
      headers: { "Content-Type": "application/json", ...window.tracepulseRequestHeaders?.() },
      body: JSON.stringify(body)
    });
    const result = await response.json();
    if (!response.ok || !result.ok || result.data === void 0) throw new Error(result.error || `HTTP ${response.status}`);
    return result.data;
  }
  function element(tag, content) {
    const node = document.createElement(tag);
    if (content !== void 0) node.textContent = content;
    return node;
  }
  function object(value) {
    return value && typeof value === "object" && !Array.isArray(value) ? value : {};
  }
  function array(value) {
    return Array.isArray(value) ? value : [];
  }
  function configuredOids(value) {
    if (typeof value === "string") return value.split("|").filter((part) => /^\.?\d+(\.\d+)+$/.test(part));
    if (typeof value !== "object") return [];
    return [...new Set(Object.values(value).flatMap(configuredOids))];
  }
  function button(label, action) {
    const node = element("button", label);
    node.type = "button";
    node.addEventListener("click", action);
    return node;
  }
  function optionalSections(path, value) {
    if (path === "profile") return {
      cpu: { candidates: [], aggregation: "first" },
      memory: { mode: "direct", oid: "", fallbacks: [] },
      sensors: { temperature: [], power: [], fan: [] },
      metrics: [],
      tables: {}
    };
    if (path.endsWith(".tables")) return {
      interfaces: { base_oid: "", index: "", name: "", in_octets: "", out_octets: "" },
      dom: { base_oid: "", metrics: [] }
    };
    if (path.endsWith(".sensors")) return { temperature: [], power: [], fan: [] };
    if (path.endsWith(".memory")) return { mode: "direct", oid: "", used_prefix: "", free_prefix: "", total_prefix: "", aggregation: "first", used_is_bytes: false, fallbacks: [] };
    if (path.endsWith(".dom")) return { index_oid: "", entity: { name_oid: "", contained_in_oid: "", alias_oid: "", type_oid: "", scale_oid: "", precision_oid: "", status_oid: "" } };
    if (/\.sensors\.(temperature|power|fan)\.\d+$/.test(path)) return { alarm_states: [2, 3, 4, 6] };
    if (/\.metrics\.\d+$/.test(path) && !("key" in value)) return {
      name_oid: "",
      match_name: "",
      unit_oid: "",
      match_unit: "",
      scale_oid: "",
      precision_oid: "",
      warning_value: 0
    };
    return {};
  }
  function arraySeed(path, current) {
    if (path.endsWith(".metrics")) {
      if (path.includes(".dom.")) return { key: "rx_power", oid: "", unit: "dBm", scale: 0.01, label: "Rx Power", invalid_values: [], match_names: [] };
      return { key: "custom_metric", oid: "", unit: "count", label: "Custom metric", data_type: "gauge" };
    }
    if (path.endsWith(".fallbacks")) return { mode: "direct", oid: "", used_prefix: "", free_prefix: "", total_prefix: "", aggregation: "first", used_is_bytes: false };
    if (/\.(temperature|power|fan)$/.test(path)) return {
      descr_prefix: "",
      value_prefix: "",
      state_prefix: "",
      fallback_builtin_power: false,
      normal_states: [],
      state_labels: {},
      match_names: []
    };
    if (path.endsWith(".invalid_values") || path.endsWith(".normal_states") || path.endsWith(".alarm_states")) return 0;
    if (current.length && typeof current[0] !== "object") return typeof current[0] === "number" ? 0 : "";
    return "";
  }
  function initializeTemplateManager() {
    const root = document.getElementById("snmp-definitions");
    if (!root) return;
    const style = element("style");
    style.textContent = `
        #snmp-definitions .template-controls{display:flex;gap:8px;flex-wrap:wrap;align-items:center;margin:12px 0}
        #snmp-definitions .template-source{display:grid;grid-template-columns:max-content minmax(0,1fr);gap:4px 12px;margin:12px 0;padding:10px 12px;border-left:3px solid var(--tp-accent,#38c5b2);background:var(--tp-bg,#0b1524);font-size:.88rem}
        #snmp-definitions .template-source dt{color:var(--tp-muted,#94a3b8)}
        #snmp-definitions .template-source dd{min-width:0;margin:0;font-weight:600;color:var(--tp-text,#eef5fb);overflow-wrap:anywhere}
        html[data-theme=light] #snmp-definitions .template-source{background:#f1f5f9}
        html[data-theme=light] #snmp-definitions .template-source dt{color:#64748b}
        html[data-theme=light] #snmp-definitions .template-source dd{color:#0f172a}
        #snmp-definitions fieldset{min-width:0;margin:10px 0;padding:12px;border:1px solid var(--tp-border,#475569);border-radius:4px}
        #snmp-definitions legend{padding:0 5px;overflow-wrap:anywhere}
        #snmp-definitions label{display:grid;gap:5px;margin:8px 0;overflow-wrap:anywhere}
        #snmp-definitions input,#snmp-definitions select{max-width:100%;min-width:0;padding:8px;background:var(--tp-surface,#152337);color:var(--tp-text,#eef5fb);border:1px solid #64748b;border-radius:4px}
        #snmp-definitions button{max-width:100%;overflow-wrap:anywhere;padding:7px 10px;background:var(--tp-raised,#1a2b40);color:var(--tp-text,#eef5fb);border:1px solid #64748b;border-radius:4px;cursor:pointer}
        #snmp-definitions button:disabled{opacity:.5;cursor:default}
        #snmp-definitions .template-scalar{display:flex;gap:8px;align-items:end}#snmp-definitions .template-scalar label{flex:1}
        #snmp-definitions [role=alert]{color:#f87171}html[data-theme=light] #snmp-definitions [role=alert]{color:#b91c1c}
        #snmp-definitions p{margin:10px 0;overflow-wrap:anywhere}
        #snmp-definitions .template-toolbar{position:sticky;top:0;z-index:5;display:flex;flex-wrap:wrap;gap:8px;align-items:flex-end;padding:12px 0;background:var(--tp-surface,#172638);border-bottom:1px solid var(--tp-border,#334155)}
        #snmp-definitions .template-toolbar>label{flex:1 1 260px;min-width:0;margin:0}
        #snmp-definitions .template-toolbar select,#snmp-definitions .template-toolbar button{box-sizing:border-box;height:40px;font-size:.9rem;line-height:1.25}
        @media(max-width:600px){#snmp-definitions .template-toolbar>label{flex-basis:100%}#snmp-definitions .template-toolbar button{height:auto;min-height:40px}}
        #snmp-definitions .template-toolbar .template-save{background:var(--tp-accent,#38c5b2);color:#082524;border-color:transparent;font-weight:600}
        #snmp-definitions .template-toolbar[data-state=dirty] .template-save{outline:2px solid #f59e0b;outline-offset:2px;box-shadow:0 0 12px rgba(245,158,11,.5)}
        #snmp-definitions .template-status-bar{display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin:10px 0}
        #snmp-definitions .template-status-bar p{margin:0}
        #snmp-definitions [data-template-status]:empty{display:none}
        #snmp-definitions [data-template-status][data-state]{display:inline-flex;align-items:center;gap:.45rem;max-width:100%;box-sizing:border-box;padding:.45rem .7rem;border-radius:6px;font-weight:600}
        #snmp-definitions [data-template-status][data-state=saved]{background:rgba(20,83,45,.55);border:1px solid #22c55e;color:#86efac;animation:template-status-appear .3s ease-out}
        #snmp-definitions [data-template-status][data-state=saved]::before{content:"\u2713";font-weight:700}
        #snmp-definitions [data-template-status][data-state=dirty]{background:rgba(120,53,15,.45);border:1px solid #f59e0b;color:#fde68a}
        #snmp-definitions [data-template-status][data-state=dirty]::before{content:"\u25CF";font-size:.75em}
        html[data-theme=light] #snmp-definitions [data-template-status][data-state=saved]{background:#dcfce7;border-color:#16a34a;color:#166534}
        html[data-theme=light] #snmp-definitions [data-template-status][data-state=dirty]{background:#fef3c7;border-color:#d97706;color:#92400e}
        #snmp-definitions .template-generation{display:inline-flex;align-items:center;max-width:100%;box-sizing:border-box;padding:.4rem .7rem;border:1px solid var(--tp-border,#475569);border-radius:999px;background:var(--tp-bg,#0b1524);color:var(--tp-text,#eef5fb);font-size:.85rem}
        html[data-theme=light] #snmp-definitions .template-generation{background:#f1f5f9;color:#0f172a}
        #snmp-definitions .template-generation.template-generation-flash{animation:template-generation-flash 1.5s ease-out}
        #snmp-definitions .template-items tr.template-row-highlight td{animation:template-row-highlight 2.5s ease-out}
        .template-saved-toast{position:fixed;z-index:1200;top:1rem;right:1rem;display:flex;align-items:flex-start;gap:.6rem;box-sizing:border-box;width:max-content;max-width:calc(100vw - 2rem);padding:.75rem 1rem;border:1px solid #22c55e;border-radius:8px;background:#14532d;color:#dcfce7;box-shadow:0 8px 24px #0006;transition:opacity .25s ease,transform .25s ease;animation:template-status-appear .3s ease-out}
        .template-saved-toast::before{content:"\u2713";font-weight:700}
        .template-saved-toast span{overflow-wrap:anywhere}
        .template-saved-toast button{flex:none;padding:0 .2rem;border:0;background:transparent;color:inherit;font-size:1.2rem;line-height:1;cursor:pointer}
        .template-saved-toast.is-closing{opacity:0;transform:translateY(-8px);pointer-events:none}
        html[data-theme=light] .template-saved-toast{background:#dcfce7;border-color:#16a34a;color:#166534;box-shadow:0 8px 24px #0003}
        @keyframes template-status-appear{from{opacity:0;transform:translateY(-4px)}to{opacity:1;transform:translateY(0)}}
        @keyframes template-generation-flash{0%,30%{background:#166534;color:#dcfce7;box-shadow:0 0 12px #22c55e}100%{box-shadow:0 0 0 transparent}}
        @keyframes template-row-highlight{0%,20%{background:rgba(34,197,94,.35)}100%{background:transparent}}
        @media(prefers-reduced-motion:reduce){#snmp-definitions [data-template-status][data-state=saved],#snmp-definitions .template-generation.template-generation-flash,#snmp-definitions .template-items tr.template-row-highlight td,.template-saved-toast{animation:none}.template-saved-toast{transition:none}}
        #snmp-definitions .template-tabs{display:flex;gap:4px;overflow:auto;border-bottom:1px solid var(--tp-border,#334155);margin:14px 0}
        #snmp-definitions .template-tabs button{flex:0 0 auto;border:0;border-bottom:3px solid transparent;border-radius:0;background:transparent;padding:10px 12px}
        #snmp-definitions .template-tabs [aria-selected=true]{border-bottom-color:var(--tp-accent,#38c5b2);color:var(--tp-accent,#38c5b2);font-weight:600}
        #snmp-definitions .template-list-actions{display:flex;flex-wrap:wrap;justify-content:flex-end;gap:8px;margin:12px 0}
        #snmp-definitions .template-table-scroll{overflow:auto;max-width:100%}
        #snmp-definitions .template-items{table-layout:fixed;min-width:760px;margin:0}
        #snmp-definitions .template-items th:nth-child(1){width:22%}#snmp-definitions .template-items th:nth-child(2){width:42%}
        #snmp-definitions .template-items th:nth-child(3){width:18%}#snmp-definitions .template-items th:nth-child(4){width:18%}
        #snmp-definitions .template-items th:nth-child(3),#snmp-definitions .template-items td:nth-child(3){white-space:nowrap}
        #snmp-definitions .template-items th,#snmp-definitions .template-items td{vertical-align:top;overflow-wrap:anywhere}
        #snmp-definitions .template-oid{display:block;font-family:monospace;font-size:.82rem;margin-bottom:4px;overflow-wrap:anywhere}
        #snmp-definitions .template-row-actions{display:flex;gap:6px;flex-wrap:wrap}
        .template-dialog{box-sizing:border-box;width:min(760px,calc(100% - 24px));max-height:calc(100dvh - 32px);overflow:auto;margin:auto;padding:20px;border:1px solid var(--tp-border,#475569);border-radius:8px;background:var(--tp-surface,#172638);color:var(--tp-text,#eef5fb)}
        .template-dialog::backdrop{background:rgb(0 0 0 / .6)}
        .template-dialog h3{margin:0 0 14px;font-size:1.05rem;overflow-wrap:anywhere}
        .template-dialog fieldset{min-width:0;border:0;border-top:1px solid var(--tp-border,#475569);padding:12px 0;margin:12px 0}
        .template-dialog legend{padding-right:8px;overflow-wrap:anywhere}
        .template-dialog label{display:grid;gap:6px;margin:10px 0;min-width:0;overflow-wrap:anywhere}
        .template-dialog input,.template-dialog select{box-sizing:border-box;width:100%;min-width:0;padding:8px;background:var(--tp-bg,#0b1524);color:inherit;border:1px solid #64748b;border-radius:4px}
        .template-dialog input[type=checkbox]{width:auto;justify-self:start}
        .template-dialog button{padding:8px 12px;border:1px solid #64748b;border-radius:4px;background:var(--tp-raised,#1a2b40);color:inherit;cursor:pointer}
        .template-dialog .template-scalar{display:flex;gap:8px;align-items:end}.template-dialog .template-scalar label{flex:1}
        .template-dialog .template-dialog-actions{display:flex;gap:8px;justify-content:flex-end;position:sticky;bottom:-20px;padding:12px 0;background:var(--tp-surface,#172638)}
        .template-dialog .template-advanced{border-top:1px solid var(--tp-border,#475569);padding:10px 0;margin-top:12px}
        .template-dialog summary,#snmp-definitions summary{cursor:pointer;padding:8px 0}
        .template-dialog [role=alert]{color:#f87171}
        .template-dialog .template-hint{font-size:.85rem;font-weight:normal;color:var(--tp-muted,#94a3b8)}
        .template-dialog .template-scope-actions{display:flex;flex-wrap:wrap;gap:8px;margin:10px 0}
        .template-dialog button:disabled{opacity:.5;cursor:default}
        html[data-theme=light] .template-dialog,html[data-theme=light] .template-dialog .template-dialog-actions,html[data-theme=light] #snmp-definitions .template-toolbar{background:#fff;color:#0f172a}
        html[data-theme=light] .template-dialog input,html[data-theme=light] .template-dialog select,html[data-theme=light] .template-dialog button{background:#f1f5f9;color:#0f172a}
        html[data-theme=light] .template-dialog [role=alert]{color:#b91c1c}
    `;
    document.head.append(style);
    let view;
    let devices = [];
    let definitions;
    let selected = "generic";
    let dirty = false;
    let busy = false;
    let message = "";
    let savedMessage = false;
    let error = "";
    let flashGeneration = false;
    let toast;
    let toastTimer;
    let toastRemovalTimer;
    let capture = () => {
    };
    let category = "system";
    let activeDialog;
    let modalDirty = false;
    let operation = "loading";
    let operationStarted = 0;
    let stopOperationProgress = () => {
    };
    const showSavedToast = (generation) => {
      if (toastTimer !== void 0) window.clearTimeout(toastTimer);
      if (toastRemovalTimer !== void 0) window.clearTimeout(toastRemovalTimer);
      toast?.remove();
      const node = element("div");
      node.className = "template-saved-toast";
      node.setAttribute("role", "status");
      const content = element("span", text("savedToast").replace("{generation}", String(generation)));
      const close = button("\xD7", () => dismiss());
      close.setAttribute("aria-label", text("closeToast"));
      const dismiss = () => {
        if (toastTimer !== void 0) window.clearTimeout(toastTimer);
        node.classList.add("is-closing");
        toastRemovalTimer = window.setTimeout(() => {
          node.remove();
          if (toast === node) toast = void 0;
        }, 250);
      };
      node.append(content, close);
      document.body.append(node);
      toast = node;
      toastTimer = window.setTimeout(dismiss, 3500);
    };
    const run = async (action, kind = "loading") => {
      if (busy) return;
      operation = kind;
      operationStarted = performance.now();
      busy = true;
      error = "";
      render();
      if (view) root.querySelector(".operation-progress")?.scrollIntoView({ block: "nearest" });
      try {
        await action();
      } catch (reason) {
        error = String(reason);
      } finally {
        busy = false;
        render();
      }
    };
    const changed = () => {
      capture();
      dirty = true;
      message = "";
      savedMessage = false;
      const status = root.querySelector("[data-template-status]");
      if (status) {
        status.textContent = text("dirty");
        status.dataset.state = "dirty";
      }
      const toolbar = root.querySelector(".template-toolbar");
      if (toolbar) toolbar.dataset.state = "dirty";
    };
    const load = async () => {
      const [loadedView, loadedDevices] = await Promise.all([api("/api/snmp/definitions"), registeredDevices()]);
      view = loadedView;
      devices = loadedDevices;
      definitions = clone(view.definitions);
      dirty = false;
      message = "";
      savedMessage = false;
    };
    const editTree = (value, path, update, remove, notify = changed, refresh = render) => {
      if (typeof value !== "object") {
        const row = element("div");
        row.className = "template-scalar";
        const label = element("label", fieldLabel(path));
        const input = element("input");
        input.dataset.templatePath = path;
        input.type = typeof value === "boolean" ? "checkbox" : typeof value === "number" ? "number" : "text";
        if (input.type === "checkbox") input.checked = Boolean(value);
        else input.value = String(value);
        if (input.type === "number") input.step = "any";
        input.addEventListener("input", () => {
          update(input.type === "checkbox" ? input.checked : input.type === "number" ? input.valueAsNumber : input.value);
          notify();
        });
        label.append(input);
        const field = path.split(".").pop();
        const oidField = field !== "sysObjectID_prefix" && (/(^oid$|_oid$|_prefix$)/.test(field || "") || /^profile\.tables\.interfaces\.(index|name|in_octets|out_octets)$/.test(path) || /^profile\.cpu\.candidates\.\d+$/.test(path) || /^common\.hardware_oids\.\d+$/.test(path));
        if (field === "scale" || oidField) {
          const hint = element("small", text(field === "scale" ? "scaleHint" : "scalarHint"));
          hint.className = "template-hint";
          hint.id = `template-hint-${path.replace(/\./g, "-")}`;
          input.setAttribute("aria-label", fieldLabel(path));
          input.setAttribute("aria-describedby", hint.id);
          label.append(hint);
        }
        row.append(label);
        if (remove) row.append(button(text("remove"), () => {
          remove();
          notify();
          refresh();
        }));
        return row;
      }
      const group = element("fieldset");
      group.dataset.templateGroup = path;
      group.append(element("legend", fieldLabel(path)));
      if (remove) group.append(button(text("remove"), () => {
        remove();
        notify();
        refresh();
      }));
      if (Array.isArray(value)) {
        value.forEach((item, index) => group.append(editTree(item, `${path}.${index}`, (next) => {
          value[index] = next;
        }, () => {
          value.splice(index, 1);
        }, notify, refresh)));
        group.append(button(text("add"), () => {
          value.push(arraySeed(path, value));
          notify();
          refresh();
        }));
      } else {
        const advanced = element("details");
        advanced.className = "template-advanced";
        advanced.dataset.templateAdvanced = path;
        advanced.append(element("summary", text("advanced")));
        const advancedKeys = /* @__PURE__ */ new Set(["aggregation", "mode", "base_oid", "index_oid", "entity", "used_is_bytes", "fallbacks", "state_labels", "normal_states", "alarm_states", "invalid_values", "match_names", "offset", "scale", "data_type", "enterprise_id", "sysObjectID_prefix"]);
        for (const [key, item] of Object.entries(value)) {
          const locked = path === "profile" && ["enterprise_id", "sysObjectID_prefix"].includes(key);
          const field = editTree(
            item,
            `${path}.${key}`,
            (next) => {
              value[key] = next;
            },
            ["cpu", "memory", "sensors", "metrics", "tables"].includes(key) && path === "profile" || path.endsWith(".tables") || path.endsWith(".sensors") || path.endsWith(".state_labels") || path.endsWith(".dom") && ["entity", "index_oid"].includes(key) || /(?:\.sensors\.(?:temperature|power|fan)|\.metrics)\.\d+$/.test(path) && ["alarm_states", "name_oid", "match_name", "unit_oid", "match_unit", "scale_oid", "precision_oid", "warning_value"].includes(key) ? () => {
              delete value[key];
            } : void 0,
            notify,
            refresh
          );
          if (locked) field.querySelectorAll("input").forEach((input) => {
            input.disabled = true;
          });
          (advancedKeys.has(key) ? advanced : group).append(field);
        }
        if (path.endsWith(".state_labels")) {
          const key = element("input");
          key.type = "number";
          key.step = "1";
          key.setAttribute("aria-label", fieldLabel(`${path}.0`));
          const label = element("input");
          label.setAttribute("aria-label", fieldLabel("label"));
          group.append(key, label, button(text("add"), () => {
            if (!Number.isSafeInteger(key.valueAsNumber) || !label.value.trim()) {
              key.reportValidity();
              label.focus();
              return;
            }
            value[String(key.valueAsNumber)] = label.value;
            notify();
            refresh();
          }));
        }
        const options = Object.entries(optionalSections(path, value)).filter(([key]) => path !== "profile" && !(key in value));
        if (options.length) {
          const label = element("label", text("property"));
          const select = element("select");
          select.setAttribute("aria-label", text("property"));
          for (const [key] of options) select.append(new Option(fieldLabel(`${path}.${key}`), key));
          label.append(select);
          advanced.append(label);
          advanced.append(button(text("add"), () => {
            value[select.value] = clone(Object.fromEntries(options)[select.value]);
            notify();
            refresh();
          }));
        }
        if (advanced.children.length > 1) group.append(advanced);
      }
      return group;
    };
    const dialog = (title) => {
      activeDialog?.close();
      const previous = document.activeElement;
      const node = element("dialog");
      node.className = "template-dialog";
      node.id = "template-edit-dialog";
      const heading = element("h3", title);
      heading.id = "template-dialog-title";
      node.setAttribute("aria-labelledby", heading.id);
      node.append(heading);
      node.addEventListener("close", () => {
        node.remove();
        if (activeDialog === node) {
          activeDialog = void 0;
          modalDirty = false;
          if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
          else root.querySelector("#template-profile")?.focus();
        }
      });
      activeDialog = node;
      document.body.append(node);
      return node;
    };
    const editRow = (row) => {
      const node = dialog(`${text("edit")}: ${row.label}`);
      const form = element("form"), body = element("div"), alert = element("p");
      alert.setAttribute("role", "alert");
      const actions = element("div");
      actions.className = "template-dialog-actions";
      const apply = button(text("applyItem"), () => {
      });
      apply.type = "submit";
      actions.append(button(text("cancel"), () => node.close()), apply);
      form.append(body, alert, actions);
      node.append(form);
      let local = clone(row.value);
      const commitDraft = capture;
      const refresh = () => {
        const open = Array.from(body.querySelectorAll("details[open]")).map((details) => details.dataset.templateAdvanced);
        body.replaceChildren(editTree(local, row.path, (next) => {
          local = next;
        }, void 0, () => {
          modalDirty = true;
        }, refresh));
        body.querySelectorAll("details").forEach((details) => {
          details.open = open.includes(details.dataset.templateAdvanced);
        });
      };
      refresh();
      form.onsubmit = (event) => {
        event.preventDefault();
        const invalid = Array.from(form.querySelectorAll("input[type=number][data-template-path]:not(:disabled)")).find((input) => !input.checkValidity() || !Number.isFinite(input.valueAsNumber));
        if (invalid) {
          const details = invalid.closest("details");
          if (details) details.open = true;
          alert.textContent = text("invalidNumber");
          invalid.focus();
          return;
        }
        row.commit(local);
        commitDraft();
        dirty = true;
        modalDirty = false;
        message = "";
        savedMessage = false;
        error = "";
        node.close();
        render();
        const path = row.highlightPath || row.path;
        const updated = Array.from(root.querySelectorAll("[data-template-row]")).find((entry) => entry.dataset.templateRow === path);
        if (updated) {
          updated.classList.add("template-row-highlight");
          window.setTimeout(() => updated.classList.remove("template-row-highlight"), 2500);
        }
      };
      node.showModal();
    };
    const rowsFor = (doc, common) => {
      const rows = [];
      const list = (values, path, title, commit) => {
        values.forEach((value, index) => rows.push({
          label: String(object(value).label || object(value).name || object(value).key || `${title} ${index + 1}`),
          path: `${path}.${index}`,
          value,
          commit: (next) => {
            values[index] = next;
            commit(values);
          },
          remove: () => {
            values.splice(index, 1);
            commit(values);
          }
        }));
      };
      if (common) {
        if (category === "system") for (const key of ["cpu_oid", "memory_oid"]) {
          if (doc[key]) rows.push({ label: fieldLabel(`common.${key}`), path: `common.${key}`, value: doc[key], commit: (next) => {
            doc[key] = next;
          }, remove: () => {
            doc[key] = "";
          } });
        }
        if (category === "hardware") list(array(doc.hardware_oids), "common.hardware_oids", fieldLabel("common.hardware_oids"), (values) => {
          doc.hardware_oids = values;
        });
        return rows;
      }
      if (category === "system") {
        list(array(object(doc.cpu).candidates), "profile.cpu.candidates", text("cpuCandidate"), (values) => {
          doc.cpu = { ...object(doc.cpu), candidates: values };
        });
        if ("memory" in doc) {
          const memory = object(doc.memory);
          rows.push({ label: text("memorySource"), path: "profile.memory", value: Object.fromEntries(Object.entries(memory).filter(([key]) => key !== "fallbacks")), commit: (next) => {
            doc.memory = { ...object(next), ...!("fallbacks" in object(next)) && memory.fallbacks ? { fallbacks: memory.fallbacks } : {} };
          }, remove: () => {
            delete doc.memory;
          } });
          list(array(memory.fallbacks), "profile.memory.fallbacks", fieldLabel("profile.memory.fallbacks"), (values) => {
            object(doc.memory).fallbacks = values;
          });
        }
        list(array(doc.metrics), "profile.metrics", text("customMetric"), (values) => {
          doc.metrics = values;
        });
      }
      if (category === "interfaces") {
        const tables = object(doc.tables), interfaces = object(tables.interfaces);
        for (const key of ["index", "name", "in_octets", "out_octets"]) if (key in interfaces) rows.push({
          label: fieldLabel(`profile.tables.interfaces.${key}`),
          path: `profile.tables.interfaces.${key}`,
          value: interfaces[key],
          commit: (next) => {
            interfaces[key] = next;
          },
          remove: () => {
            delete interfaces[key];
          }
        });
        list(array(object(tables.dom).metrics), "profile.tables.dom.metrics", text("domMetric"), (values) => {
          object(tables.dom).metrics = values;
        });
      }
      if (category === "hardware") for (const kind of ["temperature", "power", "fan"]) {
        list(array(object(doc.sensors)[kind]), `profile.sensors.${kind}`, fieldLabel(`profile.sensors.${kind}`), (values) => {
          doc.sensors = { ...object(doc.sensors), [kind]: values };
        });
      }
      if (category === "profile") rows.push({ label: String(doc.name || text("profileTab")), path: "profile", value: Object.fromEntries(Object.entries(doc).filter(([key]) => ["name", "description", "sysObjectID_prefix", "enterprise_id"].includes(key))), configured: true, commit: (next) => {
        Object.assign(doc, object(next));
      } });
      return rows;
    };
    const additionsFor = (doc, common) => {
      if (common) {
        if (category === "system") return ["cpu_oid", "memory_oid"].filter((key) => !doc[key]).map((key) => ({ label: fieldLabel(`common.${key}`), path: `common.${key}`, value: "", commit: (next) => {
          doc[key] = next;
        } }));
        if (category === "hardware") return [{ label: fieldLabel("common.hardware_oids"), path: `common.hardware_oids.${array(doc.hardware_oids).length}`, value: "temperature|", commit: (next) => {
          doc.hardware_oids = [...array(doc.hardware_oids), next];
        } }];
        return [];
      }
      if (category === "system") return [
        { label: text("cpuCandidate"), path: `profile.cpu.candidates.${array(object(doc.cpu).candidates).length}`, value: "", commit: (next) => {
          doc.cpu = { ...object(doc.cpu), candidates: [...array(object(doc.cpu).candidates), next] };
        } },
        {
          label: text("memorySource"),
          path: "profile.memory",
          highlightPath: "memory" in doc ? `profile.memory.fallbacks.${array(object(doc.memory).fallbacks).length}` : "profile.memory",
          value: { mode: "direct", oid: "", aggregation: "first", used_is_bytes: false },
          commit: (next) => {
            if ("memory" in doc) object(doc.memory).fallbacks = [...array(object(doc.memory).fallbacks), next];
            else doc.memory = next;
          }
        },
        { label: text("customMetric"), path: `profile.metrics.${array(doc.metrics).length}`, value: arraySeed("profile.metrics", []), commit: (next) => {
          doc.metrics = [...array(doc.metrics), next];
        } }
      ];
      if (category === "interfaces") return [
        ...["index", "name", "in_octets", "out_octets"].filter((key) => !(key in object(object(doc.tables).interfaces))).map((key) => ({
          label: fieldLabel(`profile.tables.interfaces.${key}`),
          path: `profile.tables.interfaces.${key}`,
          value: "",
          commit: (next) => {
            doc.tables = { ...object(doc.tables), interfaces: { base_oid: "1.3.6.1.2.1.2.2.1", ...object(object(doc.tables).interfaces), [key]: next } };
          }
        })),
        { label: text("domMetric"), path: `profile.tables.dom.metrics.${array(object(object(doc.tables).dom).metrics).length}`, value: arraySeed("profile.tables.dom.metrics", []), commit: (next) => {
          doc.tables = { ...object(doc.tables), dom: { base_oid: "", ...object(object(doc.tables).dom), metrics: [...array(object(object(doc.tables).dom).metrics), next] } };
        } }
      ];
      if (category === "hardware") return ["temperature", "power", "fan"].map((kind) => ({ label: fieldLabel(`profile.sensors.${kind}`), path: `profile.sensors.${kind}.${array(object(doc.sensors)[kind]).length}`, value: arraySeed(`profile.sensors.${kind}`, []), commit: (next) => {
        doc.sensors = { ...object(doc.sensors), [kind]: [...array(object(doc.sensors)[kind]), next] };
      } }));
      return [];
    };
    const render = () => {
      capture = () => {
      };
      stopOperationProgress();
      root.replaceChildren(element("h2", text("title")));
      if (busy) {
        stopOperationProgress = showOperationProgress(root, text(operation), () => {
          const seconds = Math.floor((performance.now() - operationStarted) / 1e3);
          return `${text(operation)}... ${localStorage.getItem("tracepulse-lang") === "ja" ? `\uFF08\u7D4C\u904E ${seconds}\u79D2\uFF09` : `(${seconds}s elapsed)`}`;
        });
      }
      const alert = element("p", error);
      alert.setAttribute("role", "alert");
      root.append(alert);
      const reload = button(text("reload"), () => {
        if (!dirty || window.confirm(text("discard"))) void run(load);
      });
      if (!view || !definitions) {
        reload.disabled = busy;
        root.append(reload);
        return;
      }
      const currentView = view, draft = definitions;
      const choices = new Map(view.profiles.map((profile2) => [profile2.id, profile2]));
      for (const [id, document2] of Object.entries(draft.profiles)) choices.set(id, { id, document: document2, source: "database", base: choices.get(id)?.base || null, resettable: true });
      if (selected !== "@common" && !choices.has(selected)) selected = "generic";
      const common = selected === "@common", profile = choices.get(selected);
      const doc = common ? { cpu_oid: draft.common.cpu_oid, memory_oid: draft.common.memory_oid, hardware_oids: draft.common.hardware_oids } : draft.profiles[selected] || clone(profile.document);
      const profileId = selected;
      capture = () => {
        if (common) draft.common = { cpu_oid: String(doc.cpu_oid), memory_oid: String(doc.memory_oid), hardware_oids: array(doc.hardware_oids).map(String) };
        else draft.profiles[profileId] = doc;
      };
      const toolbar = element("div");
      toolbar.className = "template-toolbar";
      if (dirty) toolbar.dataset.state = "dirty";
      const label = element("label", text("profile")), select = element("select");
      select.id = "template-profile";
      const deviceGroup = element("optgroup"), commonGroup = element("optgroup");
      deviceGroup.label = text("deviceProfiles");
      commonGroup.label = text("commonProfiles");
      commonGroup.append(new Option(text("common"), "@common"));
      for (const item of choices.values()) (selector(item.document) ? deviceGroup : commonGroup).append(new Option(`${item.document.name} (${item.id})`, item.id));
      select.append(deviceGroup, commonGroup);
      select.value = selected;
      select.onchange = () => {
        selected = select.value;
        render();
      };
      label.append(select);
      toolbar.append(label);
      const save = button(text("save"), () => void run(async () => {
        const next = await api("/api/snmp/definitions", { generation: currentView.generation, definitions: draft });
        view = next;
        definitions = clone(next.definitions);
        dirty = false;
        message = text("saved");
        savedMessage = true;
        flashGeneration = true;
        showSavedToast(next.applied_generation);
      }, "saving"));
      save.className = "template-save";
      toolbar.append(save);
      if (profile) {
        toolbar.append(button(text("reset"), () => {
          if (!window.confirm(text("resetConfirm"))) return;
          delete draft.profiles[selected];
          const original = view.profiles.find((item) => item.id === selected);
          if (profile.base && original) {
            original.document = clone(profile.base);
            original.source = "file";
          } else {
            view.profiles = view.profiles.filter((item) => item.id !== selected);
            selected = "generic";
          }
          capture = () => {
          };
          changed();
          render();
        }));
        toolbar.append(button(text("export"), () => void run(async () => {
          const data = await api("/api/snmp/definitions/convert", { document: doc });
          const url = URL.createObjectURL(new Blob([data.toml], { type: "application/toml" }));
          const link = element("a");
          link.href = url;
          link.download = `${selected.split(".").join("-")}.toml`;
          link.click();
          setTimeout(() => URL.revokeObjectURL(url), 1e3);
        }, "exporting")));
      }
      const file = element("input");
      file.type = "file";
      file.accept = ".toml";
      file.id = "template-import";
      file.hidden = true;
      file.onchange = () => {
        const selectedFile = file.files?.[0];
        if (!selectedFile) return;
        void run(async () => {
          const result = await api("/api/snmp/definitions/convert", { toml: await selectedFile.text() });
          const imported = result.document;
          const id = typeof imported.sysObjectID_prefix === "string" ? imported.sysObjectID_prefix.replace(/^\.|\.$/g, "") : Number(imported.enterprise_id) ? `1.3.6.1.4.1.${imported.enterprise_id}` : "generic";
          if (choices.has(id) && !window.confirm(`${text("import")}: ${id}?`)) return;
          draft.profiles[id] = imported;
          selected = id;
          capture = () => {
          };
          changed();
        }, "importing");
      };
      toolbar.append(file, button(text("import"), () => file.click()), reload);
      toolbar.append(button(text("newProfileTitle"), () => {
        const node = dialog(text("newProfileTitle")), form = element("form");
        form.append(element("p", text("profileGuide")));
        const nameLabel = element("label", text("profileName")), name = element("input");
        name.id = "template-new-name";
        name.required = true;
        name.placeholder = text("profileNameExample");
        nameLabel.append(name);
        const deviceLabel = element("label", text("registeredDevice")), deviceSelect = element("select");
        deviceSelect.id = "template-new-device";
        deviceSelect.append(new Option(text("chooseDevice"), ""));
        for (const device of devices) {
          deviceSelect.append(new Option(`${device.name || device.ip} (${device.ip}) - ${device.sys_object_id || text("unknownOid")}`, device.ip));
        }
        deviceLabel.append(deviceSelect);
        const ipLabel = element("label", text("probeIp")), ip = element("input");
        ip.id = "template-probe-ip";
        ip.placeholder = "192.0.2.10";
        ipLabel.append(ip);
        const communityLabel = element("label", text("community")), community = element("input");
        community.type = "password";
        community.autocomplete = "off";
        communityLabel.append(community);
        const newLabel = element("label", text("newProfile")), prefix = element("input");
        prefix.id = "template-new-prefix";
        prefix.placeholder = "1.3.6.1.4.1.12345";
        prefix.required = true;
        newLabel.append(prefix);
        const failure = element("p");
        failure.setAttribute("role", "alert");
        let fullOid = "";
        const scope = element("p"), probeStatus = element("p");
        probeStatus.setAttribute("role", "status");
        scope.setAttribute("aria-live", "polite");
        scope.id = "template-scope-preview";
        prefix.setAttribute("aria-describedby", scope.id);
        const vendorPrefix = () => /^1\.3\.6\.1\.4\.1\.\d+(?=\.|$)/.exec(fullOid)?.[0] || "";
        const updateScope = () => {
          scope.textContent = validOid(prefix.value) ? `${localStorage.getItem("tracepulse-lang") === "ja" ? `"${prefix.value}" ${text("scopePreview")}` : `${text("scopePreview")} "${prefix.value}".`} ${text("longestHint")}` : text("noSelector");
          vendor.disabled = !validOid(fullOid) || !vendorPrefix();
          model.disabled = !validOid(fullOid);
        };
        const useOid = (oid) => {
          prefix.value = oid.replace(/^\./, "");
          fullOid = prefix.value;
          modalDirty = true;
          updateScope();
        };
        const vendor = button(text("vendorScope"), () => {
          prefix.value = vendorPrefix();
          modalDirty = true;
          updateScope();
        });
        const model = button(text("modelScope"), () => {
          prefix.value = fullOid;
          modalDirty = true;
          updateScope();
        });
        const scopeActions = element("div");
        scopeActions.className = "template-scope-actions";
        scopeActions.append(vendor, model);
        prefix.oninput = () => {
          fullOid = prefix.value.replace(/^\./, "");
          modalDirty = true;
          updateScope();
        };
        deviceSelect.onchange = () => {
          const device = devices.find((device2) => device2.ip === deviceSelect.value);
          if (!device) return;
          ip.value = device.ip;
          modalDirty = true;
          useOid(device.sys_object_id || "");
        };
        let probing = false;
        const probe = button(text("probe"), () => {
          if (probing) return;
          probing = true;
          probe.disabled = true;
          create.disabled = true;
          failure.textContent = "";
          probeStatus.textContent = text("probing");
          const targetIp = ip.value.trim();
          void api("/api/snmp/probe-sysobjectid", { ip: targetIp, community: community.value }).then((result) => {
            if (!node.open) return;
            useOid(result.sys_object_id);
            const device = devices.find((device2) => device2.ip === targetIp);
            if (device) device.sys_object_id = result.sys_object_id;
            probeStatus.textContent = [result.sys_name, result.sys_object_id].filter(Boolean).join(" \u2014 ");
          }).catch((reason) => {
            if (node.open) failure.textContent = String(reason);
          }).finally(() => {
            probing = false;
            probe.disabled = false;
            create.disabled = false;
            if (node.open && failure.textContent) probeStatus.textContent = "";
          });
        });
        const baseLabel = element("label", text("baseTemplate")), base = element("select");
        base.id = "template-new-base";
        for (const item of choices.values()) base.append(new Option(String(item.document.name), item.id));
        base.append(new Option(text("emptyTemplate"), "@empty"));
        base.value = "generic";
        baseLabel.append(base);
        const create = button(text("create"), () => {
        });
        create.type = "submit";
        const actions = element("div");
        actions.className = "template-dialog-actions";
        actions.append(button(text("cancel"), () => node.close()), create);
        form.append(nameLabel, deviceLabel, ipLabel, communityLabel, probe, probeStatus, newLabel, scope, scopeActions, baseLabel, failure, actions);
        node.append(form);
        form.addEventListener("input", () => {
          modalDirty = true;
        });
        form.onsubmit = (event) => {
          event.preventDefault();
          if (probing) return;
          const id = prefix.value.trim().replace(/^\./, "");
          if (!validOid(id)) {
            failure.textContent = text("noSelector");
            return;
          }
          if (!name.value.trim()) {
            name.focus();
            return;
          }
          if (choices.has(id)) {
            failure.textContent = text("duplicateProfile");
            return;
          }
          const document2 = base.value === "@empty" ? { description: "", cpu: { candidates: [] }, memory: {}, sensors: {}, metrics: [], tables: {} } : clone(choices.get(base.value).document);
          const enterprise = /^1\.3\.6\.1\.4\.1\.(\d+)(?=\.|$)/.exec(id);
          draft.profiles[id] = { ...document2, name: name.value.trim(), sysObjectID_prefix: id, enterprise_id: enterprise ? Number(enterprise[1]) : 0 };
          dirty = true;
          message = "";
          savedMessage = false;
          capture = () => {
          };
          selected = id;
          node.close();
          render();
        };
        updateScope();
        node.showModal();
      }));
      root.append(toolbar);
      const statusBar = element("div");
      statusBar.className = "template-status-bar";
      const status = element("p", [dirty ? text("dirty") : "", savedMessage ? text("saved") : message].filter(Boolean).join(" \u2014 "));
      status.dataset.templateStatus = "";
      status.setAttribute("role", "status");
      if (dirty) status.dataset.state = "dirty";
      else if (savedMessage) status.dataset.state = "saved";
      const generation = element("p", `${text("version")}: ${view.generation} / ${view.applied_generation}`);
      generation.className = "template-generation";
      if (flashGeneration && !busy) {
        generation.classList.add("template-generation-flash");
        flashGeneration = false;
        window.setTimeout(() => generation.classList.remove("template-generation-flash"), 1500);
      }
      statusBar.append(status, generation);
      root.append(statusBar);
      {
        const matches = common ? devices : devices.filter((device) => selectedProfile(device.sys_object_id, choices) === selected);
        const count = element("dd", String(matches.length));
        count.dataset.templateMatches = "";
        count.title = matches.map((device) => `${device.name || device.ip} (${device.ip})`).join("\n");
        const condition = common ? text("commonCondition") : selector(doc) || text("fallbackCondition");
        const sourceInfo = element("dl");
        sourceInfo.className = "template-source";
        sourceInfo.append(element("dt", text("condition")), element("dd", condition), element("dt", text("matchingDevices")), count);
        if (profile) {
          const source = draft.profiles[selected] ? "database" : profile.source;
          const sources = { database: ["User definition (SQLite)", "\u30E6\u30FC\u30B6\u30FC\u5B9A\u7FA9\uFF08SQLite\uFF09"], bundled: ["Bundled template", "\u540C\u68B1\u30C6\u30F3\u30D7\u30EC\u30FC\u30C8"], byo: ["External TOML template", "\u5916\u90E8TOML\u30C6\u30F3\u30D7\u30EC\u30FC\u30C8"], file: ["Original template", "\u5143\u306E\u30C6\u30F3\u30D7\u30EC\u30FC\u30C8"] };
          sourceInfo.dataset.templateSource = source;
          sourceInfo.prepend(element("dt", text("source")), element("dd", sources[source]?.[localStorage.getItem("tracepulse-lang") === "ja" ? 1 : 0] || source));
        }
        root.append(sourceInfo);
      }
      const categories = common ? [["system", "systemTab"], ["hardware", "hardwareTab"]] : [["system", "systemTab"], ["interfaces", "interfacesTab"], ["hardware", "hardwareTab"], ["profile", "profileTab"]];
      if (!categories.some(([key]) => key === category)) category = "system";
      const tabs = element("div");
      tabs.className = "template-tabs";
      tabs.setAttribute("role", "tablist");
      tabs.setAttribute("aria-label", text("title"));
      categories.forEach(([key, title], index) => {
        const tab = button(text(title), () => {
          category = key;
          render();
          root.querySelector(`#template-tab-${key}`)?.focus();
        });
        tab.id = `template-tab-${key}`;
        tab.setAttribute("role", "tab");
        tab.setAttribute("aria-selected", String(category === key));
        tab.setAttribute("aria-controls", "template-panel");
        tab.tabIndex = category === key ? 0 : -1;
        tab.onkeydown = (event) => {
          const next = event.key === "ArrowRight" ? (index + 1) % categories.length : event.key === "ArrowLeft" ? (index + categories.length - 1) % categories.length : event.key === "Home" ? 0 : event.key === "End" ? categories.length - 1 : -1;
          if (next >= 0) {
            event.preventDefault();
            category = categories[next][0];
            render();
            root.querySelector(`#template-tab-${category}`)?.focus();
          }
        };
        tabs.append(tab);
      });
      root.append(tabs);
      const panel = element("div");
      panel.id = "template-panel";
      panel.dataset.templateCategory = category;
      panel.setAttribute("role", "tabpanel");
      panel.setAttribute("aria-labelledby", `template-tab-${category}`);
      const listActions = element("div");
      listActions.className = "template-list-actions";
      const settings = (title, path, section, excluded, commit) => {
        listActions.append(button(text(title), () => editRow({ label: text(title), path, value: Object.fromEntries(Object.entries(section).filter(([key]) => !excluded.includes(key))), commit: (next) => commit(object(next)) })));
      };
      if (!common && category === "system") settings("cpuSettings", "profile.cpu", { aggregation: "first", ...object(doc.cpu) }, ["candidates"], (next) => {
        doc.cpu = { ...next, candidates: array(object(doc.cpu).candidates) };
      });
      if (!common && category === "interfaces") {
        settings("interfaceSettings", "profile.tables.interfaces", { base_oid: "1.3.6.1.2.1.2.2.1", ...object(object(doc.tables).interfaces) }, ["index", "name", "in_octets", "out_octets"], (next) => {
          doc.tables = { ...object(doc.tables), interfaces: { ...object(object(doc.tables).interfaces), ...next } };
        });
        settings("domSettings", "profile.tables.dom", { base_oid: "", ...object(object(doc.tables).dom) }, ["metrics"], (next) => {
          doc.tables = { ...object(doc.tables), dom: { ...next, metrics: array(object(object(doc.tables).dom).metrics) } };
        });
      }
      const additions = additionsFor(doc, common);
      if (category !== "profile") {
        const add = button(text("addItem"), () => {
          const node = dialog(text("addItem")), label2 = element("label", text("itemType")), kind = element("select");
          kind.setAttribute("aria-label", text("itemType"));
          additions.forEach((row, index) => kind.append(new Option(row.label, String(index))));
          label2.append(kind);
          node.append(label2, button(text("cancel"), () => node.close()), button(text("add"), () => editRow(additions[Number(kind.value)])));
          node.showModal();
        });
        add.disabled = !additions.length;
        listActions.append(add);
      }
      panel.append(listActions);
      const scroll = element("div");
      scroll.className = "template-table-scroll";
      scroll.tabIndex = 0;
      scroll.setAttribute("role", "region");
      scroll.setAttribute("aria-label", text(categories.find(([key]) => key === category)[1]));
      const table = element("table");
      table.className = "template-items";
      table.id = "template-items";
      const head = element("thead"), header = element("tr");
      for (const key of ["item", "oids", "configuration", "edit"]) {
        const cell = element("th", text(key));
        cell.scope = "col";
        header.append(cell);
      }
      head.append(header);
      table.append(head);
      const body = element("tbody");
      const rows = rowsFor(doc, common);
      for (const row of rows) {
        const entry = element("tr");
        entry.dataset.templateRow = row.path;
        entry.append(element("td", row.label));
        const oidCell = element("td"), oids = configuredOids(row.value);
        if (oids.length) for (const oid of oids) {
          const code = element("code", oid);
          code.className = "template-oid";
          oidCell.append(code);
        }
        else oidCell.textContent = row.configured ? String(object(row.value).description || object(row.value).name || "\u2014") : "\u2014";
        entry.append(oidCell, element("td", text(row.configured || oids.length ? "configured" : "unconfigured")));
        const controls = element("td"), actions = element("div");
        actions.className = "template-row-actions";
        const edit = button(text("edit"), () => editRow(row));
        edit.setAttribute("aria-label", `${text("edit")}: ${row.label}`);
        actions.append(edit);
        if (row.remove) actions.append(button(text("remove"), () => {
          if (window.confirm(text("confirmRemove"))) {
            row.remove();
            changed();
            render();
          }
        }));
        controls.append(actions);
        entry.append(controls);
        body.append(entry);
      }
      if (!rows.length) {
        const empty = element("tr"), cell = element("td", text("empty"));
        cell.colSpan = 4;
        empty.append(cell);
        body.append(empty);
      }
      table.append(body);
      scroll.append(table);
      panel.append(scroll);
      root.append(panel);
      const preview = element("details");
      preview.append(element("summary", text("preview")));
      const previewLabel = element("label", text("sysObject")), sysObject = element("input");
      sysObject.id = "template-preview-oid";
      previewLabel.append(sysObject);
      preview.append(previewLabel, button(text("preview"), () => {
        const oid = sysObject.value;
        void run(async () => {
          const result = await api("/api/snmp/definitions/preview", { definitions: draft, sys_object_id: oid });
          message = `${text("preview")}: ${result.document.name} (${result.id})`;
          savedMessage = false;
        }, "previewing");
      }));
      const notes = element("details");
      notes.append(element("summary", text("notes")), element("p", text("help")), element("p", text("storage")), element("p", text("disclaimer")));
      root.append(preview, notes);
      if (busy) root.querySelectorAll("input,button,select").forEach((node) => {
        node.disabled = true;
      });
    };
    window.addEventListener("tracepulse-language-change", () => {
      if (!activeDialog?.open) render();
    });
    window.addEventListener("beforeunload", (event) => {
      if (dirty || modalDirty) {
        event.preventDefault();
        event.returnValue = "";
      }
    });
    void run(load);
  }

  // frontend/src/shared/escape.ts
  function esc(value) {
    return String(value === null || value === void 0 ? "" : value).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/\x22/g, "&quot;").replace(/\x27/g, "&#39;");
  }

  // frontend/src/shared/secret-visibility.ts
  var eye = `<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true" focusable="false"><path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12Z"/><circle cx="12" cy="12" r="3"/><path class="secret-eye-slash" d="m3 3 18 18"/></svg>`;
  function addSecretVisibility(input) {
    if (input.parentElement?.classList.contains("secret-input")) return;
    if (!document.getElementById("secret-visibility-style")) {
      const style = document.createElement("style");
      style.id = "secret-visibility-style";
      style.textContent = `
            .secret-input{position:relative;display:block;min-width:0}
            .secret-input input{padding-right:44px!important}
            .secret-input .secret-toggle{position:absolute;right:2px;top:2px;bottom:2px;width:40px;padding:0;display:flex;align-items:center;justify-content:center;border:0;border-radius:4px;background:transparent;color:inherit;cursor:pointer}
            .secret-toggle:focus-visible{outline:2px solid #3b82f6;outline-offset:-2px}
            .secret-toggle:hover{background:rgb(127 127 127 / .15)}
            .secret-toggle .secret-eye-slash{display:none}
            .secret-toggle[aria-pressed=true] .secret-eye-slash{display:block}
        `;
      document.head.append(style);
    }
    const wrapper = document.createElement("span");
    wrapper.className = "secret-input";
    const button2 = document.createElement("button");
    button2.type = "button";
    button2.className = "secret-toggle";
    button2.setAttribute("aria-controls", input.id);
    button2.innerHTML = eye;
    const update = () => {
      const visible = input.type === "text";
      const key = visible ? "hide_community" : "show_community";
      const fallback = visible ? "Hide community" : "Show community";
      const label = window.t?.(key) || fallback;
      button2.setAttribute("aria-label", label);
      button2.setAttribute("aria-pressed", String(visible));
      button2.title = label;
    };
    button2.addEventListener("click", () => {
      input.type = input.type === "password" ? "text" : "password";
      update();
    });
    input.before(wrapper);
    wrapper.append(input, button2);
    update();
  }
  window.addEventListener("tracepulse-language-change", () => {
    document.querySelectorAll(".secret-toggle").forEach((button2) => {
      const key = button2.getAttribute("aria-pressed") === "true" ? "hide_community" : "show_community";
      const label = window.t?.(key);
      if (label) {
        button2.setAttribute("aria-label", label);
        button2.title = label;
      }
    });
  });

  // frontend/src/shared/snmp-credentials.ts
  var japanese = () => document.documentElement.lang === "ja";
  var text2 = (key, fallback) => window.t?.(key) || fallback;
  var credentialTitle = () => text2("snmp_credential_title", japanese() ? "SNMP\u8A8D\u8A3C\u60C5\u5831" : "SNMP credentials");
  function credentialFields(prefix, global = false, includeCommunity = true) {
    if (!document.getElementById("snmp-credential-styles")) {
      const style = document.createElement("style");
      style.id = "snmp-credential-styles";
      style.textContent = `.snmp-credential-fields{display:grid;gap:12px;min-width:0;border:0;padding:0;margin:12px 0}
            .snmp-credential-fields label{display:grid;gap:6px;font-size:.88rem;min-width:0}
            .snmp-credential-fields input,.snmp-credential-fields select{box-sizing:border-box;width:100%;min-width:0;padding:8px;border:1px solid var(--tp-border,#475569);border-radius:4px;background:var(--tp-bg,#0f172a);color:var(--tp-text,#e2e8f0)}
            .snmp-credential-fields p{margin:0;font-size:.82rem;overflow-wrap:anywhere}
            #snmp-connection-form{max-width:680px}#snmp-connection-form .field-row{grid-template-columns:1fr}
            #snmp-connection-form .secret-input input{box-sizing:border-box;width:100%}
            #manual-box .snmp-credential-fields{flex:1 1 100%}
            html[data-theme=light] .snmp-credential-fields input,html[data-theme=light] .snmp-credential-fields select{background:#fff;color:#0f172a;border-color:#cbd5e1}`;
      document.head.append(style);
    }
    const hintKey = global ? "snmp_credential_keep_hint" : "snmp_credential_inherit_hint";
    const hint = text2(hintKey, global ? "Leave blank to keep existing defaults." : "Blank fields inherit global defaults. Saving replaces the current device overrides.");
    const optionKey = global ? "snmp_credential_unchanged" : "snmp_credential_global";
    const label = (key, fallback) => `<span data-i18n="snmp_credential_${key}">${esc(text2(`snmp_credential_${key}`, fallback))}</span>`;
    const input = (key, fallback) => `<label for="${prefix}-${key}">${label(key, fallback)}<input id="${prefix}-${key}" data-snmp="${key}" type="password" autocomplete="new-password" maxlength="1024" placeholder="${esc(hint)}" data-i18n-placeholder="${hintKey}" aria-describedby="${prefix}-hint"></label>`;
    const protocolLabels = { "2c": "SNMPv2c", "3": "SNMPv3", sha1: "SHA-1", sha256: "SHA-256", sha512: "SHA-512", aes128: "AES-128" };
    const select = (key, fallback, options) => `<label for="${prefix}-${key}">${label(key, fallback)}<select id="${prefix}-${key}" data-snmp="${key}"><option value="" data-i18n="${optionKey}">${esc(text2(optionKey, global ? "Unchanged" : "Global default"))}</option>${options.map((value) => value === "none" ? `<option value="none" data-i18n="snmp_credential_none">${esc(text2("snmp_credential_none", "None"))}</option>` : `<option value="${value}">${esc(protocolLabels[value] || value)}</option>`).join("")}</select></label>`;
    return `<fieldset class="snmp-credential-fields">
        <legend data-i18n="snmp_credential_title">${esc(credentialTitle())}</legend><p id="${prefix}-hint" data-i18n="${hintKey}">${esc(hint)}</p>
        ${select("version", "SNMP version", ["2c", "3"])}
        ${includeCommunity ? input("community", "Community (SNMPv2c)") : ""}
        ${input("v3_username", "v3 username")}
        ${select("v3_auth_protocol", "v3 authentication", ["sha256", "sha512", "sha1", "none"])}
        ${input("v3_auth_password", "v3 authentication password")}
        ${select("v3_priv_protocol", "v3 privacy", ["aes128", "none"])}
        ${input("v3_priv_password", "v3 privacy password")}
    </fieldset>`;
  }
  function readCredentials(container) {
    const credentials = {};
    container.querySelectorAll("[data-snmp]").forEach((input) => {
      if (input.value) credentials[input.dataset.snmp] = input.value;
    });
    return credentials;
  }
  function clearCredentials(container) {
    container.querySelectorAll("[data-snmp]").forEach((input) => {
      input.value = "";
    });
  }
  function initializeGlobalSnmp() {
    const form = document.getElementById("snmp-connection-form");
    if (!(form instanceof HTMLFormElement)) return;
    const community = form.querySelector("#community");
    community.dataset.snmp = "community";
    addSecretVisibility(community);
    const button2 = form.querySelector("[type=submit]");
    button2.insertAdjacentHTML("beforebegin", credentialFields("global-snmp", true, false));
    form.onsubmit = async (event) => {
      event.preventDefault();
      if (button2.disabled) return;
      const credentials = readCredentials(form);
      const { community: defaultCommunity, ...v3Credentials } = credentials;
      const payload = { default_community: defaultCommunity || "", ...Object.keys(v3Credentials).length ? { snmp_credentials: v3Credentials } : {} };
      button2.disabled = true;
      const controls = form.querySelectorAll("input,select");
      controls.forEach((control) => {
        control.disabled = true;
      });
      const status = form.querySelector("[role=status]");
      const error = form.querySelector("[role=alert]");
      status.textContent = "";
      error.textContent = "";
      try {
        const response = await fetch("/api/settings", { method: "POST", headers: { "Content-Type": "application/json", ...window.tracepulseRequestHeaders?.() }, body: JSON.stringify(payload) });
        const result = await response.json();
        if (!response.ok || !result.ok) throw new Error(result.error || `HTTP ${response.status}`);
        clearCredentials(form);
        if (community.type === "text") form.querySelector("button[aria-controls=community]")?.click();
        status.textContent = window.t?.("settings_saved") || "Saved";
      } catch (reason) {
        error.textContent = String(reason);
      } finally {
        button2.disabled = false;
        controls.forEach((control) => {
          control.disabled = false;
        });
      }
    };
  }

  // frontend/src/pages/snmp-definitions.ts
  initializeTemplateManager();
  initializeGlobalSnmp();
})();
