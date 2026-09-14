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
  function element(id) {
    return document.getElementById(id);
  }
  function populateForm(values) {
    element("interval").value = String(values.interval);
    element("community").value = values.community;
    element("timezone").value = values.timezone || "utc";
    try {
      localStorage.setItem("tracepulse-timezone", element("timezone").value);
    } catch {
    }
    element("error_rate").value = String(values.error_rate);
    element("spike").value = String(values.spike);
    element("warn_t").value = String(values.warn);
    element("crit_t").value = String(values.crit);
    element("days").value = String(values.days);
    updateBar();
  }
  function updateBar() {
    const warning = parseInt(element("warn_t").value, 10) || 80;
    const critical = parseInt(element("crit_t").value, 10) || 60;
    const bar = element("tbar");
    bar.style.setProperty("--warn", `${warning}%`);
    bar.style.setProperty("--crit", `${critical}%`);
    element("bar-label").textContent = `crit < ${critical} <= warn < ${warning} <= online`;
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
    const payload = {
      interval_seconds: parseInt(element("interval").value, 10),
      default_community: element("community").value,
      timezone,
      error_rate_threshold: parseFloat(element("error_rate").value),
      spike_threshold: parseInt(element("spike").value, 10),
      health_warning_threshold: parseInt(element("warn_t").value, 10),
      health_critical_threshold: parseInt(element("crit_t").value, 10),
      history_days: parseInt(element("days").value, 10)
    };
    fetch("/api/settings", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload) }).then((response) => response.json()).then((data) => {
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
  return __toCommonJS(settings_exports);
})();
