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
    runTopologyOnly: () => runTopologyOnly,
    showManual: () => showManual,
    startScan: () => startScan,
    toggleAll: () => toggleAll,
    updateHint: () => updateHint,
    updateRegisterBtn: () => updateRegisterBtn
  });

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
    const button = document.createElement("button");
    button.type = "button";
    button.className = "secret-toggle";
    button.setAttribute("aria-controls", input.id);
    button.innerHTML = eye;
    const update = () => {
      const visible = input.type === "text";
      const key = visible ? "hide_community" : "show_community";
      const fallback = visible ? "Hide community" : "Show community";
      const label = window.t?.(key) || fallback;
      button.setAttribute("aria-label", label);
      button.setAttribute("aria-pressed", String(visible));
      button.title = label;
    };
    button.addEventListener("click", () => {
      input.type = input.type === "password" ? "text" : "password";
      update();
    });
    input.before(wrapper);
    wrapper.append(input, button);
    update();
  }
  window.addEventListener("tracepulse-language-change", () => {
    document.querySelectorAll(".secret-toggle").forEach((button) => {
      const key = button.getAttribute("aria-pressed") === "true" ? "hide_community" : "show_community";
      const label = window.t?.(key);
      if (label) {
        button.setAttribute("aria-label", label);
        button.title = label;
      }
    });
  });

  // frontend/src/shared/snmp-credentials.ts
  var japanese = () => document.documentElement.lang === "ja";
  var text = (key, fallback) => window.t?.(key) || fallback;
  var credentialTitle = () => text("snmp_credential_title", japanese() ? "SNMP\u8A8D\u8A3C\u60C5\u5831" : "SNMP credentials");
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
    const hint = text(hintKey, global ? "Leave blank to keep existing defaults." : "Blank fields inherit global defaults. Saving replaces the current device overrides.");
    const optionKey = global ? "snmp_credential_unchanged" : "snmp_credential_global";
    const label = (key, fallback) => `<span data-i18n="snmp_credential_${key}">${esc(text(`snmp_credential_${key}`, fallback))}</span>`;
    const input = (key, fallback) => `<label for="${prefix}-${key}">${label(key, fallback)}<input id="${prefix}-${key}" data-snmp="${key}" type="password" autocomplete="new-password" maxlength="1024" placeholder="${esc(hint)}" data-i18n-placeholder="${hintKey}" aria-describedby="${prefix}-hint"></label>`;
    const protocolLabels = { "2c": "SNMPv2c", "3": "SNMPv3", sha1: "SHA-1", sha256: "SHA-256", sha512: "SHA-512", aes128: "AES-128" };
    const select = (key, fallback, options) => `<label for="${prefix}-${key}">${label(key, fallback)}<select id="${prefix}-${key}" data-snmp="${key}"><option value="" data-i18n="${optionKey}">${esc(text(optionKey, global ? "Unchanged" : "Global default"))}</option>${options.map((value) => value === "none" ? `<option value="none" data-i18n="snmp_credential_none">${esc(text("snmp_credential_none", "None"))}</option>` : `<option value="${value}">${esc(protocolLabels[value] || value)}</option>`).join("")}</select></label>`;
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

  // frontend/src/shared/status.ts
  function deviceStatusClass(status) {
    switch (status?.trim().toLowerCase()) {
      case "online":
        return "status-online";
      case "offline":
        return "status-offline";
      case "warning":
        return "status-warning";
      case "critical":
        return "status-critical";
      default:
        return "status-unknown";
    }
  }

  // frontend/src/shared/operation-progress.ts
  function showOperationProgress(container, label, message) {
    if (!document.getElementById("operation-progress-styles")) {
      const style = document.createElement("style");
      style.id = "operation-progress-styles";
      style.textContent = `.operation-progress{box-sizing:border-box;flex-basis:100%;width:100%;padding:12px;border:1px solid var(--tp-border,#475569);border-radius:4px;background:var(--tp-raised,#1a2b40);color:var(--tp-text,#e2e8f0)}.operation-progress progress{display:block;width:100%;height:8px;accent-color:var(--tp-accent,#38c5b2)}.operation-progress p{margin:8px 0 0;overflow-wrap:anywhere}`;
      document.head.append(style);
    }
    const element = document.createElement("div");
    element.className = "operation-progress";
    const bar = document.createElement("progress");
    bar.setAttribute("aria-label", label);
    const status = document.createElement("p");
    status.setAttribute("role", "status");
    status.setAttribute("aria-live", "polite");
    element.append(bar, status);
    container.append(element);
    const started = performance.now();
    const update = () => {
      status.textContent = message(Math.floor((performance.now() - started) / 1e3));
    };
    update();
    const timer = window.setInterval(update, 1e3);
    return () => {
      window.clearInterval(timer);
      element.remove();
    };
  }

  // frontend/src/pages/discovery.ts
  var resultRows = /* @__PURE__ */ new Map();
  var registeringIps = /* @__PURE__ */ new Set();
  var registrationBusy = false;
  var resultGeneration = 0;
  var registrationSummary = null;
  var scanTimeoutSeconds = 300;
  var pollIntervalMs = 1500;
  var currentJobId = null;
  var pollTimer = null;
  var elapsedTimer = null;
  var scanStartedAt = null;
  var cancelled = false;
  var scanCommunity = "";
  var scanGeneration = 0;
  function t(key) {
    return typeof window.t === "function" ? window.t(key) : key;
  }
  function el(id) {
    return document.getElementById(id);
  }
  function maxDiscoveryCidrs() {
    return window.TRACEPULSE_DISCOVERY_LIMITS?.maxCidrs ?? null;
  }
  function existingIps() {
    return window.DISCOVERY_EXISTING ?? (window.DISCOVERY_EXISTING = /* @__PURE__ */ new Set());
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
    const maxCidrs = maxDiscoveryCidrs();
    if (maxCidrs !== null && (entries.length > maxCidrs || smallestPrefix < 24)) {
      hint.textContent = `This deployment accepts up to ${maxCidrs} CIDR range(s) at /24 or narrower. Current: /${smallestPrefix}`;
      hint.className = "host-hint warn";
      return;
    }
    const seconds = Math.ceil(total * 0.5 / 256);
    hint.textContent = `${total.toLocaleString()} hosts \u2022 est. ~${seconds >= 60 ? `${Math.ceil(seconds / 60)} min` : `${seconds}s`}`;
    hint.className = "host-hint";
  }
  function applyPageLanguage(_lang) {
    window.refreshTopologyTexts?.();
    updateRegistrationRows();
    showRegistrationSummary();
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
    const maxCidrs = maxDiscoveryCidrs();
    if (maxCidrs !== null && (entries.length > maxCidrs || smallestPrefix < 24)) {
      showScanError(`This deployment accepts up to ${maxCidrs} CIDR range(s) at /24 or narrower.`);
      return;
    }
    const total = entries.map(cidrHostCount).reduce((sum, count) => sum + (count || 0), 0);
    const generation = ++scanGeneration;
    scanCommunity = community;
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
      const response = await fetch("/api/discovery/scan", { method: "POST", headers: { "Content-Type": "application/x-www-form-urlencoded", ...window.tracepulseRequestHeaders?.() }, body: `cidr=${encodeURIComponent(cidr)}&community=${encodeURIComponent(community)}&max_hosts=${encodeURIComponent(total || 65534)}` });
      const data = await response.json();
      if (generation !== scanGeneration) return;
      if (data.error || !data.job_id) {
        finishScanError(data.error || "Failed to start scan");
        return;
      }
      currentJobId = data.job_id;
      scanStartedAt = Date.now();
      if (seedIp && typeof window.startTopologyDiscovery === "function") window.startTopologyDiscovery(seedIp, community);
      startPolling(data.total || total || 0, generation);
    } catch (error) {
      if (generation === scanGeneration) finishScanError(`Failed to start scan: ${String(error)}`);
    }
  }
  async function runTopologyOnly() {
    const seedIp = el("seed-ip").value.trim();
    const community = el("community").value.trim() || "public";
    if (!seedIp) {
      showScanError(t("topology_seed_required"));
      return;
    }
    if (typeof window.startTopologyDiscovery !== "function") {
      showScanError("Topology discovery is unavailable.");
      return;
    }
    el("scan-error").style.display = "none";
    await window.startTopologyDiscovery(seedIp, community);
  }
  function startPolling(total, generation) {
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
      const job = currentJobId;
      try {
        const response = await fetch(`/api/discovery/scan/${job}`);
        const data = await response.json();
        if (cancelled || generation !== scanGeneration || currentJobId !== job) return;
        if (data.error) {
          finishScanError(data.error);
          return;
        }
        setProgress(data.scanned || 0, data.total || total, data.elapsed || 0);
        if (data.status === "done") finishScanDone(data.devices || [], data.scanned || 0, data.total || total, data.elapsed || 0);
        else if (data.status === "error") finishScanError(data.error || "Scan failed");
      } catch (error) {
        if (!cancelled && generation === scanGeneration) finishScanError(`Polling error: ${String(error)}`);
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
    scanGeneration++;
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
    resultGeneration++;
    resultRows = /* @__PURE__ */ new Map();
    registrationSummary = null;
    el("register-result").style.display = "none";
    const tbody = el("results-body");
    tbody.innerHTML = "";
    el("results-title").textContent = `${t("scan_results_label")} - ${devices.length} ${t("found")} (scanned ${scanned}/${total}, ${elapsed}s)`;
    if (devices.length === 0) tbody.innerHTML = `<tr><td colspan='5' class='empty'>${t("no_snmp_devices_found")}</td></tr>`;
    devices.forEach((device) => {
      const row = document.createElement("tr");
      row.innerHTML = `<td><input type='checkbox' class='row-cb' aria-label="${esc(`${t("select_device")} ${device.ip}`)}"></td><td>${esc(device.ip)}</td><td>${esc(device.name || "")}</td><td><span class='${deviceStatusClass(device.status)}'>${esc(device.status || "unknown")}</span></td><td class='registration-cell'></td>`;
      const checkbox = row.querySelector("input");
      checkbox.dataset.ip = device.ip;
      checkbox.onchange = updateRegisterBtn;
      resultRows.set(device.ip, { device, row, checkbox, cell: row.cells[4], community: scanCommunity });
      tbody.appendChild(row);
    });
    el("results-section").style.display = "block";
    el("select-all").checked = false;
    updateRegistrationRows();
  }
  function updateRegisterBtn() {
    const selectable = [...resultRows.values()].filter((state) => !existingIps().has(state.device.ip));
    const count = selectable.filter((state) => state.checkbox.checked).length;
    el("register-action").style.display = count > 0 ? "flex" : "none";
    el("select-count").textContent = count ? `${count} ${t("selected")}` : "";
    const button = el("register-btn");
    button.disabled = registrationBusy || count === 0;
    button.textContent = registrationBusy ? t("registering") : `${t("register")} (${count})`;
    const master = el("select-all");
    master.disabled = registrationBusy || selectable.length === 0;
    master.checked = selectable.length > 0 && count === selectable.length;
    master.indeterminate = count > 0 && count < selectable.length;
  }
  function toggleAll(master) {
    document.querySelectorAll(".row-cb:not(:disabled)").forEach((checkbox) => {
      checkbox.checked = master.checked;
    });
    updateRegisterBtn();
  }
  function updateRegistrationRows() {
    document.querySelector(".discovery-results-scroll")?.setAttribute("aria-label", t("scan_results_label"));
    for (const state of resultRows.values()) {
      const ip = state.device.ip;
      const registered = existingIps().has(ip);
      state.checkbox.disabled = registered || registrationBusy;
      state.checkbox.setAttribute("aria-label", `${t("select_device")} ${ip}`);
      if (registered) {
        state.checkbox.checked = false;
        state.error = void 0;
      }
      state.cell.replaceChildren();
      if (registered) {
        const badge = document.createElement("span");
        badge.className = "badge-registered";
        badge.textContent = t("registered");
        state.cell.append(badge);
      } else if (registeringIps.has(ip)) {
        const text2 = document.createElement("span");
        text2.setAttribute("role", "status");
        text2.textContent = t("registering");
        const progress = document.createElement("progress");
        progress.className = "registration-row-progress";
        progress.setAttribute("aria-label", t("registering"));
        state.cell.append(text2, progress);
      } else {
        const button = document.createElement("button");
        button.type = "button";
        button.className = "btn btn-secondary btn-xs register-device";
        button.textContent = t("register");
        button.disabled = registrationBusy;
        button.onclick = () => {
          if (resultRows.get(ip) !== state) return;
          void registerDevices([{ ip, name: state.device.name, community: state.community }]);
        };
        state.cell.append(button);
        if (state.error) {
          const error = document.createElement("span");
          error.className = "registration-error";
          error.setAttribute("role", "alert");
          error.textContent = t(state.error);
          state.cell.append(error);
        }
      }
    }
    updateRegisterBtn();
    document.querySelectorAll("#manual-box button").forEach((button) => {
      button.disabled = registrationBusy;
    });
  }
  function showRegistrationSummary() {
    if (!registrationSummary) return;
    const { registered, skipped, failed } = registrationSummary;
    const result = el("register-result");
    result.className = failed ? "reg-error" : "reg-success";
    result.textContent = t("registration_summary").replace("{registered}", String(registered)).replace("{skipped}", String(skipped)).replace("{failed}", String(failed));
    result.style.display = "block";
  }
  function registrationError(value) {
    return typeof value === "string" && /device registration limit reached|node limit|registration limit/i.test(value) ? "registration_limit" : "registration_failed";
  }
  async function requestRegistration(targets) {
    const failed = (key) => ({ registered: [], skipped: [], failures: new Map(targets.map((device) => [device.ip, key])) });
    try {
      const response = await fetch("/api/discovery/register", { method: "POST", headers: { "Content-Type": "application/json", ...window.tracepulseRequestHeaders?.() }, body: JSON.stringify(targets) });
      if (response.status === 401 || response.status === 403) return failed("registration_denied");
      const data = await response.json();
      if (!response.ok || !data || data.error) return failed(registrationError(data?.error));
      const strings = (value) => Array.isArray(value) && value.every((item) => typeof item === "string");
      if (!strings(data.registered) || !strings(data.skipped) || !strings(data.errors)) return failed("registration_failed");
      const outcome = { registered: [], skipped: [], failures: /* @__PURE__ */ new Map() };
      for (const target of targets) {
        const error = data.errors.find((message) => message.startsWith(`${target.ip}:`));
        if (error) outcome.failures.set(target.ip, registrationError(error));
        else if (data.registered.includes(target.ip)) outcome.registered.push(target.ip);
        else if (data.skipped.includes(target.ip)) outcome.skipped.push(target.ip);
        else outcome.failures.set(target.ip, "registration_failed");
      }
      return outcome;
    } catch {
      return failed("registration_network_error");
    }
  }
  async function registerDevices(targets, progressAnchor = el("register-result")) {
    if (registrationBusy || targets.some((target) => registeringIps.has(target.ip))) return null;
    targets = targets.filter((target) => !existingIps().has(target.ip));
    if (!targets.length) {
      updateRegistrationRows();
      return null;
    }
    registrationBusy = true;
    const generation = resultGeneration;
    const rows = new Map(resultRows);
    for (const target of targets) {
      registeringIps.add(target.ip);
      const row = rows.get(target.ip);
      if (row) row.error = void 0;
    }
    updateRegistrationRows();
    const container = document.createElement("div");
    progressAnchor.before(container);
    const stop = showOperationProgress(container, t("registering"), (seconds) => t("registration_progress").replace("{count}", String(targets.length)).replace("{seconds}", String(seconds)));
    if (targets.length > 1) container.scrollIntoView({ block: "nearest" });
    try {
      const outcome = await requestRegistration(targets);
      for (const ip of [...outcome.registered, ...outcome.skipped]) existingIps().add(ip);
      if (generation === resultGeneration) {
        for (const [ip, error] of outcome.failures) {
          const state = rows.get(ip);
          if (state && resultRows.get(ip) === state) state.error = error;
        }
        registrationSummary = { registered: outcome.registered.length, skipped: outcome.skipped.length, failed: outcome.failures.size };
        showRegistrationSummary();
      }
      return outcome;
    } finally {
      stop();
      container.remove();
      registrationBusy = false;
      for (const target of targets) registeringIps.delete(target.ip);
      updateRegistrationRows();
    }
  }
  async function registerSelected() {
    const selected = [...resultRows.values()].filter((state) => state.checkbox.checked && !existingIps().has(state.device.ip)).map((state) => ({ ip: state.device.ip, name: state.device.name, community: state.community }));
    if (!selected.length) {
      window.alert(t("select_at_least_one_device"));
      return;
    }
    await registerDevices(selected);
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
    const community = el("manual-community").value;
    const snmp_credentials = readCredentials(el("manual-box"));
    const result = el("manual-result");
    if (!ip) {
      result.textContent = t("registration_ip_required");
      return;
    }
    if (existingIps().has(ip)) {
      result.textContent = `${ip} ${t("registered")}`;
      updateRegistrationRows();
      return;
    }
    const outcome = await registerDevices([{ ip, name, community, snmp_credentials }], result);
    if (!outcome) return;
    const error = outcome.failures.get(ip);
    result.textContent = error ? t(error) : `${ip} ${outcome.registered.length ? t("registration_manual_success") : t("registered")}`;
    if (!error) {
      el("manual-ip").value = "";
      el("manual-name").value = "";
      el("manual-community").value = "";
      clearCredentials(el("manual-box"));
    }
  }
  window.addManual = addManual;
  window.cancelScan = cancelScan;
  window.hideManual = hideManual;
  window.registerSelected = registerSelected;
  window.runTopologyOnly = runTopologyOnly;
  window.showManual = showManual;
  window.startScan = startScan;
  window.toggleAll = toggleAll;
  window.updateHint = updateHint;
  window.updateRegisterBtn = updateRegisterBtn;
  window.applyPageLanguage = applyPageLanguage;
  var manualCommunity = document.getElementById("manual-community");
  manualCommunity?.parentElement?.insertAdjacentHTML("afterend", credentialFields("manual-snmp", false, false));
  for (const id of ["community", "manual-community"]) {
    const input = document.getElementById(id);
    if (input instanceof HTMLInputElement) addSecretVisibility(input);
  }
  return __toCommonJS(discovery_exports);
})();
