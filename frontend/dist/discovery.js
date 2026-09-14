"use strict";
var TracePulseDiscovery = (() => {
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

  // frontend/src/pages/discovery.ts
  var discovery_exports = {};
  __export(discovery_exports, {
    addManual: () => addManual,
    cancelScan: () => cancelScan,
    hideManual: () => hideManual,
    registerSelected: () => registerSelected,
    showManual: () => showManual,
    startScan: () => startScan,
    toggleAll: () => toggleAll,
    updateHint: () => updateHint,
    updateRegisterBtn: () => updateRegisterBtn
  });
  var scanTimeoutSeconds = 300;
  var pollIntervalMs = 1500;
  var currentJobId = null;
  var pollTimer = null;
  var elapsedTimer = null;
  var scanStartedAt = null;
  var cancelled = false;
  function t(key) {
    return typeof window.t === "function" ? window.t(key) : key;
  }
  function el(id) {
    return document.getElementById(id);
  }
  function isEnterprise() {
    return !!window.TRACEPULSE_WEB_EDITION?.enterprise;
  }
  function existingIps() {
    return window.DISCOVERY_EXISTING || /* @__PURE__ */ new Set();
  }
  function cidrHostCount(cidr) {
    const match = cidr.match(/^\d+\.\d+\.\d+\.\d+\/(\d+)$/);
    if (!match) return null;
    const prefix = parseInt(match[1], 10);
    if (prefix < 0 || prefix > 32) return null;
    if (prefix === 32) return 1;
    if (prefix === 31) return 2;
    return Math.pow(2, 32 - prefix) - 2;
  }
  function cidrEntries(value) {
    return value.split(",").map((entry) => entry.trim()).filter(Boolean);
  }
  function updateHint() {
    const hint = el("host-hint");
    const entries = cidrEntries(el("cidr").value.trim());
    const counts = entries.map(cidrHostCount);
    if (entries.length === 0 || counts.some((count) => count === null)) {
      hint.innerHTML = "";
      return;
    }
    const total = counts.reduce((sum, count) => sum + (count || 0), 0);
    if (!total) {
      hint.innerHTML = "";
      return;
    }
    const smallestPrefix = Math.min(...entries.map((entry) => parseInt(entry.split("/")[1], 10)));
    const maxPrefix = isEnterprise() ? 0 : 24;
    if (!isEnterprise() && smallestPrefix < maxPrefix) {
      hint.textContent = `Community\u7248\u3067\u306F\u5358\u4E00\u30B5\u30D6\u30CD\u30C3\u30C8\uFF08/${maxPrefix}\u4EE5\u4E0A\uFF09\u306E\u307F\u30B9\u30AD\u30E3\u30F3\u53EF\u80FD\u3067\u3059\u3002\u73FE\u5728: /${smallestPrefix}`;
      hint.className = "host-hint warn";
      return;
    }
    const seconds = Math.ceil(total * 0.5 / 256);
    hint.textContent = `${total.toLocaleString()} hosts \u2022 est. ~${seconds >= 60 ? `${Math.ceil(seconds / 60)} min` : `${seconds}s`}`;
    hint.className = "host-hint";
  }
  function applyPageLanguage(_lang) {
    const cidr = document.getElementById("cidr");
    if (cidr && isEnterprise()) cidr.placeholder = t("cidr_range_placeholder_enterprise");
    window.refreshTopologyTexts?.();
  }
  async function startScan() {
    const cidr = el("cidr").value.trim();
    const community = el("community").value.trim() || "public";
    const seedIp = document.getElementById("seed-ip")?.value.trim() || "";
    if (!cidr) {
      showScanError("Please enter a CIDR range.");
      return;
    }
    const entries = cidrEntries(cidr);
    const prefixes = entries.map((entry) => parseInt(entry.split("/")[1] || "33", 10));
    const smallestPrefix = Math.min(...prefixes);
    if (!isEnterprise() && (entries.length > 1 || smallestPrefix < 24)) {
      showScanError("Community\u7248\u3067\u306F\u5358\u4E00\u30B5\u30D6\u30CD\u30C3\u30C8\uFF08/24\u4EE5\u4E0A\uFF09\u306E\u307F\u30B9\u30AD\u30E3\u30F3\u53EF\u80FD\u3067\u3059\u3002");
      return;
    }
    const total = entries.map(cidrHostCount).reduce((sum, count) => sum + (count || 0), 0);
    cancelled = false;
    currentJobId = null;
    stopPolling();
    el("scan-btn").style.display = "none";
    el("cancel-btn").style.display = "";
    el("scan-error").style.display = "none";
    el("results-section").style.display = "none";
    el("topology-section").style.display = "none";
    setProgress(0, 0, 0);
    el("scan-progress").style.display = "block";
    try {
      const response = await fetch("/api/discovery/scan", { method: "POST", headers: { "Content-Type": "application/x-www-form-urlencoded" }, body: `cidr=${encodeURIComponent(cidr)}&community=${encodeURIComponent(community)}&max_hosts=${encodeURIComponent(total || 65534)}` });
      const data = await response.json();
      if (data.error || !data.job_id) {
        finishScanError(data.error || "Failed to start scan");
        return;
      }
      currentJobId = data.job_id;
      scanStartedAt = Date.now();
      if (seedIp && typeof window.startTopologyDiscovery === "function") window.startTopologyDiscovery(seedIp, community);
      startPolling(data.total || total || 0);
    } catch (error) {
      finishScanError(`Failed to start scan: ${String(error)}`);
    }
  }
  function startPolling(total) {
    elapsedTimer = window.setInterval(() => {
      if (!scanStartedAt) return;
      const elapsed = Math.floor((Date.now() - scanStartedAt) / 1e3);
      el("progress-elapsed").textContent = `Elapsed: ${elapsed}s`;
      if (elapsed >= scanTimeoutSeconds) {
        cancelScan();
        finishScanError(`Scan timed out after ${elapsed}s.`);
      }
    }, 1e3);
    pollTimer = window.setInterval(async () => {
      if (!currentJobId || cancelled) return;
      try {
        const response = await fetch(`/api/discovery/scan/${currentJobId}`);
        const data = await response.json();
        if (cancelled) return;
        if (data.error) {
          finishScanError(data.error);
          return;
        }
        setProgress(data.scanned || 0, data.total || total, data.elapsed || 0);
        if (data.status === "done") finishScanDone(data.devices || [], data.scanned || 0, data.total || total, data.elapsed || 0);
        else if (data.status === "error") finishScanError(data.error || "Scan failed");
      } catch (error) {
        finishScanError(`Polling error: ${String(error)}`);
      }
    }, pollIntervalMs);
  }
  function setProgress(scanned, total, elapsed) {
    const percent = total > 0 ? Math.round(scanned / total * 100) : 0;
    el("progress-bar").style.width = `${percent}%`;
    el("progress-stats").textContent = `${scanned} / ${total || "?"} (${percent}%)`;
    el("progress-elapsed").textContent = `${t("elapsed")}: ${elapsed}s`;
    el("progress-label").textContent = t("scanning");
  }
  function stopPolling() {
    if (pollTimer !== null) window.clearInterval(pollTimer);
    if (elapsedTimer !== null) window.clearInterval(elapsedTimer);
    pollTimer = null;
    elapsedTimer = null;
  }
  function cancelScan() {
    cancelled = true;
    currentJobId = null;
    stopPolling();
    el("scan-btn").style.display = "";
    el("cancel-btn").style.display = "none";
    el("scan-progress").style.display = "none";
  }
  function finishScanError(message) {
    stopPolling();
    el("scan-btn").style.display = "";
    el("cancel-btn").style.display = "none";
    el("scan-progress").style.display = "none";
    showScanError(message);
  }
  function finishScanDone(devices, scanned, total, elapsed) {
    stopPolling();
    currentJobId = null;
    el("scan-btn").style.display = "";
    el("cancel-btn").style.display = "none";
    el("scan-progress").style.display = "none";
    renderResults(devices, scanned, total, elapsed);
  }
  function showScanError(message) {
    const error = el("scan-error");
    error.textContent = `! ${message}`;
    error.style.display = "block";
  }
  function renderResults(devices, scanned, total, elapsed) {
    const tbody = el("results-body");
    tbody.innerHTML = "";
    el("results-title").textContent = `${t("scan_results_label")} - ${devices.length} ${t("found")} (scanned ${scanned}/${total}, ${elapsed}s)`;
    if (devices.length === 0) tbody.innerHTML = `<tr><td colspan='5' class='empty'>${t("no_snmp_devices_found")}</td></tr>`;
    devices.forEach((device) => {
      const registered = existingIps().has(device.ip);
      const row = document.createElement("tr");
      row.innerHTML = `<td><input type='checkbox' class='row-cb' data-ip='${device.ip}' data-name='${device.name || ""}' data-community='${device.community || "public"}'${registered ? " disabled" : " onchange='updateRegisterBtn()'"}></td><td>${device.ip}</td><td>${device.name || ""}</td><td><span class='${device.status === "online" ? "status-online" : "status-unknown"}'>${device.status || "unknown"}</span></td><td>${registered ? "registered" : ""}</td>`;
      tbody.appendChild(row);
    });
    el("results-section").style.display = "block";
    el("select-all").checked = false;
    updateRegisterBtn();
  }
  function updateRegisterBtn() {
    const count = document.querySelectorAll(".row-cb:checked").length;
    el("register-action").style.display = count > 0 ? "flex" : "none";
    el("select-count").textContent = count ? `${count} ${t("selected")}` : "";
    if (count) el("register-btn").textContent = `+ ${t("register")} ${count} device${count > 1 ? "s" : ""}`;
  }
  function toggleAll(master) {
    document.querySelectorAll(".row-cb:not(:disabled)").forEach((checkbox) => {
      checkbox.checked = master.checked;
    });
    updateRegisterBtn();
  }
  async function registerSelected() {
    const selected = Array.from(document.querySelectorAll(".row-cb:checked")).map((checkbox) => ({ ip: checkbox.dataset.ip, name: checkbox.dataset.name, community: checkbox.dataset.community }));
    if (!selected.length) {
      window.alert(t("select_at_least_one_device"));
      return;
    }
    const button = el("register-btn");
    button.disabled = true;
    button.textContent = t("registering");
    try {
      const response = await fetch("/api/discovery/register", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(selected) });
      const data = await response.json();
      const result = el("register-result");
      result.className = data.error || data.registered.length === 0 ? "reg-error" : "reg-success";
      result.textContent = data.error || `Registered: ${data.registered.join(", ")}. Skipped: ${data.skipped.join(", ")}.`;
      result.style.display = "block";
      data.registered.forEach((ip) => existingIps().add(ip));
      updateRegisterBtn();
    } catch (error) {
      el("register-result").textContent = String(error);
    } finally {
      button.disabled = false;
      updateRegisterBtn();
    }
  }
  function showManual() {
    el("manual-box").style.display = "block";
  }
  function hideManual() {
    el("manual-box").style.display = "none";
    el("manual-result").textContent = "";
  }
  async function addManual() {
    const ip = el("manual-ip").value.trim();
    const name = el("manual-name").value.trim() || `device-${ip.split(".").pop()}`;
    const community = el("manual-community").value.trim() || "public";
    const result = el("manual-result");
    if (!ip) {
      result.textContent = "! IP address is required.";
      return;
    }
    try {
      const response = await fetch("/api/discovery/register", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify([{ ip, name, community }]) });
      const data = await response.json();
      result.textContent = data.error || data.skipped.length ? `${ip} is already registered.` : `${ip} registered successfully.`;
      if (data.registered.length) {
        existingIps().add(ip);
        el("manual-ip").value = "";
        el("manual-name").value = "";
      }
    } catch (error) {
      result.textContent = String(error);
    }
  }
  window.addManual = addManual;
  window.cancelScan = cancelScan;
  window.hideManual = hideManual;
  window.registerSelected = registerSelected;
  window.showManual = showManual;
  window.startScan = startScan;
  window.toggleAll = toggleAll;
  window.updateHint = updateHint;
  window.updateRegisterBtn = updateRegisterBtn;
  window.applyPageLanguage = applyPageLanguage;
  return __toCommonJS(discovery_exports);
})();
