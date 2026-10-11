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

  // frontend/src/shared/onboarding.ts
  var storageKey = "tracepulse-onboarding-v1";
  var copy = {
    en: {
      guide: "Setup Guide",
      welcome: "Welcome to TracePulse",
      intro: "Start monitoring your network by discovering and registering SNMP devices.",
      discovery: "Discover and register devices",
      cidr: "IP range (IPv4 CIDR)",
      community: "SNMP v2c community",
      scan: "Discover",
      scanning: "Discovering devices...",
      next: "Next",
      back: "Back",
      skip: "Skip",
      finish: "Finish",
      register: "Register selected devices",
      empty: "No SNMP devices found. Check the range, community and connectivity, then try again.",
      selected: "Select devices to monitor",
      registered: "Devices registered. Monitoring starts automatically.",
      alerts: "Alerts (optional)",
      alertInfo: "TracePulse records alerts locally and displays them on the dashboard. Adjust thresholds in Settings when needed. Outbound notifications are not available in Community edition.",
      settings: "Alert settings",
      rangeError: "Enter one valid IPv4 CIDR range at /24 or narrower.",
      failure: "The operation failed. Please retry.",
      timeout: "Discovery timed out. Check connectivity and retry.",
      progress: "Discovery progress",
      step: "Step",
      continue: "Continue without registration",
      privacy: "The community is used for this discovery and registered devices only."
    },
    ja: {
      guide: "\u30BB\u30C3\u30C8\u30A2\u30C3\u30D7\u30AC\u30A4\u30C9",
      welcome: "TracePulse\u3078\u3088\u3046\u3053\u305D",
      intro: "\u30CD\u30C3\u30C8\u30EF\u30FC\u30AF\u5185\u306ESNMP\u6A5F\u5668\u3092\u63A2\u7D22\u30FB\u767B\u9332\u3057\u3066\u3001\u76E3\u8996\u3092\u59CB\u3081\u307E\u3057\u3087\u3046\u3002",
      discovery: "\u6A5F\u5668\u306E\u63A2\u7D22\u30FB\u767B\u9332",
      cidr: "IP\u7BC4\u56F2\uFF08IPv4 CIDR\uFF09",
      community: "SNMP v2c\u30B3\u30DF\u30E5\u30CB\u30C6\u30A3",
      scan: "\u63A2\u7D22\u3059\u308B",
      scanning: "\u6A5F\u5668\u3092\u63A2\u7D22\u4E2D...",
      next: "\u6B21\u3078",
      back: "\u623B\u308B",
      skip: "\u30B9\u30AD\u30C3\u30D7",
      finish: "\u5B8C\u4E86",
      register: "\u9078\u629E\u3057\u305F\u6A5F\u5668\u3092\u767B\u9332",
      empty: "SNMP\u6A5F\u5668\u304C\u898B\u3064\u304B\u308A\u307E\u305B\u3093\u3067\u3057\u305F\u3002IP\u7BC4\u56F2\u30FB\u30B3\u30DF\u30E5\u30CB\u30C6\u30A3\u30FB\u63A5\u7D9A\u3092\u78BA\u8A8D\u3057\u3066\u518D\u63A2\u7D22\u3057\u3066\u304F\u3060\u3055\u3044\u3002",
      selected: "\u76E3\u8996\u3059\u308B\u6A5F\u5668\u3092\u9078\u629E",
      registered: "\u6A5F\u5668\u3092\u767B\u9332\u3057\u307E\u3057\u305F\u3002\u76E3\u8996\u306F\u81EA\u52D5\u7684\u306B\u958B\u59CB\u3055\u308C\u307E\u3059\u3002",
      alerts: "\u30A2\u30E9\u30FC\u30C8\u8A2D\u5B9A\uFF08\u4EFB\u610F\uFF09",
      alertInfo: "TracePulse\u306F\u30A2\u30E9\u30FC\u30C8\u3092\u30ED\u30FC\u30AB\u30EB\u306B\u8A18\u9332\u3057\u3001\u30C0\u30C3\u30B7\u30E5\u30DC\u30FC\u30C9\u306B\u8868\u793A\u3057\u307E\u3059\u3002\u5FC5\u8981\u306B\u5FDC\u3058\u3066\u8A2D\u5B9A\u753B\u9762\u3067\u3057\u304D\u3044\u5024\u3092\u5909\u66F4\u3067\u304D\u307E\u3059\u3002Community\u7248\u3067\u306F\u5916\u90E8\u3078\u306E\u901A\u77E5\u9001\u4FE1\u306F\u5229\u7528\u3067\u304D\u307E\u305B\u3093\u3002",
      settings: "\u30A2\u30E9\u30FC\u30C8\u8A2D\u5B9A\u3078",
      rangeError: "/24\u4EE5\u4E0A\u306E\u30D7\u30EC\u30D5\u30A3\u30C3\u30AF\u30B9\u3067\u3001\u6709\u52B9\u306AIPv4 CIDR\u30921\u3064\u5165\u529B\u3057\u3066\u304F\u3060\u3055\u3044\u3002",
      failure: "\u51E6\u7406\u306B\u5931\u6557\u3057\u307E\u3057\u305F\u3002\u518D\u8A66\u884C\u3057\u3066\u304F\u3060\u3055\u3044\u3002",
      timeout: "\u63A2\u7D22\u304C\u30BF\u30A4\u30E0\u30A2\u30A6\u30C8\u3057\u307E\u3057\u305F\u3002\u63A5\u7D9A\u3092\u78BA\u8A8D\u3057\u3066\u518D\u8A66\u884C\u3057\u3066\u304F\u3060\u3055\u3044\u3002",
      progress: "\u63A2\u7D22\u306E\u9032\u6357",
      step: "\u30B9\u30C6\u30C3\u30D7",
      continue: "\u767B\u9332\u305B\u305A\u306B\u6B21\u3078",
      privacy: "\u30B3\u30DF\u30E5\u30CB\u30C6\u30A3\u306F\u4ECA\u56DE\u306E\u63A2\u7D22\u3068\u767B\u9332\u3059\u308B\u6A5F\u5668\u306B\u306E\u307F\u4F7F\u7528\u3057\u307E\u3059\u3002"
    }
  };
  function initializeOnboarding(auto = false) {
    const text = (key) => copy[document.documentElement.lang === "ja" ? "ja" : "en"][key];
    const style = document.createElement("style");
    style.textContent = `
        .setup-dialog{box-sizing:border-box;inset:0;margin:auto;width:min(600px,calc(100% - 24px));max-height:calc(100dvh - 24px);padding:24px;border:1px solid var(--border,#475569);border-radius:8px;background:var(--panel-bg,#1e293b);color:var(--text,#e2e8f0);overflow:auto}
        .setup-dialog::backdrop{background:rgb(0 0 0 / .6)}
        .setup-dialog h2{font-size:1.25rem;margin:12px 0}.setup-dialog p{line-height:1.6;overflow-wrap:anywhere}
        .setup-dialog form{display:grid;gap:12px}.setup-dialog label{display:grid;gap:6px}
        .setup-dialog input:not([type=checkbox]),.setup-dialog select{box-sizing:border-box;width:100%;min-width:0;padding:10px;border-radius:4px;border:1px solid var(--border,#64748b);background:var(--input-bg,#0f172a);color:inherit}
        .setup-actions{display:flex;gap:8px;flex-wrap:wrap;justify-content:flex-end;margin-top:20px}
        .setup-dialog button,.setup-launch{padding:8px 14px;cursor:pointer;border-radius:4px;border:1px solid var(--border,#64748b);background:var(--input-bg,#0f172a);color:inherit;white-space:normal}
        .setup-dialog button:disabled{opacity:.5;cursor:default}.setup-dialog .setup-primary{background:#0369a1;color:white;border-color:#0369a1}
        .setup-dialog .setup-results{display:grid;gap:8px;max-height:220px;overflow:auto;padding:8px 0}
        .setup-dialog .setup-device{display:flex;align-items:center;gap:10px;overflow-wrap:anywhere}.setup-dialog progress{width:100%}
        .setup-dialog [role=alert]{color:#f87171}.setup-step{font-size:.85rem}.setup-launch{margin-bottom:16px}
        :root[data-theme=light] .setup-dialog{background:#fff;color:#1e293b}:root[data-theme=light] .setup-dialog input,:root[data-theme=light] .setup-dialog select,:root[data-theme=light] .setup-dialog button:not(.setup-primary),:root[data-theme=light] .setup-launch{background:#f1f5f9;color:#1e293b}
        @media(max-width:480px){.setup-dialog{padding:16px}.setup-actions button{flex:1 1 auto}}
    `;
    document.head.append(style);
    const dialog = document.createElement("dialog");
    dialog.className = "setup-dialog";
    dialog.id = "setup-guide";
    dialog.setAttribute("aria-labelledby", "setup-title");
    document.body.append(dialog);
    let step = 0;
    let choosingLanguage = false;
    let languageChoiceEnabled = false;
    let busy = false;
    let devices = [];
    let registered = false;
    let controller = new AbortController();
    let timer;
    let deadline;
    let previousFocus = null;
    let dismissed = false;
    try {
      dismissed = localStorage.getItem(storageKey) === "done";
    } catch {
    }
    const stop = () => {
      controller.abort();
      window.clearTimeout(timer);
      window.clearTimeout(deadline);
      controller = new AbortController();
      busy = false;
    };
    const remember = () => {
      dismissed = true;
      if (auto) launch.hidden = true;
      try {
        localStorage.setItem(storageKey, "done");
      } catch {
      }
    };
    const close = () => {
      stop();
      remember();
      dialog.close();
      dialog.replaceChildren();
      devices = [];
      if (previousFocus?.hidden) {
        const heading = document.querySelector("main h1");
        heading?.setAttribute("tabindex", "-1");
        heading?.focus();
      } else previousFocus?.focus();
      window.refreshDashboard?.();
    };
    const status = (message, error = false) => {
      const node = dialog.querySelector("#setup-status");
      if (node) {
        node.textContent = message;
        node.setAttribute("role", error ? "alert" : "status");
      }
    };
    const setBusy = (value) => {
      busy = value;
      dialog.querySelectorAll("button:not([data-action=skip])").forEach((button) => {
        button.disabled = value;
      });
      dialog.querySelectorAll("input").forEach((input) => {
        input.disabled = value;
      });
    };
    const request = async (url, signal, body) => {
      const response = await fetch(url, {
        signal,
        ...body ? { method: "POST", headers: { "Content-Type": body instanceof URLSearchParams ? "application/x-www-form-urlencoded" : "application/json", "X-CSRF-Token": window.tracepulseCsrf || "" }, body: body instanceof URLSearchParams ? body.toString() : JSON.stringify(body) } : {}
      });
      const data = await response.json();
      if (!response.ok || data.error) throw new Error(data.error || `HTTP ${response.status}`);
      return data;
    };
    const render = () => {
      if (choosingLanguage) {
        dialog.innerHTML = `<h2 id="setup-title" tabindex="-1">Choose Language</h2><form id="setup-language-form"><label for="setup-language">Language</label><select id="setup-language" required><option value="" disabled selected>Select a language</option><option value="ja">Japanese</option><option value="en">English</option></select><div class="setup-actions"><button type="submit" class="setup-primary">Continue</button></div></form>`;
        dialog.querySelector("#setup-language")?.focus();
        dialog.querySelector("form")?.addEventListener("submit", (event) => {
          event.preventDefault();
          const language = dialog.querySelector("#setup-language").value;
          if (language !== "ja" && language !== "en") return;
          choosingLanguage = false;
          window.setLanguage?.(language);
          window.refreshDashboard?.();
          render();
        });
        return;
      }
      const titles = ["welcome", "discovery", "alerts"];
      dialog.innerHTML = `<div class="setup-step">${text("step")} ${step + 1} / 3</div><h2 id="setup-title" tabindex="-1">${text(titles[step])}</h2>` + (step === 0 ? `<p>${text("intro")}</p>` : step === 1 ? `<form id="setup-form"><label>${text("cidr")}<input id="setup-cidr" required placeholder="192.168.1.0/24" autocomplete="off"></label><label>${text("community")}<input id="setup-community" type="password" required value="public" autocomplete="off"></label><p>${text("privacy")}</p><button class="setup-primary" type="submit">${text("scan")}</button></form><progress id="setup-progress" hidden aria-label="${text("progress")}"></progress><div id="setup-results" class="setup-results"></div><button type="button" data-action="register" hidden>${text("register")}</button><p id="setup-status" role="status" aria-live="polite"></p>` : `<p>${text("alertInfo")}</p><a href="/settings" id="setup-settings">${text("settings")}</a>`) + `<div class="setup-actions"><button type="button" data-action="skip">${text("skip")}</button>${step > 0 || step === 0 && languageChoiceEnabled ? `<button type="button" data-action="${step === 0 ? "language-back" : "back"}">${text("back")}</button>` : ""}<button type="button" class="setup-primary" data-action="next">${text(step === 2 ? "finish" : step === 1 && !registered ? "continue" : "next")}</button></div>`;
      dialog.querySelector("#setup-title")?.focus();
      dialog.querySelector("#setup-settings")?.addEventListener("click", close);
      dialog.querySelector("form")?.addEventListener("submit", (event) => {
        event.preventDefault();
        void scan();
      });
    };
    const scan = async () => {
      if (busy) return;
      const cidr = dialog.querySelector("#setup-cidr").value.trim();
      const community = dialog.querySelector("#setup-community").value;
      const match = cidr.match(/^(\d{1,3}(?:\.\d{1,3}){3})\/(\d{1,2})$/);
      if (!match || match[1].split(".").some((part) => Number(part) > 255) || Number(match[2]) < 24 || Number(match[2]) > 32) {
        status(text("rangeError"), true);
        return;
      }
      stop();
      devices = [];
      registered = false;
      dialog.querySelector("#setup-results").replaceChildren();
      dialog.querySelector("[data-action=register]").hidden = true;
      const signal = controller.signal;
      setBusy(true);
      status(text("scanning"));
      const progress = dialog.querySelector("#setup-progress");
      progress.hidden = false;
      progress.removeAttribute("value");
      const fail = (error) => {
        if (signal.aborted) return;
        stop();
        setBusy(false);
        progress.hidden = true;
        status(error instanceof Error ? error.message : text("failure"), true);
      };
      deadline = window.setTimeout(() => fail(new Error(text("timeout"))), 3e5);
      try {
        const prefix = Number(match[2]);
        const hosts = prefix >= 31 ? 2 ** (32 - prefix) : 2 ** (32 - prefix) - 2;
        const job = await request("/api/discovery/scan", signal, new URLSearchParams({ cidr, community, max_hosts: String(hosts) }));
        if (signal.aborted) return;
        if (!job.job_id) throw new Error(text("failure"));
        const poll = async () => {
          try {
            const result = await request(`/api/discovery/scan/${encodeURIComponent(job.job_id)}`, signal);
            if (signal.aborted) return;
            progress.max = result.total || hosts;
            progress.value = result.scanned || 0;
            status(`${text("scanning")} ${result.scanned || 0} / ${result.total || hosts}`);
            if (result.status === "error") throw new Error(text("failure"));
            if (result.status !== "done") {
              timer = window.setTimeout(() => void poll(), 1500);
              return;
            }
            const existing = await request("/api/devices", signal);
            if (signal.aborted) return;
            window.clearTimeout(deadline);
            const known = new Set(existing.map((device) => device.ip));
            devices = (result.devices || []).filter((device) => !known.has(device.ip)).map((device) => ({ ...device, community: device.community || community }));
            setBusy(false);
            progress.hidden = true;
            status(devices.length ? text("selected") : text("empty"));
            dialog.querySelector("#setup-results").innerHTML = devices.map((device, index) => `<label class="setup-device"><input type="checkbox" value="${index}" checked><span>${esc(device.ip)} ${esc(device.name || "")}</span></label>`).join("");
            dialog.querySelector("[data-action=register]").hidden = devices.length === 0;
          } catch (error) {
            fail(error);
          }
        };
        await poll();
      } catch (error) {
        fail(error);
      }
    };
    const register = async () => {
      if (busy) return;
      const selected = Array.from(dialog.querySelectorAll("#setup-results input:checked")).map((input) => devices[Number(input.value)]);
      if (!selected.length) {
        status(text("selected"), true);
        return;
      }
      const signal = controller.signal;
      setBusy(true);
      try {
        const result = await request("/api/discovery/register", signal, selected);
        if (signal.aborted) return;
        const accepted = /* @__PURE__ */ new Set([...result.registered, ...result.skipped]);
        dialog.querySelectorAll("#setup-results input").forEach((input) => {
          if (accepted.has(devices[Number(input.value)].ip)) input.closest("label")?.remove();
        });
        registered = result.registered.length > 0 || result.skipped.length > 0;
        status(result.errors.length ? result.errors.join("\n") : registered ? text("registered") : text("failure"), result.errors.length > 0 || !registered);
        window.refreshDashboard?.();
        dialog.querySelector("[data-action=next]").textContent = text(registered ? "next" : "continue");
        dialog.querySelector("[data-action=register]").hidden = dialog.querySelectorAll("#setup-results input").length === 0;
      } catch (error) {
        if (!signal.aborted) status(error instanceof Error ? error.message : text("failure"), true);
      } finally {
        if (!signal.aborted) setBusy(false);
      }
    };
    dialog.addEventListener("click", (event) => {
      const action = event.target.closest("[data-action]")?.dataset.action;
      if (action === "skip") close();
      else if (action === "language-back" && languageChoiceEnabled) {
        choosingLanguage = true;
        render();
      } else if (!busy && action === "register") void register();
      else if (!busy && (action === "back" || action === "next")) {
        if (action === "next" && step === 2) {
          close();
          return;
        }
        stop();
        devices = [];
        step += action === "back" ? -1 : 1;
        render();
      }
    });
    dialog.addEventListener("cancel", (event) => {
      event.preventDefault();
      close();
    });
    const open = (selectLanguage = false) => {
      if (dialog.open) return;
      previousFocus = document.activeElement;
      stop();
      step = 0;
      registered = false;
      languageChoiceEnabled = selectLanguage;
      choosingLanguage = selectLanguage;
      dialog.showModal();
      render();
    };
    const launch = document.createElement("button");
    launch.className = "setup-launch";
    launch.type = "button";
    launch.textContent = text("guide");
    launch.hidden = auto && dismissed;
    launch.addEventListener("click", () => open());
    document.querySelector("main h1")?.after(launch);
    window.addEventListener("tracepulse-language-change", () => {
      launch.textContent = text("guide");
      if (dialog.open && step !== 1) render();
    });
    if (auto) {
      if (!dismissed) void request("/api/devices", controller.signal).then((existing) => {
        if (existing.length === 0) open(true);
      }).catch(() => {
      });
    }
  }

  // frontend/src/pages/dashboard.ts
  var columns = ["ip", "name", "status", "health_score", "community", "last_seen"];
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
  function statusClass(status) {
    switch (status) {
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
  function compareValue(a, b, column) {
    if (column === "ip") return ipToNumber(a.ip) - ipToNumber(b.ip);
    if (column === "status") return statusOrder(a.status) - statusOrder(b.status);
    if (column === "health_score") return (a.health_score ?? -1) - (b.health_score ?? -1);
    const left = String(a[column] || "").toLowerCase();
    const right = String(b[column] || "").toLowerCase();
    return left < right ? -1 : left > right ? 1 : 0;
  }
  function currentDevices() {
    return window.DEVICES || [];
  }
  function renderAttention(devices = currentDevices()) {
    const element = document.getElementById("device-attention");
    if (!element) return;
    const attention = devices.filter((device) => device.error_spike || device.status === "offline" || device.status === "critical" || device.status === "warning");
    element.classList.toggle("has-items", attention.length > 0);
    if (attention.length === 0) {
      element.innerHTML = "";
      return;
    }
    const items = attention.map((device) => {
      const reasons = [];
      if (device.error_spike) reasons.push(t("error_spikes"));
      if (device.status === "offline") reasons.push(t("offline"));
      else if (device.status === "critical") reasons.push("Critical");
      else if (device.status === "warning") reasons.push(t("warning"));
      return `<li class='attention-item'><a href='/device/${encodeURIComponent(device.ip)}'>${esc(device.name || device.ip)}</a><span class='attention-reason'>${esc(reasons.join(" / "))}</span></li>`;
    }).join("");
    element.innerHTML = `<h2>${t("attention_devices")}</h2><ul class='attention-list'>${items}</ul>`;
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
      tbody.innerHTML = `<tr><td colspan=7 class=empty>${t("no_devices")} <a href=/discovery>${t("nav_discovery")}</a></td></tr>`;
      return;
    }
    tbody.innerHTML = sorted.map((device) => {
      const statusBadgeClass = statusClass(device.status);
      const rowClass = device.error_spike ? " class=spike-row" : "";
      const spikeBadge = device.error_spike ? "<span class=spike-badge>&#9888; Error Spike</span>" : "";
      const deleting = deletingIps.has(device.ip);
      const actionLabel = deleting ? t("unregistering") : t("unregister");
      const encodedIp = encodeURIComponent(device.ip);
      const encodedName = encodeURIComponent(device.name || device.ip);
      const actionButton = `<button class='btn-row-danger' data-ip="${esc(encodedIp)}" data-name="${esc(encodedName)}" onclick='unregisterDevice(this.dataset.ip,this.dataset.name)' ${deleting ? "disabled" : ""}>${actionLabel}</button>`;
      const lastSeen = fmtTime(device.last_seen || device.last_seen_at);
      const healthBadgeClass = statusClass(device.health_status);
      const health = typeof device.health_score === "number" ? `<span class="${esc(healthBadgeClass)}">${device.health_score}/100</span><meter min='0' max='100' value='${device.health_score}' aria-label="${esc(t("health_score"))}" style='width:4rem;margin-left:.5rem;vertical-align:middle'></meter>` : "-";
      return `<tr${rowClass}><td><a href="/device/${esc(encodeURIComponent(device.ip))}" style='color:#38bdf8;text-decoration:none'>${esc(device.ip)}</a></td><td>${esc(device.name || "")}${spikeBadge}</td><td><span class="${esc(statusBadgeClass)}">${esc(device.status)}</span></td><td>${health}</td><td>${esc(device.community)}</td><td>${lastSeen}</td><td><div class='row-actions'>${actionButton}</div></td></tr>`;
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
    let critical = 0;
    let offline = 0;
    let spikes = 0;
    devices.forEach((device) => {
      if (device.status === "online") online++;
      else if (device.status === "warning") warning++;
      else if (device.status === "critical") critical++;
      else if (device.status === "offline") offline++;
      if (device.error_spike) spikes++;
    });
    const element = document.getElementById("summary-cards");
    if (!element) return;
    const spikeClass = spikes > 0 ? "card card-spike" : "card";
    element.innerHTML = `<div class='card card-online'><div class='card-num'>${online}</div><div class='card-label'>${t("online")}</div></div><div class='card card-warning'><div class='card-num'>${warning}</div><div class='card-label'>${t("warning")}</div></div><div class='card card-offline'><div class='card-num'>${critical}</div><div class='card-label'>Critical</div></div><div class='card card-offline'><div class='card-num'>${offline}</div><div class='card-label'>${t("offline")}</div></div><div class='card'><div class='card-num'>${devices.length}</div><div class='card-label'>${t("total")}</div></div><div class='${spikeClass}'><div class='card-num'>${spikes}</div><div class='card-label'>${t("error_spikes")}</div></div>`;
  }
  function unregisterDevice(encodedIp, encodedName) {
    const ip = decodeURIComponent(encodedIp || "");
    const name = decodeURIComponent(encodedName || "");
    if (!window.confirm(`${t("unregister_confirm")}

${ip}${name ? ` (${name})` : ""}`)) return;
    deletingIps.add(ip);
    renderTable();
    fetch(`/api/device/${encodeURIComponent(ip)}`, { method: "DELETE", headers: { "X-CSRF-Token": window.tracepulseCsrf || "" } }).then(async (response) => ({ ok: response.ok, status: response.status, body: await response.json() })).then((result) => {
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
      renderAttention(devices);
      updateLastRefreshed();
    }).catch((error) => console.warn("refresh failed", error));
  }
  window.renderTable = renderTable;
  window.renderSummary = renderSummary;
  window.refreshDashboard = refresh;
  window.sortBy = sortBy;
  window.unregisterDevice = unregisterDevice;
  renderTable();
  renderSummary();
  renderAttention();
  updateSortIndicators();
  updateLastRefreshed();
  window.setInterval(refresh, 3e4);
  document.addEventListener("DOMContentLoaded", () => initializeOnboarding(true));
  return __toCommonJS(dashboard_exports);
})();
