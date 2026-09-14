"use strict";
var TracePulseNotifications = (() => {
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

  // frontend/src/pages/notifications.ts
  var notifications_exports = {};
  __export(notifications_exports, {
    saveNotifications: () => saveNotifications,
    testNotification: () => testNotification
  });
  function field(id) {
    return document.getElementById(id);
  }
  function t(key) {
    return typeof window.t === "function" ? window.t(key) : key;
  }
  function showNotifyToast(ok, message) {
    const success = field("ntoast-ok");
    const failure = field("ntoast-err");
    success.style.display = "none";
    failure.style.display = "none";
    const target = ok ? success : failure;
    target.textContent = `${ok ? "+" : "x"} ${message}`;
    target.style.display = "inline-block";
    window.setTimeout(() => {
      target.style.display = "none";
    }, ok ? 5e3 : 8e3);
  }
  function populateNotifications(settings) {
    field("slack_enabled").checked = !!settings.slack.enabled;
    field("slack_url").value = settings.slack.webhook_url || "";
    field("slack_url_env").value = settings.slack.webhook_url_env || "";
    field("teams_enabled").checked = !!settings.teams.enabled;
    field("teams_url").value = settings.teams.webhook_url || "";
    field("teams_url_env").value = settings.teams.webhook_url_env || "";
    field("flap_window").value = String(settings.flap_guard.window_seconds || 0);
    field("retry_attempts").value = String(settings.retry.max_attempts || 1);
  }
  function notificationsPayload() {
    return {
      slack: { enabled: field("slack_enabled").checked, webhook_url: field("slack_url").value.trim(), webhook_url_env: field("slack_url_env").value.trim() },
      teams: { enabled: field("teams_enabled").checked, webhook_url: field("teams_url").value.trim(), webhook_url_env: field("teams_url_env").value.trim() },
      flap_guard: { window_seconds: parseInt(field("flap_window").value, 10) },
      retry: { max_attempts: parseInt(field("retry_attempts").value, 10) }
    };
  }
  function saveNotifications() {
    return fetch("/api/notifications", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(notificationsPayload()) }).then((response) => response.json()).then((data) => {
      if (data.ok) {
        showNotifyToast(true, t("notifications_saved"));
        return true;
      }
      showNotifyToast(false, data.error || "Unknown error");
      return false;
    }).catch((error) => {
      showNotifyToast(false, String(error));
      return false;
    });
  }
  function testNotification(channel) {
    saveNotifications().then((saved) => {
      if (!saved) return;
      showNotifyToast(true, t("sending_test"));
      fetch("/api/notifications/test", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ channel }) }).then((response) => response.json()).then((data) => showNotifyToast(!!data.ok, data.ok ? data.message || t("test_sent") : data.error || "Unknown error")).catch((error) => showNotifyToast(false, String(error)));
    });
  }
  window.saveNotifications = saveNotifications;
  window.testNotification = testNotification;
  fetch("/api/notifications").then((response) => response.json()).then((data) => {
    if (!data.error) populateNotifications(data);
  }).catch(() => void 0);
  return __toCommonJS(notifications_exports);
})();
