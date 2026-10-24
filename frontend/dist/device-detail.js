"use strict";
var TracePulseDeviceDetail = (() => {
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

  // frontend/src/pages/device-detail.ts
  var device_detail_exports = {};
  __export(device_detail_exports, {
    isInterfaceSelected: () => isInterfaceSelected,
    loadDeviceDetail: () => loadDeviceDetail,
    onErrorBreakdownIfaceChange: () => onErrorBreakdownIfaceChange,
    refreshTrafficProtocols: () => refreshTrafficProtocols,
    renderAlerts: () => renderAlerts,
    renderBwChart: () => renderBwChart,
    renderCpuChart: () => renderCpuChart,
    renderDetail: () => renderDetail,
    renderErrChart: () => renderErrChart,
    renderErrorBreakdownCard: () => renderErrorBreakdownCard,
    renderHardwareStatus: () => renderHardwareStatus,
    renderInterfaces: () => renderInterfaces,
    renderMemoryChart: () => renderMemoryChart,
    renderSpikes: () => renderSpikes,
    renderSysChart: () => renderSysChart,
    renderTemplateMetrics: () => renderTemplateMetrics,
    renderTrafficProtocols: () => renderTrafficProtocols,
    selectAllInterfaces: () => selectAllInterfaces,
    sparkline: () => sparkline,
    toggleInterfaceSelection: () => toggleInterfaceSelection
  });

  // frontend/src/shared/escape.ts
  function esc(value) {
    return String(value === null || value === void 0 ? "" : value).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/\x22/g, "&quot;").replace(/\x27/g, "&#39;");
  }

  // frontend/src/shared/device-metadata.ts
  if (!document.getElementById("metadata-styles")) {
    const style = document.createElement("style");
    style.id = "metadata-styles";
    style.textContent = `
        .metadata-dialog{box-sizing:border-box;width:min(640px,calc(100% - 24px));max-height:calc(100dvh - 24px);overflow:auto;margin:auto;padding:24px;border:1px solid var(--tp-border,#475569);border-radius:8px;background:var(--tp-surface,#1e293b);color:var(--tp-text,#e2e8f0)}
        .metadata-dialog::backdrop{background:rgb(0 0 0 / .6)}
        .metadata-dialog form,.metadata-dialog label{display:grid;gap:8px}
        .metadata-dialog p,.metadata-dialog li{overflow-wrap:anywhere}
        .metadata-dialog input,.metadata-dialog select,.device-toolbar input{box-sizing:border-box;width:100%;padding:8px;background:var(--tp-bg,#0f172a);color:inherit;border:1px solid #64748b;border-radius:4px;min-width:0}
        .metadata-dialog button,.device-toolbar button,#edit-device-metadata{padding:8px 12px;cursor:pointer;background:var(--tp-bg,#0f172a);color:inherit;border:1px solid #64748b;border-radius:4px}
        .metadata-dialog button:disabled,.device-toolbar button:disabled{opacity:.5;cursor:default}
        .metadata-actions,.device-toolbar,.device-tags{display:flex;gap:8px;flex-wrap:wrap;align-items:center}
        .metadata-actions{justify-content:flex-end;margin-top:12px}.device-toolbar{margin:12px 0}.device-toolbar input{flex:1 1 220px}
        .device-tag{display:inline-flex;align-items:center;gap:4px;padding:2px 7px;background:var(--tp-bg,#0f172a);border:1px solid #64748b;border-radius:12px;font-size:.8rem;overflow-wrap:anywhere}
        .device-tags{margin-top:4px}.device-tag button{padding:0 4px}.device-original{font-size:.8rem;color:var(--tp-muted,#94a3b8);overflow-wrap:anywhere}
        .metadata-dialog [role=alert],.device-toolbar [role=alert]{color:#f87171}
        :root[data-theme=light] .metadata-dialog{background:#fff;color:#1e293b}
        :root[data-theme=light] .metadata-dialog input,:root[data-theme=light] .metadata-dialog select,:root[data-theme=light] .metadata-dialog button,:root[data-theme=light] .device-toolbar input,:root[data-theme=light] .device-toolbar button,:root[data-theme=light] .device-tag,:root[data-theme=light] #edit-device-metadata{background:#f1f5f9;color:#1e293b}
    `;
    document.head.append(style);
  }
  function deviceLabel(device) {
    if (device.display_name?.trim()) return device.display_name;
    if (device.name?.trim()) return device.name;
    return device.ip || "";
  }
  var copy = {
    en: {
      edit: "Edit alias and tags",
      alias: "Display name",
      tags: "Tags",
      add: "Add tag",
      remove: "Remove",
      cancel: "Cancel",
      save: "Save",
      rename: "Bulk rename",
      prefix: "Prefix",
      start: "Start number",
      padding: "Padding",
      suffix: "Suffix",
      order: "Sort order",
      ip: "IP ascending",
      selection: "Selection order",
      preview: "Preview",
      apply: "Apply names",
      search: "Search alias, hostname, IP or tags",
      select: "Select device",
      all: "Select visible devices",
      selected: "selected",
      failed: "Request failed",
      missing: "Select at least one device",
      tagHint: "Enter a tag, then press Add tag or Enter.",
      clear: "Leave the display name empty to use sysName or IP."
    },
    ja: {
      edit: "\u8868\u793A\u540D\u30FB\u30BF\u30B0\u3092\u7DE8\u96C6",
      alias: "\u8868\u793A\u540D",
      tags: "\u30BF\u30B0",
      add: "\u30BF\u30B0\u3092\u8FFD\u52A0",
      remove: "\u524A\u9664",
      cancel: "\u30AD\u30E3\u30F3\u30BB\u30EB",
      save: "\u4FDD\u5B58",
      rename: "\u4E00\u62EC\u30EA\u30CD\u30FC\u30E0\uFF08\u9023\u756A\u4ED8\u4E0E\uFF09",
      prefix: "\u63A5\u982D\u8F9E",
      start: "\u958B\u59CB\u756A\u53F7",
      padding: "\u30BC\u30ED\u57CB\u3081\u6841\u6570",
      suffix: "\u63A5\u5C3E\u8F9E",
      order: "\u4E26\u3073\u9806",
      ip: "IP\u30A2\u30C9\u30EC\u30B9\u6607\u9806",
      selection: "\u9078\u629E\u9806",
      preview: "\u30D7\u30EC\u30D3\u30E5\u30FC",
      apply: "\u540D\u79F0\u3092\u9069\u7528",
      search: "\u8868\u793A\u540D\u30FB\u30DB\u30B9\u30C8\u540D\u30FBIP\u30FB\u30BF\u30B0\u3067\u691C\u7D22",
      select: "\u6A5F\u5668\u3092\u9078\u629E",
      all: "\u8868\u793A\u4E2D\u306E\u6A5F\u5668\u3092\u3059\u3079\u3066\u9078\u629E",
      selected: "\u4EF6\u9078\u629E",
      failed: "\u51E6\u7406\u306B\u5931\u6557\u3057\u307E\u3057\u305F",
      missing: "\u6A5F\u5668\u3092\u9078\u629E\u3057\u3066\u304F\u3060\u3055\u3044",
      tagHint: "\u30BF\u30B0\u3092\u5165\u529B\u3057\u3001\u8FFD\u52A0\u30DC\u30BF\u30F3\u307E\u305F\u306FEnter\u3067\u78BA\u5B9A\u3057\u307E\u3059\u3002",
      clear: "\u8868\u793A\u540D\u3092\u7A7A\u306B\u3059\u308B\u3068sysName\u307E\u305F\u306FIP\u3092\u8868\u793A\u3057\u307E\u3059\u3002"
    }
  };
  function metadataText(key) {
    return copy[document.documentElement.lang === "ja" ? "ja" : "en"][key];
  }
  async function post(url, body) {
    const response = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json", "X-CSRF-Token": window.tracepulseCsrf || "" },
      body: JSON.stringify(body)
    });
    const data = await response.json();
    if (!response.ok || data.error || data.ok !== true) throw new Error(data.error || metadataText("failed"));
    return data;
  }
  function dialog(id, title) {
    const existing = document.getElementById(id);
    if (existing instanceof HTMLDialogElement) existing.remove();
    const element = document.createElement("dialog");
    element.id = id;
    element.className = "metadata-dialog";
    element.setAttribute("aria-labelledby", `${id}-title`);
    element.innerHTML = `<h2 id="${id}-title">${esc(title)}</h2>`;
    document.body.append(element);
    return element;
  }
  function field(id, label, value, type = "text", limits = "") {
    return `<label for="${id}">${esc(label)}<input id="${id}" type="${type}" value="${esc(value)}" ${limits}></label>`;
  }
  function editDeviceMetadata(device, saved) {
    if (!device.ip) return;
    const element = dialog("device-metadata-dialog", metadataText("edit"));
    let tags = [...device.tags || []];
    element.insertAdjacentHTML("beforeend", `<form>
        <p>${esc(device.name || "")} (${esc(device.ip)})</p>
        ${field("device-alias", metadataText("alias"), device.display_name || "", "text", "maxlength=512")}
        <p>${esc(metadataText("clear"))}</p>
        <label for="device-tag-input">${esc(metadataText("tags"))}</label>
        <div id="device-tag-chips" class="device-tags"></div>
        <input id="device-tag-input" maxlength="64" aria-describedby="device-tag-hint">
        <p id="device-tag-hint">${esc(metadataText("tagHint"))}</p>
        <button type="button" id="device-add-tag">${esc(metadataText("add"))}</button>
        <p role="alert"></p><div class="metadata-actions">
        <button type="button" data-cancel>${esc(metadataText("cancel"))}</button>
        <button type="submit">${esc(metadataText("save"))}</button></div></form>`);
    const input = element.querySelector("#device-tag-input");
    const error = element.querySelector("[role=alert]");
    const renderTags = () => {
      const chips = element.querySelector("#device-tag-chips");
      chips.replaceChildren();
      tags.forEach((tag) => {
        const chip = document.createElement("span");
        chip.className = "device-tag";
        chip.append(document.createTextNode(tag));
        const remove = document.createElement("button");
        remove.type = "button";
        remove.textContent = "\xD7";
        remove.setAttribute("aria-label", `${metadataText("remove")} ${tag}`);
        remove.onclick = () => {
          tags = tags.filter((value) => value !== tag);
          renderTags();
        };
        chip.append(remove);
        chips.append(chip);
      });
    };
    const addTag = () => {
      const tag = input.value.trim();
      if (tag && !tags.includes(tag)) tags.push(tag);
      input.value = "";
      renderTags();
    };
    element.querySelector("#device-add-tag").onclick = addTag;
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter" && !event.isComposing) {
        event.preventDefault();
        addTag();
      }
    });
    element.querySelector("[data-cancel]").onclick = () => element.close();
    element.querySelector("form").onsubmit = async (event) => {
      event.preventDefault();
      addTag();
      const submit = element.querySelector("[type=submit]");
      submit.disabled = true;
      error.textContent = "";
      try {
        await post(`/api/device/${encodeURIComponent(device.ip)}/metadata`, {
          display_name: element.querySelector("#device-alias").value || null,
          tags
        });
        element.close();
        saved();
      } catch (failure) {
        error.textContent = String(failure);
      } finally {
        submit.disabled = false;
      }
    };
    renderTags();
    element.showModal();
  }

  // frontend/src/shared/format.ts
  function formatBps(value) {
    const numericValue = Number(value || 0);
    if (numericValue >= 1e9) return `${(numericValue / 1e9).toFixed(1)} Gbps`;
    if (numericValue >= 1e6) return `${(numericValue / 1e6).toFixed(1)} Mbps`;
    if (numericValue >= 1e3) return `${(numericValue / 1e3).toFixed(1)} Kbps`;
    return `${Math.round(numericValue)} bps`;
  }
  function fmtNum(value) {
    return new Intl.NumberFormat("en-US").format(Number(value || 0));
  }
  function parseTracePulseTime(iso) {
    if (typeof iso !== "string" || iso.length === 0) return /* @__PURE__ */ new Date(NaN);
    if (/Z$|[+-]\d\d:\d\d$/.test(iso)) return new Date(iso);
    return /* @__PURE__ */ new Date(`${iso.replace(" ", "T")}Z`);
  }
  function tracePulseTimeZone() {
    try {
      return localStorage.getItem("tracepulse-timezone") === "jst" ? "Asia/Tokyo" : "UTC";
    } catch {
      return "UTC";
    }
  }
  function formatTracePulseTime(value) {
    const parts = new Intl.DateTimeFormat("en-US", {
      timeZone: tracePulseTimeZone(),
      month: "numeric",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false
    }).formatToParts(value);
    const map = {};
    parts.forEach((part) => {
      if (part.type !== "literal") map[part.type] = part.value;
    });
    return `${map.month}/${map.day} ${map.hour}:${map.minute}:${map.second}`;
  }
  function formatTracePulseClock(value) {
    const parts = new Intl.DateTimeFormat("en-US", {
      timeZone: tracePulseTimeZone(),
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false
    }).formatToParts(value);
    const map = {};
    parts.forEach((part) => {
      if (part.type !== "literal") map[part.type] = part.value;
    });
    return `${map.hour}:${map.minute}:${map.second}`;
  }
  function fmtTime(iso) {
    if (!iso) return "-";
    const date = parseTracePulseTime(iso);
    return Number.isNaN(date.getTime()) ? String(iso) : formatTracePulseTime(date);
  }

  // frontend/src/pages/device-detail.ts
  var palette = ["#38bdf8", "#4ade80", "#f59e0b", "#f87171", "#a78bfa", "#34d399"];
  var chartColors = ["#38bdf8", "#4ade80", "#f59e0b", "#f87171", "#a78bfa", "#34d399", "#fb923c", "#e879f9"];
  var analyticsRefreshOptions = [5, 10, 30, 60];
  var analyticsPollingMs = loadAnalyticsPollingMs();
  var deviceRefreshSeconds = 30;
  var errorBreakdownSelectedIf = null;
  var analyticsWindow = "60s";
  var analyticsPaused = false;
  var analyticsTimer = null;
  var deviceRefreshTimer = null;
  var ifaceSelected = /* @__PURE__ */ new Set();
  var lastDeviceDetail = null;
  var ifaceSelectionReady = false;
  var resizeTimer = null;
  function loadAnalyticsPollingMs() {
    try {
      const seconds = Number(localStorage.getItem("tracepulse-traffic-refresh-seconds"));
      return analyticsRefreshOptions.includes(seconds) ? seconds * 1e3 : 5e3;
    } catch {
      return 5e3;
    }
  }
  function saveAnalyticsPollingMs(value) {
    try {
      localStorage.setItem("tracepulse-traffic-refresh-seconds", String(value / 1e3));
    } catch {
    }
  }
  function t(key) {
    return typeof window.t === "function" ? window.t(key) : key;
  }
  function ifaceSelectionKey() {
    return `tracepulse-iface-selection:${window.DEVICE_IP || ""}`;
  }
  function saveSelectedInterfaces() {
    try {
      localStorage.setItem(ifaceSelectionKey(), JSON.stringify(Array.from(ifaceSelected)));
    } catch {
    }
  }
  function syncSelectedInterfaces(ifaces) {
    const known = new Set(ifaces.map((iface) => String(iface.if_index)));
    if (!ifaceSelectionReady) {
      let stored = null;
      try {
        stored = JSON.parse(localStorage.getItem(ifaceSelectionKey()) || "null");
      } catch {
        stored = null;
      }
      ifaceSelected = /* @__PURE__ */ new Set();
      if (Array.isArray(stored) && stored.length > 0) {
        stored.forEach((value) => {
          const key = String(value);
          if (known.has(key)) ifaceSelected.add(key);
        });
      }
      if (ifaceSelected.size === 0) {
        known.forEach((value) => ifaceSelected.add(value));
      }
      ifaceSelectionReady = true;
    } else {
      Array.from(ifaceSelected).forEach((value) => {
        if (!known.has(String(value))) ifaceSelected.delete(value);
      });
    }
    saveSelectedInterfaces();
  }
  function isInterfaceSelected(ifIndex) {
    return ifaceSelected.has(String(ifIndex));
  }
  function selectAllInterfaces(checked) {
    const ifaces = lastDeviceDetail?.interfaces || [];
    ifaceSelected = /* @__PURE__ */ new Set();
    if (checked) {
      ifaces.forEach((iface) => ifaceSelected.add(String(iface.if_index)));
    }
    saveSelectedInterfaces();
    if (lastDeviceDetail) renderDetail(lastDeviceDetail);
  }
  function toggleInterfaceSelection(ifIndex, checked) {
    const key = String(ifIndex);
    if (checked) {
      ifaceSelected.add(key);
    } else {
      ifaceSelected.delete(key);
    }
    saveSelectedInterfaces();
    if (lastDeviceDetail) renderDetail(lastDeviceDetail);
  }
  function renderInterfaceSelection(ifaces = []) {
    const box = document.getElementById("iface-filter");
    const note = document.getElementById("iface-filter-note");
    if (!box) return;
    box.innerHTML = ifaces.map((iface) => {
      const id = `iface-select-${iface.if_index}`;
      const label = iface.if_name || `if-${iface.if_index}`;
      const checked = isInterfaceSelected(iface.if_index) ? " checked" : "";
      return `<label class='iface-filter-item' for='${id}'><input id='${id}' type='checkbox' value='${iface.if_index}'${checked} onchange='toggleInterfaceSelection(this.value,this.checked)'><span>${esc(label)} (if-${iface.if_index})</span></label>`;
    }).join("");
    if (note) {
      note.textContent = ifaces.length ? `${Array.from(ifaceSelected).length} / ${ifaces.length} ${t("selected")}` : `0 / 0 ${t("selected")}`;
    }
  }
  function includeTrafficItem(item) {
    if (!item) return false;
    if (!Object.prototype.hasOwnProperty.call(item, "if_index")) return true;
    if (typeof window.isInterfaceSelected !== "function") return true;
    return window.isInterfaceSelected(item.if_index);
  }
  function trafficProtocolsEmptyHtml() {
    return `<p class='traffic-empty'>${t("traffic_protocols_empty")}</p>`;
  }
  function renderTrafficProtocols(data) {
    const box = document.getElementById("traffic-protocols");
    const section = document.getElementById("traffic-protocols-section");
    if (!box || !section) return;
    const allProtocols = Array.isArray(data?.protocols) ? data.protocols : [];
    const allTalkers = Array.isArray(data?.top_talkers) ? data.top_talkers : [];
    section.style.display = allProtocols.length || allTalkers.length ? "block" : "none";
    if (section.style.display === "none") return;
    const protocols = allProtocols.filter(includeTrafficItem);
    const talkers = allTalkers.filter(includeTrafficItem);
    if (protocols.length === 0 && talkers.length === 0) {
      box.innerHTML = trafficProtocolsEmptyHtml();
      return;
    }
    let cursor = 0;
    const stops = protocols.map((item, index) => {
      const start = cursor;
      cursor += Number(item.percentage || 0);
      return `${palette[index % palette.length]} ${start}% ${cursor}%`;
    });
    const legend = protocols.map((item, index) => {
      const color = palette[index % palette.length];
      const percent = Number(item.percentage || 0).toFixed(1);
      return `<div class='traffic-legend-item'><span class='traffic-legend-swatch' style='background:${color}'></span>${esc(item.protocol)} ${percent}% (${formatBps(item.bps)})</div>`;
    }).join("");
    const rows = talkers.map((item) => {
      const flags = (item.tcp_flags || []).join(", ");
      const ingress = item.ingress_if_index === null || item.ingress_if_index === void 0 ? "-" : item.ingress_if_index;
      const egress = item.egress_if_index === null || item.egress_if_index === void 0 ? "-" : item.egress_if_index;
      return `<tr><td>${esc(item.source_ip)}:${item.source_port || 0}</td><td>${esc(item.destination_ip)}:${item.destination_port || 0}</td><td>${esc(item.protocol)}</td><td>${esc(item.app_name || "-")}</td><td>${formatBps(item.bps)}</td><td>${formatBps(item.pps)}</td><td>${esc(flags || "-")}</td><td>${esc(item.ingress_if_name || `if-${ingress}`)}</td><td>${esc(item.egress_if_name || `if-${egress}`)}</td></tr>`;
    }).join("");
    const summary = data?.summary || {};
    const applications = (data?.applications || []).map((item) => {
      const percentage = Number(item.percentage || 0);
      return `<div class='traffic-share-item'><div class='traffic-share-label'><span>${esc(item.app_name)}</span><span>${percentage.toFixed(1)}% \xB7 ${formatBps(item.bps)}</span></div><div class='traffic-share-track'><span style='width:${Math.min(100, Math.max(0, percentage))}%'></span></div></div>`;
    }).join("");
    const endpoints = (items) => items.map((item) => `<div class='traffic-legend-item'><span>${esc(item.ip)}</span><span>${Number(item.percentage || 0).toFixed(1)}% \xB7 ${formatBps(item.bps)}</span></div>`).join("");
    const timeseries = (data?.timeseries || []).map((item) => `<tr><td>${esc(item.timestamp)}</td><td>${formatBps(item.udp_bps)}</td><td>${formatBps(item.tcp_bps)}</td><td>${formatBps(item.icmp_bps)}</td></tr>`).join("");
    box.innerHTML = `<section class='traffic-card traffic-summary-card'><div class='traffic-controls'><span class='traffic-card-label'>${t("traffic_protocols_window")}</span><button type='button' data-window='60s'>${t("traffic_protocols_live")}</button><button type='button' data-window='5m'>${t("traffic_protocols_last_5m")}</button><button type='button' data-window='1h'>${t("traffic_protocols_last_1h")}</button><button type='button' data-window='24h'>${t("traffic_protocols_last_24h")}</button><button type='button' id='traffic-pause'>${analyticsPaused ? t("traffic_protocols_resume") : t("traffic_protocols_pause")}</button><label class='traffic-card-label' for='traffic-refresh-interval'>${t("traffic_protocols_refresh_interval")}</label><select id='traffic-refresh-interval' class='traffic-refresh-select'>${analyticsRefreshOptions.map((seconds) => `<option value='${seconds}'${analyticsPollingMs === seconds * 1e3 ? " selected" : ""}>${seconds}s</option>`).join("")}</select><span class='traffic-card-label'>${t("traffic_protocols_auto_refresh").replace("{seconds}", String(analyticsPollingMs / 1e3))}</span></div><h3>${t("traffic_protocols_summary")} (${esc(analyticsWindow)})</h3><div class='traffic-kpi-row'><div><span>${t("traffic_protocols_total_bps")}</span><strong>${formatBps(summary.total_bps)}</strong></div><div><span>${t("traffic_protocols_packet_rate")}</span><strong>${formatBps(summary.total_pps)} pps</strong></div><div><span>${t("traffic_protocols_active_flows")}</span><strong>${fmtNum(summary.active_flows)}</strong></div><div><span>${t("traffic_protocols_top_protocol")}</span><strong>${esc(summary.top_protocol || "-")}</strong></div></div></section><section class='traffic-card traffic-timeseries-card'><h3>${t("traffic_protocols_timeseries")}</h3><div class='traffic-table-scroll traffic-timeseries-scroll'><table class='talker-table'><thead><tr><th>${t("traffic_protocols_time")}</th><th>UDP</th><th>TCP</th><th>ICMP</th></tr></thead><tbody>${timeseries}</tbody></table></div></section><section class='traffic-card traffic-protocol-card'><h3>${t("traffic_protocols_share")}</h3><div class='traffic-donut-wrap'><div class='traffic-donut' style='background:conic-gradient(${stops.join(",") || "#334155 0 100%"})'></div><div class='traffic-legend'>${legend}</div></div></section><section class='traffic-card traffic-applications-card'><h3>${t("traffic_protocols_applications")}</h3><div class='traffic-scroll-panel traffic-share-list'>${applications || trafficProtocolsEmptyHtml()}</div></section><section class='traffic-card traffic-endpoints-card'><h3>${t("traffic_protocols_top_sources")}</h3><div class='traffic-scroll-panel traffic-legend'>${endpoints(data?.top_sources || []) || trafficProtocolsEmptyHtml()}</div></section><section class='traffic-card traffic-endpoints-card'><h3>${t("traffic_protocols_top_destinations")}</h3><div class='traffic-scroll-panel traffic-legend'>${endpoints(data?.top_destinations || []) || trafficProtocolsEmptyHtml()}</div></section><section class='traffic-card traffic-talkers-card'><h3>${t("traffic_protocols_top_talkers")}</h3><div class='traffic-table-scroll'><table class='talker-table traffic-talkers-table'><thead><tr><th>${t("traffic_protocols_source")}</th><th>${t("traffic_protocols_destination")}</th><th>${t("traffic_protocols_protocol")}</th><th>${t("traffic_protocols_application")}</th><th>bps</th><th>pps</th><th>${t("traffic_protocols_tcp_flags")}</th><th>${t("traffic_protocols_in_if")}</th><th>${t("traffic_protocols_out_if")}</th></tr></thead><tbody>${rows}</tbody></table></div></section>`;
    box.querySelectorAll("[data-window]").forEach((button) => button.addEventListener("click", () => {
      analyticsWindow = button.dataset.window || "60s";
      refreshTrafficProtocols();
    }));
    box.querySelector("#traffic-pause")?.addEventListener("click", () => {
      analyticsPaused = !analyticsPaused;
      scheduleAnalyticsPolling();
      refreshTrafficProtocols();
    });
    box.querySelector("#traffic-refresh-interval")?.addEventListener("change", (event) => {
      const seconds = Number(event.target.value);
      if (!analyticsRefreshOptions.includes(seconds)) return;
      analyticsPollingMs = seconds * 1e3;
      saveAnalyticsPollingMs(analyticsPollingMs);
      scheduleAnalyticsPolling();
      renderTrafficProtocols(data);
    });
  }
  function refreshTrafficProtocols() {
    if (analyticsPaused || document.visibilityState === "hidden") return;
    fetch(`/api/flow/analytics?window=${encodeURIComponent(analyticsWindow)}&limit=10`, { cache: "no-store" }).then((response) => response.json()).then(renderTrafficProtocols).catch(() => renderTrafficProtocols(null));
  }
  function scheduleAnalyticsPolling() {
    if (analyticsTimer !== null) window.clearInterval(analyticsTimer);
    analyticsTimer = analyticsPaused ? null : window.setInterval(refreshTrafficProtocols, analyticsPollingMs);
  }
  function counterClass(delta, spikeThreshold) {
    if (delta >= spikeThreshold) return "crit";
    if (delta > 0) return "warn";
    return "none";
  }
  function formatCounter(total, delta) {
    const numericDelta = Number(delta || 0);
    const totalText = `<span class='counter-total'>${fmtNum(total)}</span>`;
    if (!numericDelta) return totalText;
    return `${totalText}<span class='counter-delta ${counterClass(numericDelta, window.TRACEPULSE_SPIKE_THRESHOLD || 10)}'> (+${fmtNum(numericDelta)})</span>`;
  }
  function formatLateCollisions(total, delta) {
    const numericDelta = Number(delta || 0);
    const totalText = `<span class='counter-total'>${fmtNum(total)}</span>`;
    if (!numericDelta) return totalText;
    return `${totalText}<span class='counter-delta duplex'> (+${fmtNum(numericDelta)})</span>`;
  }
  function errorBreakdownPopoverHtml(errorBreakdown = {}, metrics = {}) {
    return `<div class='err-pop'><div class='err-pop-title'>${t("error_breakdown_hover_title")}</div><div class='err-pop-row'><span class='n'>${t("in_errors")}</span><span class='v'>+${fmtNum(metrics.in_errors_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("out_errors")}</span><span class='v'>+${fmtNum(metrics.out_errors_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("in_discards")}</span><span class='v'>+${fmtNum(metrics.in_discards_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("out_discards")}</span><span class='v'>+${fmtNum(metrics.out_discards_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("late_collisions")}</span><span class='v'>+${fmtNum(metrics.late_collisions_delta)}</span></div><div class='err-pop-divider'></div><div class='err-pop-row'><span class='n'>${t("error_breakdown_fcs_errors")}</span><span class='v'>+${fmtNum(errorBreakdown.fcs_errors_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("error_breakdown_alignment_errors")}</span><span class='v'>+${fmtNum(errorBreakdown.alignment_errors_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("error_breakdown_frame_too_longs")}</span><span class='v'>+${fmtNum(errorBreakdown.frame_too_longs_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("error_breakdown_internal_mac_receive_errors")}</span><span class='v'>+${fmtNum(errorBreakdown.internal_mac_receive_errors_delta)}</span></div></div>`;
  }
  function formatCounterWithBreakdown(total, delta, errorBreakdown, metrics, flipPopover = false) {
    return `<span class='err-cell${flipPopover ? " flip" : ""}' tabindex='0'>${formatCounter(total, delta)}${errorBreakdownPopoverHtml(errorBreakdown, metrics)}</span>`;
  }
  function diagnosticBadge(status) {
    const diagnostics = {
      healthy: { className: "healthy", translation: "diagnostic_healthy" },
      l1_error: { className: "l1_error", translation: "diagnostic_l1_error" },
      duplex_mismatch: { className: "duplex_mismatch", translation: "diagnostic_duplex_mismatch" },
      congestion: { className: "congestion", translation: "diagnostic_congestion" },
      down: { className: "down", translation: "diagnostic_down" }
    };
    const diagnostic = status === void 0 ? diagnostics.healthy : Object.prototype.hasOwnProperty.call(diagnostics, status) ? diagnostics[status] : diagnostics.down;
    return `<span class='diagnostic-badge ${diagnostic.className}'>${esc(t(diagnostic.translation))}</span>`;
  }
  function renderInterfaces(ifaces = []) {
    const tbody = document.getElementById("if-tbody");
    if (!tbody) return;
    if (ifaces.length === 0) {
      tbody.innerHTML = `<tr><td colspan=13 class=no-data>${t("no_interface_data")}</td></tr>`;
      return;
    }
    const spikeThreshold = window.TRACEPULSE_SPIKE_THRESHOLD || 10;
    tbody.innerHTML = ifaces.map((iface, index) => {
      const metrics = iface.metrics || {};
      const linkClass = iface.link_status === "up" ? "link-up" : "link-down";
      const errorDelta = Math.max(metrics.in_errors_delta || 0, metrics.out_errors_delta || 0);
      const discardDelta = Math.max(metrics.in_discards_delta || 0, metrics.out_discards_delta || 0);
      const rowClass = errorDelta >= spikeThreshold || discardDelta >= spikeThreshold ? " row-crit" : errorDelta > 0 || discardDelta > 0 ? " row-warn" : "";
      const predictiveStatus = iface.predictive_status;
      const predictiveBadge = predictiveStatus && (predictiveStatus.dom_warning || predictiveStatus.trend_warning) ? "<span class='pred-badge'>[PRED]</span>" : "";
      const flipErrorPopover = index >= ifaces.length - 2;
      return `<tr class='${rowClass.trim()}'><td>${iface.if_index}</td><td>${esc(iface.if_name)}${predictiveBadge}</td><td><span class=${linkClass}>${esc(iface.link_status)}</span></td><td>${diagnosticBadge(iface.health_status)}</td><td>${formatCounterWithBreakdown(metrics.in_errors, metrics.in_errors_delta, iface.error_breakdown, metrics, flipErrorPopover)}</td><td>${formatCounterWithBreakdown(metrics.out_errors, metrics.out_errors_delta, iface.error_breakdown, metrics, flipErrorPopover)}</td><td>${formatCounter(metrics.in_discards, metrics.in_discards_delta)}</td><td>${formatCounter(metrics.out_discards, metrics.out_discards_delta)}</td><td>${formatLateCollisions(metrics.late_collisions, metrics.late_collisions_delta)}</td><td>${(Number(metrics.bandwidth_utilization || 0) * 100).toFixed(1)}%</td><td>${opticalPowerHtml(iface.tx_power_dbm)}</td><td>${opticalPowerHtml(iface.rx_power_dbm === void 0 ? predictiveStatus?.rx_optical_power_dbm : iface.rx_power_dbm)}</td><td>${fmtTime(iface.sampled_at)}</td></tr>`;
    }).join("");
  }
  function opticalPowerHtml(value) {
    if (typeof value !== "number" || !Number.isFinite(value)) return "N/A";
    return `<span class="${value <= -20 ? "status-warning" : ""}">${value.toFixed(1)} dBm</span>`;
  }
  function renderHardwareStatus(sensors = []) {
    const box = document.getElementById("hardware-status");
    if (!box) return;
    const visible = sensors.filter((sensor) => {
      const type = (sensor.sensor_type || "").toLowerCase();
      const hasValue = sensor.value !== null && sensor.value !== void 0 || sensor.status !== null && sensor.status !== void 0;
      return (type === "temperature" || type === "power" || type === "fan") && hasValue;
    });
    if (visible.length === 0) {
      box.innerHTML = "<span class=no-data>No Sensors Detected</span>";
      return;
    }
    box.innerHTML = visible.map((sensor) => {
      const cls = sensor.is_alarm || sensor.status_kind === "critical" ? "hardware-card crit" : sensor.status_kind === "warning" ? "hardware-card warn" : sensor.status_kind === "normal" ? "hardware-card" : sensor.status !== null && sensor.status !== void 0 && sensor.status !== 0 ? "hardware-card warn" : "hardware-card";
      const label = sensor.name || (sensor.source === "entity-physical" ? `component ${sensor.index}` : `sensor ${sensor.index}`);
      const displayValue = sensor.status_text ? sensor.status_text : sensor.value !== null && sensor.value !== void 0 ? `${sensor.value}${sensor.unit ? ` ${sensor.unit}` : ""}` : sensor.status !== null && sensor.status !== void 0 ? sensor.status : "N/A";
      return `<div class='${cls}'><div class='label'>${esc(label)}</div><div class='value'>${esc(displayValue)}</div></div>`;
    }).join("");
  }
  function renderTemplateMetrics(metrics = [], error) {
    const box = document.getElementById("template-metrics");
    if (!box) return;
    if (error) {
      box.innerHTML = `<span class='no-data' role='alert'>${esc(error)}</span>`;
      return;
    }
    if (metrics.length === 0) {
      box.innerHTML = `<span class='no-data'>${t("template_metrics_empty")}</span>`;
      return;
    }
    box.innerHTML = metrics.map((metric) => {
      const value = metric.value === null ? "N/A" : `${metric.value_text ?? metric.value} ${metric.unit}`.trim();
      const note = metric.error ? `<div role='alert'>${esc(metric.error)}</div>` : "";
      return `<div class='hardware-card' data-metric-key='${esc(metric.key)}'><div class='label'>${esc(metric.label)}</div><div class='value'>${esc(value)}</div>${note}</div>`;
    }).join("");
  }
  function renderErrorBreakdownCard(ifaces = []) {
    const box = document.getElementById("error-breakdown");
    if (!box) return;
    if (ifaces.length === 0) {
      box.innerHTML = `<p class='no-data'>${t("error_breakdown_no_data")}</p>`;
      return;
    }
    if (errorBreakdownSelectedIf === null || !ifaces.some((iface) => iface.if_index === errorBreakdownSelectedIf)) {
      errorBreakdownSelectedIf = ifaces[0].if_index;
    }
    const options = ifaces.map((iface) => {
      const selected2 = iface.if_index === errorBreakdownSelectedIf ? " selected" : "";
      return `<option value='${iface.if_index}'${selected2}>${esc(iface.if_name || `if-${iface.if_index}`)} (if-${iface.if_index})</option>`;
    }).join("");
    const selected = ifaces.find((iface) => iface.if_index === errorBreakdownSelectedIf) || ifaces[0];
    const eb = selected.error_breakdown || {};
    const fcs = eb.fcs_errors_delta || 0;
    const align = eb.alignment_errors_delta || 0;
    const toolong = eb.frame_too_longs_delta || 0;
    const macrx = eb.internal_mac_receive_errors_delta || 0;
    const total = fcs + align + toolong + macrx;
    const pct = (value) => total > 0 ? value / total * 100 : 0;
    const bar = `<div class='breakdown-bar'><div class='breakdown-bar-seg fcs' style='width:${pct(fcs)}%'></div><div class='breakdown-bar-seg alignment' style='width:${pct(align)}%'></div><div class='breakdown-bar-seg frametoolong' style='width:${pct(toolong)}%'></div><div class='breakdown-bar-seg macreceive' style='width:${pct(macrx)}%'></div></div>`;
    const legend = `<div class='breakdown-legend'><span><span class='swatch' style='background:#f87171'></span>${t("error_breakdown_fcs_errors")}</span><span><span class='swatch' style='background:#fbbf24'></span>${t("error_breakdown_alignment_errors")}</span><span><span class='swatch' style='background:#a78bfa'></span>${t("error_breakdown_frame_too_longs")}</span><span><span class='swatch' style='background:#38bdf8'></span>${t("error_breakdown_internal_mac_receive_errors")}</span></div>`;
    const table = `<table class='breakdown-table'><thead><tr><th>${t("error_breakdown_fcs_errors")}</th><th>${t("error_breakdown_alignment_errors")}</th><th>${t("error_breakdown_frame_too_longs")}</th><th>${t("error_breakdown_internal_mac_receive_errors")}</th></tr></thead><tbody><tr><td>${fmtNum(eb.fcs_errors)} <span class='counter-delta warn'>(+${fmtNum(fcs)})</span></td><td>${fmtNum(eb.alignment_errors)} <span class='counter-delta warn'>(+${fmtNum(align)})</span></td><td>${fmtNum(eb.frame_too_longs)} <span class='counter-delta warn'>(+${fmtNum(toolong)})</span></td><td>${fmtNum(eb.internal_mac_receive_errors)} <span class='counter-delta warn'>(+${fmtNum(macrx)})</span></td></tr></tbody></table>`;
    box.innerHTML = `<div class='error-breakdown-select'><label for='error-breakdown-if'>${t("error_breakdown_select_hint")}</label><select id='error-breakdown-if' onchange='onErrorBreakdownIfaceChange(this.value)'>${options}</select></div>` + bar + legend + table;
  }
  function onErrorBreakdownIfaceChange(value) {
    errorBreakdownSelectedIf = parseInt(value, 10);
    if (window.LAST_DEVICE_DETAIL) renderErrorBreakdownCard(window.LAST_DEVICE_DETAIL.interfaces || []);
  }
  function renderSpikes(spikes = []) {
    const tbody = document.getElementById("spike-tbody");
    if (!tbody) return;
    if (spikes.length === 0) {
      tbody.innerHTML = `<tr><td colspan=8 class=no-data>${t("no_interface_spikes")}</td></tr>`;
      return;
    }
    tbody.innerHTML = spikes.map((spike) => {
      const linkClass = spike.link_status === "up" ? "link-up" : "link-down";
      return `<tr><td>${fmtTime(spike.latest_sampled_at)}</td><td>${esc(spike.if_name)} (if-${spike.if_index || 0})</td><td><span class=${linkClass}>${esc(spike.link_status)}</span></td><td>${fmtNum(spike.in_errors_delta)}</td><td>${fmtNum(spike.out_errors_delta)}</td><td>${fmtNum(spike.in_discards_delta)}</td><td>${fmtNum(spike.out_discards_delta)}</td><td>${fmtNum(spike.total_delta)}</td></tr>`;
    }).join("");
  }
  function renderAlerts(alerts = []) {
    const tbody = document.getElementById("alert-tbody");
    if (!tbody) return;
    if (alerts.length === 0) {
      tbody.innerHTML = `<tr><td colspan=5 class=no-data>${t("no_alerts")}</td></tr>`;
      return;
    }
    const severityClasses = { warning: "sev-warning", critical: "sev-critical", info: "sev-info" };
    tbody.innerHTML = alerts.map((alert) => {
      const severity = alert.severity || "";
      const severityClass = severityClasses[severity] || "";
      const iface = alert.interface || "-";
      return `<tr><td>${fmtTime(alert.at)}</td><td>${esc(iface)}</td><td>${esc(alert.type)}</td><td><span class=${severityClass}>${esc(severity)}</span></td><td>${esc(alert.details)}</td></tr>`;
    }).join("");
  }
  function formatAxisValue(value, yLabel) {
    if (yLabel === "bytes") {
      const units = ["B", "KB", "MB", "GB", "TB"];
      let displayValue = value;
      let index = 0;
      while (displayValue >= 1024 && index + 1 < units.length) {
        displayValue /= 1024;
        index++;
      }
      return `${index === 0 ? Math.round(displayValue) : displayValue.toFixed(1)} ${units[index]}`;
    }
    if (yLabel === "%") {
      if (value < 1) return value.toFixed(2);
      if (value < 10) return value.toFixed(1);
    }
    return Math.round(value).toString();
  }
  function sparkline(svgId, series, yLabel) {
    const svg = document.getElementById(svgId);
    if (!svg) return;
    const rect = svg.getBoundingClientRect();
    const width = Math.max(200, Math.round(rect.width) || 800);
    const height = Math.max(80, Math.round(rect.height) || 160);
    svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
    const fontSize = width < 420 ? 8 : 9;
    const paddingLeft = width < 420 ? 30 : 40;
    const paddingRight = 8;
    const paddingTop = 8;
    const paddingBottom = 20;
    const chartWidth = width - paddingLeft - paddingRight;
    const chartHeight = height - paddingTop - paddingBottom;
    const allPoints = series.flatMap((item) => (item.points || []).filter((point) => point && point.v !== null && point.v !== void 0 && !Number.isNaN(point.v)));
    if (allPoints.length === 0) {
      svg.classList.add("chart-empty");
      svg.innerHTML = `<text x='${width / 2}' y='${height / 2}' text-anchor='middle' fill='#64748b' font-size='13'>N/A</text>`;
      return;
    }
    svg.classList.remove("chart-empty");
    const times = allPoints.map((point) => parseTracePulseTime(point.t).getTime());
    const values = allPoints.map((point) => point.v);
    const timeMin = Math.min(...times);
    let timeMax = Math.max(...times);
    const valueMin = 0;
    const valueMax = yLabel === "%" ? 100 : Math.max(...values) * 1.15 || 1;
    if (timeMin === timeMax) timeMax = timeMin + 1;
    const tx = (time) => paddingLeft + (parseTracePulseTime(time).getTime() - timeMin) / (timeMax - timeMin) * chartWidth;
    const ty = (value) => paddingTop + chartHeight - (value - valueMin) / (valueMax - valueMin) * chartHeight;
    let output = "";
    for (let index = 0; index <= 4; index++) {
      const gridY = paddingTop + index * chartHeight / 4;
      const gridValue = (valueMax - valueMin) * (1 - index / 4) + valueMin;
      output += `<line x1='${paddingLeft}' y1='${gridY}' x2='${width - paddingRight}' y2='${gridY}' stroke='#1e293b' stroke-width='1'/>`;
      output += `<text x='${paddingLeft - 4}' y='${gridY + 4}' text-anchor='end' fill='#64748b' font-size='${fontSize}'>${formatAxisValue(gridValue, yLabel)}</text>`;
    }
    for (let index = 0; index <= 3; index++) {
      const labelTime = timeMin + (timeMax - timeMin) * index / 3;
      const x = paddingLeft + (labelTime - timeMin) / (timeMax - timeMin) * chartWidth;
      let anchor = "middle";
      let labelX = x;
      if (index === 0) {
        anchor = "start";
        labelX = paddingLeft + 2;
      } else if (index === 3) {
        anchor = "end";
        labelX = width - paddingRight - 2;
      }
      output += `<text x='${labelX}' y='${height - 4}' text-anchor='${anchor}' fill='#475569' font-size='${fontSize}'>${formatTracePulseClock(new Date(labelTime))}</text>`;
    }
    series.forEach((item) => {
      if (item.points.length === 0) return;
      const path = item.points.map((point, index) => `${index === 0 ? "M" : "L"}${tx(point.t).toFixed(1)} ${ty(point.v).toFixed(1)}`).join(" ");
      output += `<path d='${path}' fill='none' stroke='${item.color}' stroke-width='1.5' stroke-linejoin='round'/>`;
    });
    svg.innerHTML = output;
  }
  function interfaceLabel(ifIndex) {
    return window.IFACE_LABELS?.[String(ifIndex)] || `if-${ifIndex}`;
  }
  function renderBwChart(ifSeries = []) {
    const legend = document.getElementById("bw-legend");
    if (legend) legend.innerHTML = "";
    const ordered = [...ifSeries].sort((a, b) => a.if_index - b.if_index);
    const filtered = ordered.filter((series) => window.isInterfaceSelected?.(series.if_index));
    const chartSeries = filtered.map((series, index) => {
      const color = chartColors[index % chartColors.length];
      const label = interfaceLabel(series.if_index);
      if (legend) legend.innerHTML += `<span style='color:${color};margin-right:.5rem'>&#9632; ${esc(label)}</span>`;
      return { label, color, points: series.points.map((point) => ({ t: point.t, v: point.bw || 0 })) };
    });
    sparkline("bw-chart", chartSeries, "%");
  }
  function renderErrChart(ifSeries = []) {
    const legend = document.getElementById("err-legend");
    if (legend) legend.innerHTML = "";
    const ordered = [...ifSeries].sort((a, b) => a.if_index - b.if_index);
    const chartSeries = [];
    ordered.filter((series) => window.isInterfaceSelected?.(series.if_index)).forEach((series, index) => {
      const color = chartColors[index % chartColors.length];
      const label = interfaceLabel(series.if_index);
      if (legend && index < 4) legend.innerHTML += `<span style='color:${color};margin-right:.5rem'>&#9632; ${esc(label)}</span>`;
      chartSeries.push({
        label: `${label} in_err`,
        color,
        points: series.points.map((point) => ({ t: point.t, v: (point.in_err || 0) + (point.in_dis || 0) }))
      });
    });
    sparkline("err-chart", chartSeries, "count");
  }
  function renderCpuChart(metrics = []) {
    sparkline("cpu-chart", [{
      label: "CPU %",
      color: "#38bdf8",
      points: metrics.filter((metric) => metric.cpu !== null && metric.cpu !== void 0 && metric.cpu > 0).map((metric) => ({ t: metric.t, v: metric.cpu || 0 }))
    }], "%");
  }
  function renderMemoryChart(metrics = []) {
    const memSeries = { label: "Memory Usage (%)", color: "#a78bfa", points: [] };
    metrics.forEach((metric) => {
      if (metric && metric.memory !== null && metric.memory !== void 0 && !Number.isNaN(metric.memory)) {
        memSeries.points.push({ t: metric.t, v: metric.memory });
      }
    });
    if (memSeries.points.length === 0 && metrics.length > 0) {
      const maxBytes = metrics.reduce((max, metric) => metric.memory_bytes && metric.memory_bytes > max ? metric.memory_bytes : max, 0);
      if (maxBytes > 0) {
        metrics.forEach((metric) => {
          if (metric.memory_bytes !== null && metric.memory_bytes !== void 0 && metric.memory_bytes > 0) {
            memSeries.points.push({ t: metric.t, v: Math.round(metric.memory_bytes / maxBytes * 100) });
          }
        });
      }
    }
    sparkline("memory-chart", [memSeries], "%");
  }
  function renderSysChart(metrics = []) {
    renderCpuChart(metrics);
    renderMemoryChart(metrics);
  }
  function renderHeader(device) {
    const title = document.getElementById("dev-title");
    if (title) {
      title.textContent = `${deviceLabel(device)} (${device.ip || ""})`;
      let metadata = document.getElementById("device-identity");
      if (!metadata) {
        metadata = document.createElement("div");
        metadata.id = "device-identity";
        title.parentElement.after(metadata);
      }
      metadata.innerHTML = `<div class="device-original">sysName: ${esc(device.name || "-")}</div>
            <div class="device-tags">${(device.tags || []).map((tag) => `<span class="device-tag">${esc(tag)}</span>`).join("")}</div>
            <button type="button" id="edit-device-metadata">${esc(metadataText("edit"))}</button>`;
      metadata.querySelector("button").onclick = () => {
        if (lastDeviceDetail) editDeviceMetadata(lastDeviceDetail, loadDeviceDetail);
      };
    }
    const status = document.getElementById("dev-status");
    if (status) {
      const classMap = { online: "status-online", offline: "status-offline", warning: "status-warning", critical: "status-critical" };
      status.className = classMap[device.status || ""] || "status-unknown";
      status.textContent = device.status || "unknown";
    }
    document.title = `TracePulse - ${deviceLabel(device) || "Device"}`;
  }
  function updateLastRefreshed() {
    const element = document.getElementById("device-refresh");
    if (element) {
      element.textContent = `${t("device_auto_refresh").replace("{seconds}", String(deviceRefreshSeconds))} \u2022 ${t("updated")}: ${formatTracePulseClock(/* @__PURE__ */ new Date())}`;
    }
  }
  function scheduleDeviceRefresh() {
    if (deviceRefreshTimer !== null) window.clearInterval(deviceRefreshTimer);
    deviceRefreshTimer = window.setInterval(loadDeviceDetail, deviceRefreshSeconds * 1e3);
  }
  function renderDetail(device) {
    lastDeviceDetail = device;
    window.LAST_DEVICE_DETAIL = device;
    renderHeader(device);
    const ifaces = device.interfaces || [];
    window.IFACE_LABELS = {};
    ifaces.forEach((iface) => {
      window.IFACE_LABELS[String(iface.if_index)] = iface.if_name || `if-${iface.if_index}`;
    });
    syncSelectedInterfaces(ifaces);
    renderInterfaceSelection(ifaces);
    renderInterfaces(ifaces);
    renderErrorBreakdownCard(ifaces);
    refreshTrafficProtocols();
    renderBwChart(device.if_series || []);
    renderErrChart(device.if_series || []);
    renderSysChart(device.metrics || []);
    renderHardwareStatus(device.hardware_sensors || []);
    renderTemplateMetrics(device.template_metrics || [], device.template_metrics_error);
    renderSpikes(device.spikes || []);
    renderAlerts(device.alerts || []);
    updateLastRefreshed();
  }
  function loadDeviceDetail() {
    Promise.all([
      fetch("/api/settings", { cache: "no-store" }).then((response) => response.json()).catch(() => null),
      fetch(`/api/device/${encodeURIComponent(window.DEVICE_IP || "")}`, { cache: "no-store" }).then((response) => response.json())
    ]).then(([settings, device]) => {
      const configuredInterval = Number(settings?.polling?.interval_seconds);
      if (Number.isFinite(configuredInterval) && configuredInterval >= 5 && configuredInterval <= 600) {
        deviceRefreshSeconds = configuredInterval;
        scheduleDeviceRefresh();
      }
      if (settings?.display?.timezone) {
        try {
          localStorage.setItem("tracepulse-timezone", settings.display.timezone);
        } catch {
        }
      }
      if (settings?.alert && typeof settings.alert.spike_threshold === "number") {
        window.TRACEPULSE_SPIKE_THRESHOLD = settings.alert.spike_threshold;
      }
      if (device.error) {
        const title = document.getElementById("dev-title");
        if (title) title.textContent = device.error;
        return;
      }
      renderDetail(device);
    }).catch((error) => {
      const title = document.getElementById("dev-title");
      if (title) title.textContent = `Error: ${String(error)}`;
    });
  }
  window.esc = esc;
  window.fmtNum = fmtNum;
  window.fmtTime = fmtTime;
  window.formatBps = formatBps;
  window.formatTracePulseClock = formatTracePulseClock;
  window.formatTracePulseTime = formatTracePulseTime;
  window.isInterfaceSelected = isInterfaceSelected;
  window.loadDeviceDetail = loadDeviceDetail;
  window.onErrorBreakdownIfaceChange = onErrorBreakdownIfaceChange;
  window.parseTracePulseTime = parseTracePulseTime;
  window.renderAlerts = renderAlerts;
  window.renderBwChart = renderBwChart;
  window.renderCpuChart = renderCpuChart;
  window.renderErrChart = renderErrChart;
  window.renderErrorBreakdownCard = renderErrorBreakdownCard;
  window.renderHardwareStatus = renderHardwareStatus;
  window.renderInterfaces = renderInterfaces;
  window.renderDetail = renderDetail;
  window.renderMemoryChart = renderMemoryChart;
  window.renderSpikes = renderSpikes;
  window.renderSysChart = renderSysChart;
  window.renderTrafficProtocols = renderTrafficProtocols;
  window.refreshTrafficProtocols = refreshTrafficProtocols;
  window.selectAllInterfaces = selectAllInterfaces;
  window.sparkline = sparkline;
  window.toggleInterfaceSelection = toggleInterfaceSelection;
  window.tracePulseTimeZone = tracePulseTimeZone;
  window.applyPageLanguage = () => {
    if (lastDeviceDetail) renderHeader(lastDeviceDetail);
    updateLastRefreshed();
    refreshTrafficProtocols();
  };
  loadDeviceDetail();
  scheduleDeviceRefresh();
  scheduleAnalyticsPolling();
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") {
      refreshTrafficProtocols();
      scheduleAnalyticsPolling();
    } else if (analyticsTimer !== null) {
      window.clearInterval(analyticsTimer);
      analyticsTimer = null;
    }
  });
  window.addEventListener("resize", () => {
    if (resizeTimer !== null) window.clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(() => {
      if (lastDeviceDetail) renderDetail(lastDeviceDetail);
    }, 200);
  });
  return __toCommonJS(device_detail_exports);
})();
