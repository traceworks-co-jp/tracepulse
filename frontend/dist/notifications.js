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
    field("webhook_enabled").checked = !!settings.webhook?.enabled;
    field("webhook_url").value = settings.webhook?.webhook_url || "";
    field("webhook_url_env").value = settings.webhook?.webhook_url_env || "";
    field("syslog_enabled").checked = !!settings.syslog?.enabled;
    field("syslog_host").value = settings.syslog?.host || "";
    field("syslog_port").value = String(settings.syslog?.port || 514);
    field("syslog_transport").value = settings.syslog?.transport || "udp";
    field("syslog_format").value = settings.syslog?.format || "cef";
    field("syslog_app_name").value = settings.syslog?.app_name || "tracepulse-enterprise";
    field("smtp_enabled").checked = !!settings.smtp?.enabled;
    field("smtp_host").value = settings.smtp?.host || "";
    field("smtp_port").value = String(settings.smtp?.port || 587);
    field("smtp_from").value = settings.smtp?.from || "";
    field("smtp_to").value = (settings.smtp?.to || []).join(", ");
    field("smtp_username").value = settings.smtp?.username || "";
    field("smtp_password_env").value = settings.smtp?.password_env || "";
    field("smtp_starttls").checked = settings.smtp?.starttls !== false;
    field("flap_window").value = String(settings.flap_guard.window_seconds || 0);
    field("retry_attempts").value = String(settings.retry.max_attempts || 1);
    field("send_resolved").checked = settings.send_resolved !== false;
  }
  function notificationsPayload() {
    return {
      slack: { enabled: field("slack_enabled").checked, webhook_url: field("slack_url").value.trim(), webhook_url_env: field("slack_url_env").value.trim() },
      teams: { enabled: field("teams_enabled").checked, webhook_url: field("teams_url").value.trim(), webhook_url_env: field("teams_url_env").value.trim() },
      webhook: { enabled: field("webhook_enabled").checked, webhook_url: field("webhook_url").value.trim(), webhook_url_env: field("webhook_url_env").value.trim() },
      syslog: { enabled: field("syslog_enabled").checked, host: field("syslog_host").value.trim(), port: parseInt(field("syslog_port").value, 10), transport: field("syslog_transport").value, format: field("syslog_format").value, app_name: field("syslog_app_name").value.trim() },
      smtp: { enabled: field("smtp_enabled").checked, host: field("smtp_host").value.trim(), port: parseInt(field("smtp_port").value, 10), from: field("smtp_from").value.trim(), to: field("smtp_to").value.split(",").map((value) => value.trim()).filter(Boolean), username: field("smtp_username").value.trim() || void 0, password_env: field("smtp_password_env").value.trim() || void 0, starttls: field("smtp_starttls").checked },
      flap_guard: { window_seconds: parseInt(field("flap_window").value, 10) },
      retry: { max_attempts: parseInt(field("retry_attempts").value, 10) },
      send_resolved: field("send_resolved").checked
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
