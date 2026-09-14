"use strict";
var TracePulseDiagnostics = (() => {
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

  // frontend/src/pages/diagnostics.ts
  var diagnostics_exports = {};
  __export(diagnostics_exports, {
    saveOidOverrides: () => saveOidOverrides,
    startDiagnostics: () => startDiagnostics
  });
  function t(key) {
    return typeof window.t === "function" ? window.t(key) : key;
  }
  function startDiagnostics(event) {
    event.preventDefault();
    const form = event.target;
    const button = document.getElementById("diag-run-btn");
    const progress = document.getElementById("diag-progress");
    if (button) button.disabled = true;
    if (progress) progress.textContent = t("diagnostics_running");
    const url = `${form.action}?${new URLSearchParams(new FormData(form)).toString()}`;
    fetch(url, { cache: "no-store" }).then((response) => response.text()).then((html) => {
      document.open();
      document.write(html);
      document.close();
    }).catch((error) => {
      if (progress) progress.textContent = String(error);
      if (button) button.disabled = false;
    });
    return false;
  }
  function collectHardwareOids() {
    return Array.from(document.querySelectorAll(".hardware-oid-input")).map((input) => {
      const oid = input.value.trim();
      const type = input.getAttribute("data-sensor-type") || "temperature";
      return oid ? `${type}|${oid}` : "";
    }).filter(Boolean);
  }
  function saveOidOverrides() {
    const payload = {
      cpu_oid_override: document.getElementById("cpu-oid-override")?.value || "",
      memory_oid_override: document.getElementById("memory-oid-override")?.value || "",
      hardware_oid_overrides: collectHardwareOids()
    };
    fetch("/api/diagnostics/oids", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload) }).then((response) => response.json()).then((data) => {
      const element = document.getElementById("oid-save-result");
      if (!element) return;
      if (data.ok) {
        element.className = "diag-note";
        element.textContent = t("settings_saved");
        window.setTimeout(() => window.location.reload(), 300);
      } else {
        element.textContent = data.error || "Save failed";
        element.className = "diag-note diag-error";
      }
    }).catch((error) => {
      const element = document.getElementById("oid-save-result");
      if (element) {
        element.textContent = String(error);
        element.className = "diag-note diag-error";
      }
    });
  }
  window.saveOidOverrides = saveOidOverrides;
  window.startDiagnostics = startDiagnostics;
  return __toCommonJS(diagnostics_exports);
})();
