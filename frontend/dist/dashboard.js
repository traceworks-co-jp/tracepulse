"use strict";
var TracePulseDashboard = (() => {
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

  // frontend/src/pages/dashboard.ts
  var dashboard_exports = {};
  __export(dashboard_exports, {
    refresh: () => refresh,
    renderSummary: () => renderSummary,
    renderTable: () => renderTable,
    sortBy: () => sortBy,
    unregisterDevice: () => unregisterDevice,
    updateSortIndicators: () => updateSortIndicators
  });

  // frontend/src/shared/escape.ts
  function esc(value) {
    return String(value === null || value === void 0 ? "" : value).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/\x22/g, "&quot;");
  }

  // frontend/src/shared/format.ts
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

  // frontend/src/pages/dashboard.ts
  var columns = ["ip", "name", "status", "community", "last_seen"];
  var sortColumn = "ip";
  var sortAscending = true;
  var deletingIps = /* @__PURE__ */ new Set();
  function t(key) {
    return typeof window.t === "function" ? window.t(key) : key;
  }
  function ipToNumber(ip) {
    const parts = ip.split(".");
    if (parts.length !== 4) return 0;
    return parts.reduce((value, part) => value * 256 + (parseInt(part, 10) || 0), 0);
  }
  function statusOrder(status) {
    const order = { critical: 0, offline: 1, warning: 2, unknown: 3, online: 4 };
    return order[status || ""] ?? 5;
  }
  function compareValue(a, b, column) {
    if (column === "ip") return ipToNumber(a.ip) - ipToNumber(b.ip);
    if (column === "status") return statusOrder(a.status) - statusOrder(b.status);
    const left = String(a[column] || "").toLowerCase();
    const right = String(b[column] || "").toLowerCase();
    return left < right ? -1 : left > right ? 1 : 0;
  }
  function currentDevices() {
    return window.DEVICES || [];
  }
  function renderTable() {
    const tbody = document.getElementById("dash-tbody");
    if (!tbody) return;
    const sorted = currentDevices().slice().sort((a, b) => {
      if (a.error_spike && !b.error_spike) return -1;
      if (!a.error_spike && b.error_spike) return 1;
      const result = compareValue(a, b, sortColumn);
      return sortAscending ? result : -result;
    });
    if (sorted.length === 0) {
      tbody.innerHTML = `<tr><td colspan=6 class=empty>${t("no_devices")} <a href=/discovery>${t("nav_discovery")}</a></td></tr>`;
      return;
    }
    tbody.innerHTML = sorted.map((device) => {
      const classMap = { online: "status-online", offline: "status-offline", warning: "status-warning", critical: "status-critical" };
      const statusClass = classMap[device.status || ""] || "status-unknown";
      const rowClass = device.error_spike ? " class=spike-row" : "";
      const spikeBadge = device.error_spike ? "<span class=spike-badge>&#9888; Error Spike</span>" : "";
      const deleting = deletingIps.has(device.ip);
      const actionLabel = deleting ? t("unregistering") : t("unregister");
      const encodedIp = encodeURIComponent(device.ip);
      const encodedName = encodeURIComponent(device.name || device.ip);
      const actionButton = `<button class='btn-row-danger' data-ip='${encodedIp}' data-name='${encodedName}' onclick='unregisterDevice(this.dataset.ip,this.dataset.name)' ${deleting ? "disabled" : ""}>${actionLabel}</button>`;
      const lastSeen = fmtTime(device.last_seen || device.last_seen_at);
      return `<tr${rowClass}><td><a href='/device/${encodeURIComponent(device.ip)}' style='color:#38bdf8;text-decoration:none'>${esc(device.ip)}</a></td><td>${esc(device.name || "")}${spikeBadge}</td><td><span class=${statusClass}>${esc(device.status)}</span></td><td>${esc(device.community)}</td><td>${lastSeen}</td><td><div class='row-actions'>${actionButton}</div></td></tr>`;
    }).join("");
  }
  function updateSortIndicators() {
    columns.forEach((column) => {
      const header = document.getElementById(`th-${column}`);
      const icon = document.getElementById(`sort-${column}`);
      if (!header) return;
      if (column === sortColumn) {
        header.className = sortAscending ? "sort-asc" : "sort-desc";
        if (icon) icon.textContent = sortAscending ? "\u25B4" : "\u25BE";
      } else {
        header.className = "";
        if (icon) icon.textContent = "";
      }
    });
  }
  function sortBy(column) {
    if (!columns.includes(column)) return;
    sortAscending = sortColumn === column ? !sortAscending : true;
    sortColumn = column;
    updateSortIndicators();
    renderTable();
  }
  function renderSummary(devices = currentDevices()) {
    let online = 0;
    let warning = 0;
    let offline = 0;
    let spikes = 0;
    devices.forEach((device) => {
      if (device.status === "online") online++;
      else if (device.status === "warning") warning++;
      else if (device.status === "offline" || device.status === "critical") offline++;
      if (device.error_spike) spikes++;
    });
    const element = document.getElementById("summary-cards");
    if (!element) return;
    const spikeClass = spikes > 0 ? "card card-spike" : "card";
    element.innerHTML = `<div class='card card-online'><div class='card-num'>${online}</div><div class='card-label'>${t("online")}</div></div><div class='card card-warning'><div class='card-num'>${warning}</div><div class='card-label'>${t("warning")}</div></div><div class='card card-offline'><div class='card-num'>${offline}</div><div class='card-label'>${t("offline")}</div></div><div class='card'><div class='card-num'>${devices.length}</div><div class='card-label'>${t("total")}</div></div><div class='${spikeClass}'><div class='card-num'>${spikes}</div><div class='card-label'>${t("error_spikes")}</div></div>`;
  }
  function unregisterDevice(encodedIp, encodedName) {
    const ip = decodeURIComponent(encodedIp || "");
    const name = decodeURIComponent(encodedName || "");
    if (!window.confirm(`${t("unregister_confirm")}

${ip}${name ? ` (${name})` : ""}`)) return;
    deletingIps.add(ip);
    renderTable();
    fetch(`/api/device/${encodeURIComponent(ip)}`, { method: "DELETE" }).then(async (response) => ({ ok: response.ok, status: response.status, body: await response.json() })).then((result) => {
      if (!result.ok || result.body?.error) throw new Error(result.body?.error || `HTTP ${result.status}`);
      deletingIps.delete(ip);
      refresh();
      window.alert(t("unregister_success"));
    }).catch((error) => {
      deletingIps.delete(ip);
      renderTable();
      window.alert(`${t("unregister_failed")}: ${String(error)}`);
    });
  }
  function updateLastRefreshed() {
    const element = document.getElementById("last-refreshed");
    if (element) element.textContent = `${t("updated")}: ${formatTracePulseClock(/* @__PURE__ */ new Date())}`;
  }
  function refresh() {
    fetch("/api/devices").then((response) => response.json()).then((devices) => {
      window.DEVICES = devices;
      renderTable();
      renderSummary(devices);
      updateLastRefreshed();
    }).catch((error) => console.warn("refresh failed", error));
  }
  window.renderTable = renderTable;
  window.renderSummary = renderSummary;
  window.refreshDashboard = refresh;
  window.sortBy = sortBy;
  window.unregisterDevice = unregisterDevice;
  renderTable();
  updateSortIndicators();
  updateLastRefreshed();
  window.setInterval(refresh, 3e4);
  return __toCommonJS(dashboard_exports);
})();
