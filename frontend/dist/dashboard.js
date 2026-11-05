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
    return String(value === null || value === void 0 ? "" : value).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/\x22/g, "&quot;").replace(/\x27/g, "&#39;");
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
    const text3 = (key) => copy[document.documentElement.lang === "ja" ? "ja" : "en"][key];
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
    const dialog2 = document.createElement("dialog");
    dialog2.className = "setup-dialog";
    dialog2.id = "setup-guide";
    dialog2.setAttribute("aria-labelledby", "setup-title");
    document.body.append(dialog2);
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
      dialog2.close();
      dialog2.replaceChildren();
      devices = [];
      if (previousFocus?.hidden) {
        const heading = document.querySelector("main h1");
        heading?.setAttribute("tabindex", "-1");
        heading?.focus();
      } else previousFocus?.focus();
      window.refreshDashboard?.();
    };
    const status = (message, error = false) => {
      const node = dialog2.querySelector("#setup-status");
      if (node) {
        node.textContent = message;
        node.setAttribute("role", error ? "alert" : "status");
      }
    };
    const setBusy = (value) => {
      busy = value;
      dialog2.querySelectorAll("button:not([data-action=skip])").forEach((button) => {
        button.disabled = value;
      });
      dialog2.querySelectorAll("input").forEach((input) => {
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
        dialog2.innerHTML = `<h2 id="setup-title" tabindex="-1">Choose Language</h2><form id="setup-language-form"><label for="setup-language">Language</label><select id="setup-language" required><option value="" disabled selected>Select a language</option><option value="ja">Japanese</option><option value="en">English</option></select><div class="setup-actions"><button type="submit" class="setup-primary">Continue</button></div></form>`;
        dialog2.querySelector("#setup-language")?.focus();
        dialog2.querySelector("form")?.addEventListener("submit", (event) => {
          event.preventDefault();
          const language = dialog2.querySelector("#setup-language").value;
          if (language !== "ja" && language !== "en") return;
          choosingLanguage = false;
          window.setLanguage?.(language);
          window.refreshDashboard?.();
          render();
        });
        return;
      }
      const titles = ["welcome", "discovery", "alerts"];
      dialog2.innerHTML = `<div class="setup-step">${text3("step")} ${step + 1} / 3</div><h2 id="setup-title" tabindex="-1">${text3(titles[step])}</h2>` + (step === 0 ? `<p>${text3("intro")}</p>` : step === 1 ? `<form id="setup-form"><label>${text3("cidr")}<input id="setup-cidr" required placeholder="192.168.1.0/24" autocomplete="off"></label><label>${text3("community")}<input id="setup-community" type="password" required value="public" autocomplete="off"></label><p>${text3("privacy")}</p><button class="setup-primary" type="submit">${text3("scan")}</button></form><progress id="setup-progress" hidden aria-label="${text3("progress")}"></progress><div id="setup-results" class="setup-results"></div><button type="button" data-action="register" hidden>${text3("register")}</button><p id="setup-status" role="status" aria-live="polite"></p>` : `<p>${text3("alertInfo")}</p><a href="/settings" id="setup-settings">${text3("settings")}</a>`) + `<div class="setup-actions"><button type="button" data-action="skip">${text3("skip")}</button>${step > 0 || step === 0 && languageChoiceEnabled ? `<button type="button" data-action="${step === 0 ? "language-back" : "back"}">${text3("back")}</button>` : ""}<button type="button" class="setup-primary" data-action="next">${text3(step === 2 ? "finish" : step === 1 && !registered ? "continue" : "next")}</button></div>`;
      dialog2.querySelector("#setup-title")?.focus();
      const communityInput = dialog2.querySelector("#setup-community");
      if (communityInput) addSecretVisibility(communityInput);
      dialog2.querySelector("#setup-settings")?.addEventListener("click", close);
      dialog2.querySelector("form")?.addEventListener("submit", (event) => {
        event.preventDefault();
        void scan();
      });
    };
    const scan = async () => {
      if (busy) return;
      const cidr = dialog2.querySelector("#setup-cidr").value.trim();
      const community = dialog2.querySelector("#setup-community").value;
      const match = cidr.match(/^(\d{1,3}(?:\.\d{1,3}){3})\/(\d{1,2})$/);
      if (!match || match[1].split(".").some((part) => Number(part) > 255) || Number(match[2]) < 24 || Number(match[2]) > 32) {
        status(text3("rangeError"), true);
        return;
      }
      stop();
      devices = [];
      registered = false;
      dialog2.querySelector("#setup-results").replaceChildren();
      dialog2.querySelector("[data-action=register]").hidden = true;
      const signal = controller.signal;
      setBusy(true);
      status(text3("scanning"));
      const progress = dialog2.querySelector("#setup-progress");
      progress.hidden = false;
      progress.removeAttribute("value");
      const fail = (error) => {
        if (signal.aborted) return;
        stop();
        setBusy(false);
        progress.hidden = true;
        status(error instanceof Error ? error.message : text3("failure"), true);
      };
      deadline = window.setTimeout(() => fail(new Error(text3("timeout"))), 3e5);
      try {
        const prefix = Number(match[2]);
        const hosts = prefix >= 31 ? 2 ** (32 - prefix) : 2 ** (32 - prefix) - 2;
        const job = await request("/api/discovery/scan", signal, new URLSearchParams({ cidr, community, max_hosts: String(hosts) }));
        if (signal.aborted) return;
        if (!job.job_id) throw new Error(text3("failure"));
        const poll = async () => {
          try {
            const result = await request(`/api/discovery/scan/${encodeURIComponent(job.job_id)}`, signal);
            if (signal.aborted) return;
            progress.max = result.total || hosts;
            progress.value = result.scanned || 0;
            status(`${text3("scanning")} ${result.scanned || 0} / ${result.total || hosts}`);
            if (result.status === "error") throw new Error(text3("failure"));
            if (result.status !== "done") {
              timer = window.setTimeout(() => void poll(), 1500);
              return;
            }
            const existing = await request("/api/devices", signal);
            if (signal.aborted) return;
            window.clearTimeout(deadline);
            const known = new Set(existing.map((device) => device.ip));
            devices = (result.devices || []).filter((device) => !known.has(device.ip)).map((device) => ({ ...device, community }));
            setBusy(false);
            progress.hidden = true;
            status(devices.length ? text3("selected") : text3("empty"));
            dialog2.querySelector("#setup-results").innerHTML = devices.map((device, index) => `<label class="setup-device"><input type="checkbox" value="${index}" checked><span>${esc(device.ip)} ${esc(device.name || "")}</span></label>`).join("");
            dialog2.querySelector("[data-action=register]").hidden = devices.length === 0;
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
      const selected = Array.from(dialog2.querySelectorAll("#setup-results input:checked")).map((input) => devices[Number(input.value)]);
      if (!selected.length) {
        status(text3("selected"), true);
        return;
      }
      const signal = controller.signal;
      setBusy(true);
      try {
        const result = await request("/api/discovery/register", signal, selected);
        if (signal.aborted) return;
        const accepted = /* @__PURE__ */ new Set([...result.registered, ...result.skipped]);
        dialog2.querySelectorAll("#setup-results input").forEach((input) => {
          if (accepted.has(devices[Number(input.value)].ip)) input.closest("label")?.remove();
        });
        registered = result.registered.length > 0 || result.skipped.length > 0;
        status(result.errors.length ? result.errors.join("\n") : registered ? text3("registered") : text3("failure"), result.errors.length > 0 || !registered);
        window.refreshDashboard?.();
        dialog2.querySelector("[data-action=next]").textContent = text3(registered ? "next" : "continue");
        dialog2.querySelector("[data-action=register]").hidden = dialog2.querySelectorAll("#setup-results input").length === 0;
      } catch (error) {
        if (!signal.aborted) status(error instanceof Error ? error.message : text3("failure"), true);
      } finally {
        if (!signal.aborted) setBusy(false);
      }
    };
    dialog2.addEventListener("click", (event) => {
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
    dialog2.addEventListener("cancel", (event) => {
      event.preventDefault();
      close();
    });
    const open = (selectLanguage = false) => {
      if (dialog2.open) return;
      previousFocus = document.activeElement;
      stop();
      step = 0;
      registered = false;
      languageChoiceEnabled = selectLanguage;
      choosingLanguage = selectLanguage;
      dialog2.showModal();
      render();
    };
    const launch = document.createElement("button");
    launch.className = "setup-launch";
    launch.type = "button";
    launch.textContent = text3("guide");
    launch.hidden = auto && dismissed;
    launch.addEventListener("click", () => open());
    document.querySelector("main h1")?.after(launch);
    window.addEventListener("tracepulse-language-change", () => {
      launch.textContent = text3("guide");
      if (dialog2.open && step !== 1) render();
    });
    if (auto) {
      if (!dismissed) void request("/api/devices", controller.signal).then((existing) => {
        if (existing.length === 0) open(true);
      }).catch(() => {
      });
    }
  }

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
        .metadata-dialog button,.device-toolbar button,.device-action-button,#edit-device-metadata{padding:8px 12px;cursor:pointer;background:var(--tp-bg,#0f172a);color:inherit;border:1px solid #64748b;border-radius:4px}
        .metadata-dialog button:disabled,.device-toolbar button:disabled{opacity:.5;cursor:default}
        .metadata-actions,.device-toolbar,.device-tags{display:flex;gap:8px;flex-wrap:wrap;align-items:center}
        .metadata-actions{justify-content:flex-end;margin-top:12px}.device-toolbar{margin:12px 0}.device-toolbar input{flex:1 1 220px}
        .bulk-action-progress{margin-top:12px;padding:12px;border:1px solid var(--tp-border,#475569);border-radius:4px;background:var(--tp-raised,#1a2b40)}
        .bulk-action-progress[hidden]{display:none}.bulk-action-progress progress{display:block;width:100%;height:8px;accent-color:var(--tp-accent,#38c5b2)}
        .bulk-action-progress p{margin:8px 0 0}.bulk-action-progress small{color:var(--tp-muted,#94a3b8)}
        .device-tag{display:inline-flex;align-items:center;gap:4px;padding:2px 7px;background:var(--tp-bg,#0f172a);border:1px solid #64748b;border-radius:12px;font-size:.8rem;overflow-wrap:anywhere}
        .device-tags{margin-top:4px}.device-tag button{padding:0 4px}.device-original{font-size:.8rem;color:var(--tp-muted,#94a3b8);overflow-wrap:anywhere}
        .metadata-dialog [role=alert],.device-toolbar [role=alert]{color:#f87171}
        :root[data-theme=light] .metadata-dialog{background:#fff;color:#1e293b}
        :root[data-theme=light] .metadata-dialog input,:root[data-theme=light] .metadata-dialog select,:root[data-theme=light] .metadata-dialog button,:root[data-theme=light] .device-toolbar input,:root[data-theme=light] .device-toolbar button,:root[data-theme=light] .device-action-button,:root[data-theme=light] .device-tag,:root[data-theme=light] #edit-device-metadata{background:#f1f5f9;color:#1e293b}
    `;
    document.head.append(style);
  }
  function deviceLabel(device) {
    if (device.display_name?.trim()) return device.display_name;
    if (device.name?.trim()) return device.name;
    return device.ip || "";
  }
  var copy2 = {
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
      clear: "Leave the display name empty to use sysName or IP.",
      bulk: "Bulk actions",
      actions: "Actions",
      bulkTags: "Add tags",
      bulkNames: "Set / change device names",
      unregister: "Unregister",
      targets: "Selected devices",
      appendHint: "Existing tags are retained. Duplicate tags are ignored.",
      unregisterHint: "Unregister these devices and delete their monitoring history? This cannot be undone.",
      unregistering: "Unregistering {count} devices... ({seconds}s elapsed)",
      tagging: "Adding tags to {count} devices... ({seconds}s elapsed)",
      unregisterWait: "Deleting monitoring history may take some time. Please keep this page open.",
      confirm: "Confirm"
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
      clear: "\u8868\u793A\u540D\u3092\u7A7A\u306B\u3059\u308B\u3068sysName\u307E\u305F\u306FIP\u3092\u8868\u793A\u3057\u307E\u3059\u3002",
      bulk: "\u4E00\u62EC\u64CD\u4F5C",
      actions: "\u64CD\u4F5C",
      bulkTags: "\u30BF\u30B0\u4ED8\u3051",
      bulkNames: "\u6A5F\u5668\u540D\u306E\u8A2D\u5B9A\u30FB\u5909\u66F4",
      unregister: "\u767B\u9332\u89E3\u9664",
      targets: "\u9078\u629E\u3057\u305F\u6A5F\u5668",
      appendHint: "\u65E2\u5B58\u30BF\u30B0\u3092\u4FDD\u6301\u3057\u3066\u8FFD\u52A0\u3057\u307E\u3059\u3002\u91CD\u8907\u3059\u308B\u30BF\u30B0\u306F\u8FFD\u52A0\u3057\u307E\u305B\u3093\u3002",
      unregisterHint: "\u4EE5\u4E0B\u306E\u6A5F\u5668\u306E\u767B\u9332\u3068\u76E3\u8996\u5C65\u6B74\u3092\u524A\u9664\u3057\u307E\u3059\u3002\u5143\u306B\u623B\u305B\u307E\u305B\u3093\u3002\u5B9F\u884C\u3057\u307E\u3059\u304B\uFF1F",
      unregistering: "{count}\u53F0\u306E\u767B\u9332\u3092\u89E3\u9664\u3057\u3066\u3044\u307E\u3059...\uFF08\u7D4C\u904E {seconds}\u79D2\uFF09",
      tagging: "{count}\u53F0\u306B\u30BF\u30B0\u3092\u8FFD\u52A0\u3057\u3066\u3044\u307E\u3059...\uFF08\u7D4C\u904E {seconds}\u79D2\uFF09",
      unregisterWait: "\u76E3\u8996\u5C65\u6B74\u306E\u524A\u9664\u306B\u306F\u6642\u9593\u304C\u304B\u304B\u308B\u5834\u5408\u304C\u3042\u308A\u307E\u3059\u3002\u3053\u306E\u30DA\u30FC\u30B8\u3092\u958B\u3044\u305F\u307E\u307E\u304A\u5F85\u3061\u304F\u3060\u3055\u3044\u3002",
      confirm: "\u78BA\u5B9A"
    }
  };
  function metadataText(key) {
    return copy2[document.documentElement.lang === "ja" ? "ja" : "en"][key];
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
        <button type="button" id="device-snmp-edit">${esc(credentialTitle())}</button>
        <p role="alert"></p><div class="metadata-actions">
        <button type="button" data-cancel>${esc(metadataText("cancel"))}</button>
        <button type="submit">${esc(metadataText("save"))}</button></div></form>`);
    const input = element.querySelector("#device-tag-input");
    element.querySelector("#device-snmp-edit").onclick = () => editSnmpCredentials([device], saved);
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
  function editSnmpCredentials(devices, saved) {
    const element = dialog("device-snmp-dialog", credentialTitle());
    element.insertAdjacentHTML("beforeend", `<form><p>${esc(metadataText("targets"))}: ${devices.length}</p>
        <ul>${devices.map((device) => `<li>${esc(device.ip || "")} ${esc(deviceLabel(device))}</li>`).join("")}</ul>
        ${credentialFields("device-snmp")}
        <p role="status" aria-live="polite"></p>
        <p role="alert"></p><div class="metadata-actions"><button type="button" data-reset>${document.documentElement.lang === "ja" ? "\u5171\u901A\u8A2D\u5B9A\u3078\u623B\u3059" : "Use global defaults"}</button>
        <button type="button" data-cancel>${esc(metadataText("cancel"))}</button><button type="submit">${esc(metadataText("save"))}</button></div></form>`);
    element.querySelector("[data-cancel]").onclick = () => {
      clearCredentials(element);
      element.close();
    };
    let busy = false;
    let loading = false;
    const individual = devices.length === 1 && Boolean(devices[0].ip);
    const japanese2 = document.documentElement.lang === "ja";
    const status = element.querySelector("[role=status]");
    const failure = element.querySelector("[role=alert]");
    const controls = element.querySelectorAll("input,select,button");
    const setBusy = (value) => {
      busy = value;
      controls.forEach((control) => {
        control.disabled = value;
      });
    };
    const controller = new AbortController();
    let stopSaveProgress;
    element.addEventListener("cancel", (event) => {
      if (busy && !loading) event.preventDefault();
    });
    element.addEventListener("close", () => {
      stopSaveProgress?.();
      controller.abort();
      clearCredentials(element);
    });
    const persist = async (defaults) => {
      if (busy) return;
      const credentials = defaults ? {} : readCredentials(element);
      setBusy(true);
      failure.textContent = "";
      const progressContainer = document.createElement("div");
      failure.before(progressContainer);
      const stopProgress = showOperationProgress(progressContainer, credentialTitle(), (seconds) => {
        const japanese3 = document.documentElement.lang === "ja";
        return japanese3 ? `${devices.length}\u53F0\u306E${defaults ? "\u8A8D\u8A3C\u60C5\u5831\u3092\u5171\u901A\u8A2D\u5B9A\u3078\u623B\u3057\u3066\u3044\u307E\u3059" : "SNMP\u8A8D\u8A3C\u60C5\u5831\u3092\u4FDD\u5B58\u3057\u3066\u3044\u307E\u3059"}...\uFF08\u7D4C\u904E ${seconds}\u79D2\uFF09` : `${defaults ? "Restoring global defaults for" : "Saving SNMP credentials for"} ${devices.length} devices... (${seconds}s elapsed)`;
      });
      stopSaveProgress = () => {
        stopProgress();
        progressContainer.remove();
      };
      progressContainer.scrollIntoView({ block: "nearest" });
      try {
        await post("/api/devices/snmp-credentials", { device_ids: devices.map((device) => device.id), credentials, preserve_secrets: !defaults && individual });
        clearCredentials(element);
        element.close();
        saved();
      } catch (reason) {
        failure.textContent = String(reason);
      } finally {
        stopSaveProgress?.();
        stopSaveProgress = void 0;
        setBusy(false);
      }
    };
    element.querySelector("[data-reset]").onclick = () => {
      if (window.confirm(japanese2 ? "\u500B\u5225\u306ESNMP\u8A8D\u8A3C\u60C5\u5831\u3092\u524A\u9664\u3057\u3001\u5171\u901A\u8A2D\u5B9A\u3078\u623B\u3057\u307E\u3059\u304B\uFF1F" : "Remove the device SNMP overrides and use global defaults?")) void persist(true);
    };
    element.querySelector("form").onsubmit = (event) => {
      event.preventDefault();
      void persist(false);
    };
    element.showModal();
    if (individual) {
      loading = true;
      setBusy(true);
      element.querySelector("[data-cancel]").disabled = false;
      status.textContent = japanese2 ? "\u8A2D\u5B9A\u3092\u8AAD\u307F\u8FBC\u3093\u3067\u3044\u307E\u3059\u2026" : "Loading settings...";
      const hint = element.querySelector("#device-snmp-hint");
      hint.removeAttribute("data-i18n");
      hint.textContent = japanese2 ? "\u4FDD\u5B58\u6E08\u307F\u306E\u30B3\u30DF\u30E5\u30CB\u30C6\u30A3\u3068\u30D1\u30B9\u30EF\u30FC\u30C9\u306F\u8868\u793A\u3057\u307E\u305B\u3093\u3002\u7A7A\u6B04\u306E\u307E\u307E\u4FDD\u5B58\u3059\u308B\u3068\u65E2\u5B58\u5024\u3092\u7DAD\u6301\u3057\u307E\u3059\u3002" : "Stored communities and passwords are not displayed. Leave them blank to keep existing values.";
      void (async () => {
        try {
          const response = await fetch(`/api/device/${encodeURIComponent(devices[0].ip)}/snmp-credentials`, { signal: controller.signal });
          const data = await response.json();
          if (!response.ok || !data.ok || !data.devices) throw new Error(data.error || metadataText("failed"));
          const view = data.devices;
          for (const key of ["version", "v3_username", "v3_auth_protocol", "v3_priv_protocol"]) {
            const input = element.querySelector(`[data-snmp=${key}]`);
            input.value = view.credentials[key] || "";
            if (key === "v3_username" && input instanceof HTMLInputElement) {
              input.type = "text";
              input.autocomplete = "off";
            }
          }
          for (const key of ["community", "v3_auth_password", "v3_priv_password"]) {
            const input = element.querySelector(`[data-snmp=${key}]`);
            input.removeAttribute("data-i18n-placeholder");
            input.placeholder = view.secret_configured[key] ? japanese2 ? "\u8A2D\u5B9A\u6E08\u307F\uFF08\u5909\u66F4\u3059\u308B\u5834\u5408\u306E\u307F\u5165\u529B\uFF09" : "Configured (enter only to replace)" : japanese2 ? "\u7A7A\u6B04\u306E\u5834\u5408\u306F\u5171\u901A\u8A2D\u5B9A\u3092\u4F7F\u7528" : "Blank: use global defaults";
          }
          status.textContent = view.individual_configured ? japanese2 ? "\u73FE\u5728\u306E\u8A2D\u5B9A\uFF1A\u30C7\u30D0\u30A4\u30B9\u500B\u5225\u8A2D\u5B9A" : "Current settings: device overrides" : japanese2 ? "\u73FE\u5728\u306E\u8A2D\u5B9A\uFF1A\u5171\u901A\u8A2D\u5B9A\u3092\u4F7F\u7528" : "Current settings: global defaults";
          loading = false;
          setBusy(false);
        } catch (reason) {
          if (controller.signal.aborted) return;
          status.textContent = "";
          failure.textContent = String(reason);
          element.querySelector("[data-cancel]").disabled = false;
        }
      })();
    }
  }
  function bulkActions(devices, saved, individual = false) {
    const ids = devices.map((device) => device.id).filter((id) => typeof id === "number");
    const chooser = dialog("bulk-actions-dialog", metadataText(individual ? "actions" : "bulk"));
    const list = () => `<p>${esc(metadataText("targets"))}: ${devices.length}</p><ul>${devices.map((device) => `<li>${esc(device.ip || "")} \u2014 ${esc(deviceLabel(device))}</li>`).join("")}</ul>`;
    chooser.insertAdjacentHTML("beforeend", `${list()}<div class="metadata-actions">
        <button type="button" data-action="tags">${esc(metadataText("bulkTags"))}</button>
        <button type="button" data-action="names">${esc(metadataText("bulkNames"))}</button>
        <button type="button" data-action="snmp">${esc(credentialTitle())}</button>
        <button type="button" data-action="unregister">${esc(metadataText("unregister"))}</button>
        <button type="button" data-cancel>${esc(metadataText("cancel"))}</button></div>`);
    chooser.querySelector("[data-cancel]").onclick = () => chooser.close();
    chooser.querySelector("[data-action=snmp]").onclick = () => {
      chooser.close();
      editSnmpCredentials(devices, saved);
    };
    chooser.querySelector("[data-action=names]").onclick = () => {
      chooser.close();
      if (individual) editDeviceMetadata(devices[0], saved);
      else bulkRename(ids, saved);
    };
    const actionDialog = (action) => {
      chooser.close();
      const tags = action === "add_tags";
      const element = dialog("bulk-action-confirm", metadataText(tags ? "bulkTags" : "unregister"));
      element.insertAdjacentHTML("beforeend", `<form>${list()}
            <p>${esc(metadataText(tags ? "appendHint" : "unregisterHint"))}</p>
            ${tags ? `<div id="bulk-tag-chips" class="device-tags"></div>${field("bulk-tag-input", metadataText("tags"), "", "text", "maxlength=64")}
                <p>${esc(metadataText("tagHint"))}</p><button type="button" id="bulk-tag-add">${esc(metadataText("add"))}</button>` : ""}
            <div class="bulk-action-progress" hidden><progress aria-label="${esc(metadataText(tags ? "bulkTags" : "unregister"))}"></progress><p role="status" aria-live="polite"></p>${tags ? "" : `<small>${esc(metadataText("unregisterWait"))}</small>`}</div>
            <p role="alert"></p><div class="metadata-actions"><button type="button" data-cancel>${esc(metadataText("cancel"))}</button>
            <button type="submit">${esc(metadataText("confirm"))}</button></div></form>`);
      let busy = false;
      const additions = [];
      const error = element.querySelector("[role=alert]");
      const input = element.querySelector("#bulk-tag-input");
      const renderTags = () => {
        const chips = element.querySelector("#bulk-tag-chips");
        chips.replaceChildren();
        additions.forEach((tag, index) => {
          const chip = document.createElement("span");
          chip.className = "device-tag";
          chip.append(document.createTextNode(tag));
          const remove = document.createElement("button");
          remove.type = "button";
          remove.textContent = "\xD7";
          remove.setAttribute("aria-label", `${metadataText("remove")} ${tag}`);
          remove.onclick = () => {
            additions.splice(index, 1);
            renderTags();
          };
          chip.append(remove);
          chips.append(chip);
        });
      };
      const addTag = () => {
        const tag = input.value.trim();
        if (tag && !additions.includes(tag)) additions.push(tag);
        input.value = "";
        renderTags();
        input.focus();
      };
      if (input) {
        element.querySelector("#bulk-tag-add").onclick = addTag;
        input.onkeydown = (event) => {
          if (event.key === "Enter" && !event.isComposing) {
            event.preventDefault();
            addTag();
          }
        };
      }
      element.querySelector("[data-cancel]").onclick = () => element.close();
      element.addEventListener("cancel", (event) => {
        if (busy) event.preventDefault();
      });
      element.querySelector("form").onsubmit = async (event) => {
        event.preventDefault();
        if (busy) return;
        if (input?.value.trim()) addTag();
        if (tags && additions.length === 0) {
          input?.reportValidity();
          input?.focus();
          error.textContent = metadataText("tagHint");
          return;
        }
        busy = true;
        error.textContent = "";
        const controls = element.querySelectorAll("input,button");
        controls.forEach((control) => {
          control.disabled = true;
        });
        const progress = element.querySelector(".bulk-action-progress");
        let progressTimer;
        if (progress) {
          const started = performance.now();
          const updateProgress = () => {
            progress.querySelector("[role=status]").textContent = metadataText(tags ? "tagging" : "unregistering").replace("{count}", String(ids.length)).replace("{seconds}", String(Math.floor((performance.now() - started) / 1e3)));
          };
          progress.hidden = false;
          updateProgress();
          progress.scrollIntoView({ block: "nearest" });
          progressTimer = window.setInterval(updateProgress, 1e3);
        }
        element.setAttribute("aria-busy", "true");
        try {
          await post("/api/devices/bulk-action", { action, device_ids: ids, ...tags ? { tags: additions } : {} });
          element.close();
          saved();
        } catch (failure) {
          error.textContent = String(failure);
        } finally {
          window.clearInterval(progressTimer);
          if (progress) progress.hidden = true;
          element.setAttribute("aria-busy", "false");
          busy = false;
          controls.forEach((control) => {
            control.disabled = false;
          });
        }
      };
      element.showModal();
    };
    chooser.querySelector("[data-action=tags]").onclick = () => actionDialog("add_tags");
    chooser.querySelector("[data-action=unregister]").onclick = () => actionDialog("unregister");
    chooser.showModal();
  }
  function bulkRename(ids, saved) {
    const element = dialog("bulk-rename-dialog", metadataText("rename"));
    element.insertAdjacentHTML("beforeend", `<form>
        ${field("rename-prefix", metadataText("prefix"), "", "text", "maxlength=512")}
        ${field("rename-start", metadataText("start"), "1", "number", "min=0 max=4294967295 required")}
        ${field("rename-padding", metadataText("padding"), "2", "number", "min=0 max=10 required")}
        ${field("rename-suffix", metadataText("suffix"), "", "text", "maxlength=512")}
        <label for="rename-sort">${esc(metadataText("order"))}<select id="rename-sort">
        <option value="ip_asc">${esc(metadataText("ip"))}</option>
        <option value="selection_order">${esc(metadataText("selection"))}</option></select></label>
        <p role="alert"></p><ol id="rename-preview" aria-live="polite"></ol>
        <div class="metadata-actions"><button type="button" data-cancel>${esc(metadataText("cancel"))}</button>
        <button type="button" id="rename-preview-button">${esc(metadataText("preview"))}</button>
        <button type="submit" disabled>${esc(metadataText("apply"))}</button></div></form>`);
    const form = element.querySelector("form");
    const submit = element.querySelector("[type=submit]");
    const previewButton = element.querySelector("#rename-preview-button");
    const error = element.querySelector("[role=alert]");
    const preview = element.querySelector("#rename-preview");
    let approved = null;
    let revision = 0;
    let busy = false;
    let applying = false;
    let stopRenameProgress = () => {
    };
    const showRenameProgress = (previewing) => {
      const container = document.createElement("div");
      error.before(container);
      const stop = showOperationProgress(container, metadataText("rename"), (seconds) => {
        const japanese2 = document.documentElement.lang === "ja";
        return japanese2 ? `${ids.length}\u53F0\u306E${previewing ? "\u540D\u79F0\u3092\u30D7\u30EC\u30D3\u30E5\u30FC\u3057\u3066\u3044\u307E\u3059" : "\u6A5F\u5668\u540D\u3092\u5909\u66F4\u3057\u3066\u3044\u307E\u3059"}...\uFF08\u7D4C\u904E ${seconds}\u79D2\uFF09` : `${previewing ? "Previewing names for" : "Renaming"} ${ids.length} devices... (${seconds}s elapsed)`;
      });
      stopRenameProgress = () => {
        stop();
        container.remove();
      };
      container.scrollIntoView({ block: "nearest" });
    };
    const invalidate = () => {
      revision++;
      approved = null;
      submit.disabled = true;
      preview.replaceChildren();
    };
    form.addEventListener("input", invalidate);
    form.addEventListener("change", invalidate);
    const cancel = element.querySelector("[data-cancel]");
    cancel.onclick = () => {
      if (!applying) {
        revision++;
        element.close();
      }
    };
    element.addEventListener("cancel", (event) => {
      if (applying) event.preventDefault();
      else revision++;
    });
    element.addEventListener("close", () => stopRenameProgress());
    previewButton.onclick = async () => {
      if (busy || !form.reportValidity()) return;
      invalidate();
      const version = revision;
      const payload = {
        device_ids: ids,
        prefix: element.querySelector("#rename-prefix").value,
        start_number: Number(element.querySelector("#rename-start").value),
        padding: Number(element.querySelector("#rename-padding").value),
        suffix: element.querySelector("#rename-suffix").value,
        sort_by: element.querySelector("#rename-sort").value
      };
      busy = true;
      previewButton.disabled = true;
      error.textContent = "";
      showRenameProgress(true);
      try {
        const data = await post("/api/devices/bulk-rename/preview", payload);
        if (version !== revision || !element.open) return;
        data.devices.forEach((device) => {
          const row = document.createElement("li");
          row.textContent = `${device.ip} \u2192 ${device.display_name}`;
          preview.append(row);
        });
        approved = payload;
        submit.disabled = false;
      } catch (failure) {
        error.textContent = String(failure);
      } finally {
        stopRenameProgress();
        busy = false;
        previewButton.disabled = false;
      }
    };
    form.onsubmit = async (event) => {
      event.preventDefault();
      if (!approved || busy) return;
      busy = true;
      submit.disabled = true;
      previewButton.disabled = true;
      error.textContent = "";
      applying = true;
      cancel.disabled = true;
      showRenameProgress(false);
      const inputs = form.querySelectorAll("input,select");
      inputs.forEach((input) => {
        input.disabled = true;
      });
      try {
        await post("/api/devices/bulk-rename", approved);
        element.close();
        saved();
      } catch (failure) {
        error.textContent = String(failure);
        invalidate();
      } finally {
        stopRenameProgress();
        applying = false;
        cancel.disabled = false;
        busy = false;
        previewButton.disabled = false;
        inputs.forEach((input) => {
          input.disabled = false;
        });
      }
    };
    element.showModal();
  }

  // frontend/src/shared/device-filters.ts
  var copy3 = {
    en: {
      title: "Column filters (AND)",
      add: "Add condition",
      clear: "Clear conditions",
      field: "Column",
      operator: "Condition",
      value: "Value",
      remove: "Remove condition",
      ip: "IP address",
      display_name: "Display name",
      name: "Hostname (sysName)",
      tags: "Tags",
      status: "Status",
      health_score: "Health score",
      contains: "Contains",
      equals: "Equals",
      not_equals: "Does not equal",
      starts_with: "Starts with",
      gte: "At least (\u2265)",
      lte: "At most (\u2264)",
      gt: "Greater than (>)",
      lt: "Less than (<)",
      hint: "All conditions and the search must match. Empty values are ignored. Text matching is case-insensitive; tags match individually.",
      invalid: "Health score must be a number from 0 to 100."
    },
    ja: {
      title: "\u30AB\u30E9\u30E0\u5225\u30D5\u30A3\u30EB\u30BF\u30FC\uFF08AND\uFF09",
      add: "\u6761\u4EF6\u3092\u8FFD\u52A0",
      clear: "\u6761\u4EF6\u3092\u30AF\u30EA\u30A2",
      field: "\u5BFE\u8C61\u30AB\u30E9\u30E0",
      operator: "\u6761\u4EF6",
      value: "\u5024",
      remove: "\u6761\u4EF6\u3092\u524A\u9664",
      ip: "IP\u30A2\u30C9\u30EC\u30B9",
      display_name: "\u8868\u793A\u540D",
      name: "\u30DB\u30B9\u30C8\u540D\uFF08sysName\uFF09",
      tags: "\u30BF\u30B0",
      status: "\u72B6\u614B",
      health_score: "\u30D8\u30EB\u30B9\u30B9\u30B3\u30A2",
      contains: "\u542B\u3080",
      equals: "\u4E00\u81F4\u3059\u308B",
      not_equals: "\u4E00\u81F4\u3057\u306A\u3044",
      starts_with: "\u3067\u59CB\u307E\u308B",
      gte: "\u4EE5\u4E0A\uFF08\u2265\uFF09",
      lte: "\u4EE5\u4E0B\uFF08\u2264\uFF09",
      gt: "\u3088\u308A\u5927\u304D\u3044\uFF08>\uFF09",
      lt: "\u3088\u308A\u5C0F\u3055\u3044\uFF08<\uFF09",
      hint: "\u3059\u3079\u3066\u306E\u6761\u4EF6\u3068\u691C\u7D22\u306B\u4E00\u81F4\u3059\u308B\u6A5F\u5668\u3092\u8868\u793A\u3057\u307E\u3059\u3002\u7A7A\u6B04\u306F\u7121\u8996\u3057\u307E\u3059\u3002\u82F1\u5B57\u306E\u5927\u6587\u5B57\u30FB\u5C0F\u6587\u5B57\u306F\u533A\u5225\u305B\u305A\u3001\u30BF\u30B0\u306F\u500B\u5225\u306B\u7167\u5408\u3057\u307E\u3059\u3002",
      invalid: "\u30D8\u30EB\u30B9\u30B9\u30B3\u30A2\u306F0\u304B\u3089100\u306E\u6570\u5024\u3092\u5165\u529B\u3057\u3066\u304F\u3060\u3055\u3044\u3002"
    }
  };
  var fields = ["ip", "display_name", "name", "tags", "status", "health_score"];
  var statuses = ["online", "warning", "critical", "offline", "unknown"];
  var text2 = (key) => copy3[document.documentElement.lang === "ja" ? "ja" : "en"][key];
  var operators = (field2) => field2 === "health_score" ? ["gte", "lte", "equals", "not_equals", "gt", "lt"] : field2 === "status" ? ["equals", "not_equals"] : ["contains", "equals", "not_equals", "starts_with"];
  function initializeDeviceFilters(before, changed) {
    const conditions = [];
    const section = document.createElement("section");
    section.id = "device-filters";
    section.setAttribute("aria-labelledby", "device-filters-title");
    const style = document.createElement("style");
    style.textContent = `
        #device-filters{margin:12px 0}
        .device-filter-row{display:flex;flex-wrap:wrap;gap:8px;align-items:end;margin:8px 0}
        .device-filter-row label{display:grid;gap:4px;flex:1 1 150px;min-width:0}
        .device-filter-row input,.device-filter-row select,#device-filters button{box-sizing:border-box;max-width:100%;padding:8px;border:1px solid #64748b;border-radius:4px;background:var(--tp-bg,#0f172a);color:inherit}
        .device-filter-row input,.device-filter-row select{width:100%;min-width:0}
        #device-filters button{cursor:pointer}#device-filters p{font-size:.85rem}
        #device-filters [role=alert]{color:#f87171}
        :root[data-theme=light] #device-filters input,:root[data-theme=light] #device-filters select,:root[data-theme=light] #device-filters button{background:#f1f5f9;color:#1e293b}
    `;
    document.head.append(style);
    before.before(section);
    const invalid = (condition) => condition.field === "health_score" && condition.value.trim() !== "" && (!Number.isFinite(Number(condition.value)) || Number(condition.value) < 0 || Number(condition.value) > 100);
    const update = () => {
      section.querySelector("[role=alert]").textContent = conditions.some(invalid) ? text2("invalid") : "";
      changed();
    };
    const render = () => {
      section.innerHTML = `<h3 id="device-filters-title">${esc(text2("title"))}</h3><p>${esc(text2("hint"))}</p><div id="device-filter-rows"></div><button type="button" id="device-filter-add">${esc(text2("add"))}</button> <button type="button" id="device-filter-clear">${esc(text2("clear"))}</button><p role="alert"></p>`;
      const rows = section.querySelector("#device-filter-rows");
      conditions.forEach((condition, index) => {
        const row = document.createElement("div");
        row.className = "device-filter-row";
        const options = (values, selected, labels) => values.map((value) => `<option value="${esc(value)}" ${value === selected ? "selected" : ""}>${esc(labels ? text2(value) : value)}</option>`).join("");
        row.innerHTML = `<label>${esc(text2("field"))}<select data-filter-field>${options(fields, condition.field, true)}</select></label>
                <label>${esc(text2("operator"))}<select data-filter-operator>${options(operators(condition.field), condition.operator, true)}</select></label>
                <label>${esc(text2("value"))}${condition.field === "status" ? `<select data-filter-value><option value="">-</option>${options(statuses, condition.value, false)}</select>` : `<input data-filter-value type="text" ${condition.field === "health_score" ? 'inputmode="decimal"' : ""} value="${esc(condition.value)}">`}</label>
                <button type="button" data-filter-remove>${esc(text2("remove"))}</button>`;
        row.querySelector("[data-filter-field]").onchange = (event) => {
          condition.field = event.currentTarget.value;
          condition.operator = operators(condition.field)[0];
          condition.value = "";
          render();
          update();
          section.querySelectorAll("[data-filter-field]")[index]?.focus();
        };
        row.querySelector("[data-filter-operator]").onchange = (event) => {
          condition.operator = event.currentTarget.value;
          update();
        };
        row.querySelector("[data-filter-value]").oninput = (event) => {
          condition.value = event.currentTarget.value;
          update();
        };
        row.querySelector("[data-filter-remove]").onclick = () => {
          conditions.splice(index, 1);
          render();
          update();
          section.querySelector("#device-filter-add").focus();
        };
        rows.append(row);
      });
      section.querySelector("#device-filter-add").onclick = () => {
        conditions.push({ field: "ip", operator: "contains", value: "" });
        render();
        update();
        section.querySelectorAll("[data-filter-field]")[conditions.length - 1]?.focus();
      };
      section.querySelector("#device-filter-clear").onclick = () => {
        conditions.length = 0;
        render();
        update();
        section.querySelector("#device-filter-add").focus();
      };
      section.querySelector("[role=alert]").textContent = conditions.some(invalid) ? text2("invalid") : "";
    };
    render();
    window.addEventListener("tracepulse-language-change", render);
    return (device) => conditions.every((condition) => {
      const value = condition.value.trim().toLocaleLowerCase();
      if (!value) return true;
      if (invalid(condition)) return false;
      if (condition.field === "health_score") {
        const score = device.health_score;
        if (typeof score !== "number" || !Number.isFinite(score)) return false;
        const target = Number(value);
        switch (condition.operator) {
          case "gte":
            return score >= target;
          case "lte":
            return score <= target;
          case "gt":
            return score > target;
          case "lt":
            return score < target;
          case "not_equals":
            return score !== target;
          default:
            return score === target;
        }
      }
      const raw = device[condition.field];
      const values = (Array.isArray(raw) ? raw : [raw || ""]).map((item) => item.toLocaleLowerCase());
      if (condition.operator === "not_equals") return values.every((item) => item !== value);
      return values.some((item) => condition.operator === "contains" ? item.includes(value) : condition.operator === "starts_with" ? item.startsWith(value) : item === value);
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

  // frontend/src/pages/dashboard.ts
  var columns = ["ip", "display_name", "name", "tags", "status", "health_score", "last_seen"];
  var sortColumn = "ip";
  var sortAscending = true;
  var deletingIps = /* @__PURE__ */ new Set();
  var selectedIds = /* @__PURE__ */ new Set();
  var searchQuery = "";
  var matchesFilters = () => true;
  function visibleDevices() {
    const query = searchQuery.trim().toLocaleLowerCase();
    return currentDevices().filter((device) => matchesFilters(device) && (!query || [device.ip, device.name, device.display_name, ...device.tags || []].some((value) => value?.toLocaleLowerCase().includes(query))));
  }
  function renderSelection() {
    const count = document.getElementById("device-selection-count");
    if (count) count.textContent = `${selectedIds.size} ${metadataText("selected")}`;
    const button = document.getElementById("bulk-rename");
    if (button) {
      button.textContent = metadataText("bulk");
      button.disabled = selectedIds.size === 0;
    }
    const all = document.getElementById("select-visible-devices");
    const visible = visibleDevices().filter((device) => typeof device.id === "number");
    if (all) {
      all.checked = visible.length > 0 && visible.every((device) => selectedIds.has(device.id));
      all.indeterminate = !all.checked && visible.some((device) => selectedIds.has(device.id));
      all.setAttribute("aria-label", metadataText("all"));
    }
    const search = document.getElementById("device-search");
    if (search) {
      search.placeholder = metadataText("search");
      search.setAttribute("aria-label", metadataText("search"));
    }
  }
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
    if (column === "health_score") return (a.health_score ?? -1) - (b.health_score ?? -1);
    const left = String(column === "display_name" ? deviceLabel(a) : column === "tags" ? (a.tags || []).join(", ") : a[column] || "").toLowerCase();
    const right = String(column === "display_name" ? deviceLabel(b) : column === "tags" ? (b.tags || []).join(", ") : b[column] || "").toLowerCase();
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
      return `<li class='attention-item'><a href='/device/${encodeURIComponent(device.ip)}'>${esc(deviceLabel(device))}</a><span class='attention-reason'>${esc(reasons.join(" / "))}</span></li>`;
    }).join("");
    element.innerHTML = `<h2>${t("attention_devices")}</h2><ul class='attention-list'>${items}</ul>`;
  }
  function renderTable() {
    const tbody = document.getElementById("dash-tbody");
    if (!tbody) return;
    const validIds = new Set(currentDevices().map((device) => device.id));
    selectedIds.forEach((id) => {
      if (!validIds.has(id)) selectedIds.delete(id);
    });
    renderSelection();
    const sorted = visibleDevices().slice().sort((a, b) => {
      if (a.error_spike && !b.error_spike) return -1;
      if (!a.error_spike && b.error_spike) return 1;
      const result = compareValue(a, b, sortColumn);
      return sortAscending ? result : -result;
    });
    if (sorted.length === 0) {
      tbody.innerHTML = currentDevices().length > 0 ? `<tr><td colspan=9 class=empty>${t("no_matching_devices")}</td></tr>` : `<tr><td colspan=9 class=empty>${t("no_devices")} <a href=/discovery>${t("nav_discovery")}</a></td></tr>`;
      return;
    }
    tbody.innerHTML = sorted.map((device) => {
      const statusBadgeClass = deviceStatusClass(device.status);
      const rowClass = device.error_spike ? " class=spike-row" : "";
      const spikeBadge = device.error_spike ? "<span class=spike-badge>&#9888; Error Spike</span>" : "";
      const deleting = deletingIps.has(device.ip);
      const actionButton = `<button type="button" class="device-action-button" data-action-device="${esc(device.ip)}" aria-haspopup="dialog" ${deleting || typeof device.id !== "number" ? "disabled" : ""}>${esc(metadataText("actions"))}</button>`;
      const lastSeen = fmtTime(device.last_seen || device.last_seen_at);
      const healthBadgeClass = deviceStatusClass(device.health_status);
      const health = typeof device.health_score === "number" && Number.isFinite(device.health_score) ? `<span class="${healthBadgeClass}">${device.health_score}/100</span><span class="health-meter ${healthBadgeClass}" role="meter" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${Math.min(100, Math.max(0, device.health_score))}" aria-label="${esc(t("health_score"))}"><span class="health-meter-fill" style="width:${Math.min(100, Math.max(0, device.health_score))}%"></span></span>` : "-";
      const checkbox = typeof device.id === "number" ? `<input type="checkbox" data-device-id="${device.id}" aria-label="${esc(`${metadataText("select")} ${device.ip}`)}" ${selectedIds.has(device.id) ? "checked" : ""}>` : "";
      const tags = (device.tags || []).map((tag) => `<span class="device-tag">${esc(tag)}</span>`).join("");
      return `<tr${rowClass}><td>${checkbox}</td><td><a href="/device/${esc(encodeURIComponent(device.ip))}" style='color:#38bdf8;text-decoration:none'>${esc(device.ip)}</a></td><td>${esc(deviceLabel(device))}${spikeBadge}</td><td>${esc(device.name || "-")}</td><td>${tags ? `<div class="device-tags">${tags}</div>` : "-"}</td><td><span class="${esc(statusBadgeClass)}">${esc(device.status)}</span></td><td>${health}</td><td>${lastSeen}</td><td><div class='row-actions'>${actionButton}</div></td></tr>`;
    }).join("");
    tbody.querySelectorAll("[data-action-device]").forEach((button) => {
      button.onclick = () => {
        const device = currentDevices().find((item) => item.ip === button.dataset.actionDevice);
        if (device) bulkActions([device], refresh, true);
      };
    });
    tbody.querySelectorAll("[data-device-id]").forEach((checkbox) => {
      checkbox.onchange = () => {
        const id = Number(checkbox.dataset.deviceId);
        if (checkbox.checked) selectedIds.add(id);
        else selectedIds.delete(id);
        renderSelection();
      };
    });
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
  function initializeManagement() {
    const table = document.getElementById("dash-table");
    if (!table || document.getElementById("device-search")) return;
    const toolbar = document.createElement("div");
    toolbar.className = "device-toolbar";
    const search = document.createElement("input");
    search.id = "device-search";
    search.type = "search";
    search.oninput = () => {
      searchQuery = search.value;
      renderTable();
    };
    const count = document.createElement("span");
    count.id = "device-selection-count";
    count.setAttribute("aria-live", "polite");
    const rename = document.createElement("button");
    rename.id = "bulk-rename";
    rename.onclick = () => bulkActions([...selectedIds].flatMap((id) => {
      const device = currentDevices().find((item) => item.id === id);
      return device ? [device] : [];
    }), refresh);
    toolbar.append(search, count, rename);
    const error = document.createElement("p");
    error.id = "device-load-error";
    error.setAttribute("role", "alert");
    toolbar.append(error);
    table.parentElement.before(toolbar);
    matchesFilters = initializeDeviceFilters(toolbar, renderTable);
    const header = document.createElement("th");
    const all = document.createElement("input");
    all.type = "checkbox";
    all.id = "select-visible-devices";
    all.onchange = () => {
      visibleDevices().forEach((device) => {
        if (typeof device.id !== "number") return;
        if (all.checked) selectedIds.add(device.id);
        else selectedIds.delete(device.id);
      });
      renderTable();
    };
    header.append(all);
    table.querySelector("thead tr").prepend(header);
  }
  function refresh() {
    fetch("/api/devices").then(async (response) => {
      const data = await response.json();
      if (!response.ok || !Array.isArray(data)) throw new Error(!Array.isArray(data) ? data.error || "Device request failed" : "Device request failed");
      return data;
    }).then((devices) => {
      window.DEVICES = devices;
      const error = document.getElementById("device-load-error");
      if (error) error.textContent = "";
      renderTable();
      renderSummary(devices);
      renderAttention(devices);
      updateLastRefreshed();
    }).catch((error) => {
      const message = document.getElementById("device-load-error");
      if (message) message.textContent = String(error);
      console.warn("refresh failed", error);
    });
  }
  window.renderTable = renderTable;
  window.renderSummary = renderSummary;
  window.refreshDashboard = refresh;
  window.sortBy = sortBy;
  window.unregisterDevice = unregisterDevice;
  initializeManagement();
  renderTable();
  renderSummary();
  renderAttention();
  updateSortIndicators();
  updateLastRefreshed();
  window.setInterval(refresh, 3e4);
  document.addEventListener("DOMContentLoaded", () => initializeOnboarding(true));
  window.addEventListener("tracepulse-language-change", () => {
    renderTable();
    renderAttention();
  });
  return __toCommonJS(dashboard_exports);
})();
