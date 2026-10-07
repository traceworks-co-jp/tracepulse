"use strict";
var TracePulseSettings = (() => {
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

  // frontend/src/pages/settings.ts
  var settings_exports = {};
  __export(settings_exports, {
    populateForm: () => populateForm,
    resetDefaults: () => resetDefaults,
    saveSettings: () => saveSettings,
    updateBar: () => updateBar
  });

  // frontend/src/shared/escape.ts
  function esc(value) {
    return String(value === null || value === void 0 ? "" : value).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/\x22/g, "&quot;");
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
    const stop = () => {
      controller.abort();
      window.clearTimeout(timer);
      window.clearTimeout(deadline);
      controller = new AbortController();
      busy = false;
    };
    const remember = () => {
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
      previousFocus?.focus();
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
    launch.addEventListener("click", () => open());
    document.querySelector("main h1")?.after(launch);
    window.addEventListener("tracepulse-language-change", () => {
      launch.textContent = text("guide");
      if (dialog.open && step !== 1) render();
    });
    if (auto) {
      let dismissed = false;
      try {
        dismissed = localStorage.getItem(storageKey) === "done";
      } catch {
      }
      if (!dismissed) void request("/api/devices", controller.signal).then((existing) => {
        if (existing.length === 0) open(true);
      }).catch(() => {
      });
    }
  }

  // frontend/src/pages/settings.ts
  var metrics = ["cpu", "memory", "bandwidth", "error_rate"];
  function element(id) {
    return document.getElementById(id);
  }
  function t(key) {
    return typeof window.t === "function" ? window.t(key) : key;
  }
  function populateForm(values) {
    element("interval").value = String(values.interval);
    element("community").value = values.community;
    element("timezone").value = values.timezone || "utc";
    try {
      localStorage.setItem("tracepulse-timezone", element("timezone").value);
    } catch {
    }
    for (const metric of metrics) {
      const thresholds = values.thresholds?.[metric] || (metric === "error_rate" ? { warning: 5, critical: 10 } : { warning: 80, critical: 90 });
      element(`${metric}_warning`).value = String(thresholds.warning);
      element(`${metric}_critical`).value = String(thresholds.critical);
    }
    element("spike").value = String(values.spike);
    element("warn_t").value = String(values.warn);
    element("crit_t").value = String(values.crit);
    element("days").value = String(values.days);
    updateBar();
  }
  function updateBar() {
    const warning = element("warn_t").valueAsNumber;
    const critical = element("crit_t").valueAsNumber;
    const valid = Number.isInteger(warning) && Number.isInteger(critical) && warning > critical && warning <= 100 && critical >= 0;
    element("bar-label").hidden = !valid;
    if (!valid) return;
    const bar = element("tbar");
    bar.style.setProperty("--warn", `${warning}%`);
    bar.style.setProperty("--crit", `${critical}%`);
    const ranges = { warning, critical, upper: warning - 1 };
    for (const status of ["healthy", "warning", "critical"]) {
      element(`legend-${status}`).textContent = window.tf?.(`score_legend_${status}`, ranges) || t(`score_legend_${status}`);
    }
  }
  function showToast(ok, message) {
    const success = element("toast-ok");
    const failure = element("toast-err");
    success.style.display = "none";
    failure.style.display = "none";
    if (ok) {
      success.style.display = "inline-block";
      window.setTimeout(() => {
        success.style.display = "none";
      }, 3e3);
    } else {
      failure.textContent = `x ${message || "Unknown error"}`;
      failure.style.display = "inline-block";
      window.setTimeout(() => {
        failure.style.display = "none";
      }, 5e3);
    }
  }
  function saveSettings() {
    const timezone = element("timezone").value;
    const thresholds = {};
    for (const metric of metrics) {
      const warningInput = element(`${metric}_warning`);
      const criticalInput = element(`${metric}_critical`);
      const warning = warningInput.valueAsNumber;
      const critical = criticalInput.valueAsNumber;
      if (!warningInput.checkValidity() || !criticalInput.checkValidity() || !Number.isFinite(warning) || !Number.isFinite(critical) || warning >= critical) {
        showToast(false, t("metric_threshold_invalid"));
        warningInput.focus();
        return;
      }
      thresholds[metric] = { warning, critical };
    }
    const payload = {
      interval_seconds: parseInt(element("interval").value, 10),
      default_community: element("community").value,
      timezone,
      ...thresholds,
      spike_threshold: parseInt(element("spike").value, 10),
      health_warning_threshold: parseInt(element("warn_t").value, 10),
      health_critical_threshold: parseInt(element("crit_t").value, 10),
      history_days: parseInt(element("days").value, 10)
    };
    fetch("/api/settings", { method: "POST", headers: { "Content-Type": "application/json", "X-CSRF-Token": window.tracepulseCsrf || "" }, body: JSON.stringify(payload) }).then((response) => response.json()).then((data) => {
      if (data.ok) {
        try {
          localStorage.setItem("tracepulse-timezone", timezone);
        } catch {
        }
        showToast(true);
      } else showToast(false, data.error);
    }).catch((error) => showToast(false, String(error)));
  }
  function resetDefaults() {
    populateForm({ interval: 30, community: "public", timezone: "utc", error_rate: 0.05, spike: 10, warn: 80, crit: 60, days: 7 });
  }
  window.populateForm = populateForm;
  window.resetDefaults = resetDefaults;
  window.saveSettings = saveSettings;
  window.updateBar = updateBar;
  populateForm(window.INIT);
  window.addEventListener("tracepulse-language-change", updateBar);
  document.addEventListener("DOMContentLoaded", () => initializeOnboarding());
  return __toCommonJS(settings_exports);
})();
