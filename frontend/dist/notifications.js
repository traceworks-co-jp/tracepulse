"use strict";
(() => {
  // frontend/src/pages/notifications.ts
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
    return fetch("/api/notifications", { method: "POST", headers: { "Content-Type": "application/json", ...window.tracepulseRequestHeaders?.() }, body: JSON.stringify(notificationsPayload()) }).then((response) => response.json()).then((data) => {
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
      fetch("/api/notifications/test", { method: "POST", headers: { "Content-Type": "application/json", ...window.tracepulseRequestHeaders?.() }, body: JSON.stringify({ channel }) }).then((response) => response.json()).then((data) => showNotifyToast(!!data.ok, data.ok ? data.message || t("test_sent") : data.error || "Unknown error")).catch((error) => showNotifyToast(false, String(error)));
    });
  }
  window.saveNotifications = saveNotifications;
  window.testNotification = testNotification;
  window.registerTranslations?.({
    en: {
      notify_destinations: "Alerting & Destinations",
      notify_enable_slack: "Enable Slack",
      notify_enable_teams: "Enable Teams",
      notify_enable_syslog: "Enable Syslog",
      notify_enable_webhook: "Enable Webhook",
      notify_enable_smtp: "Enable SMTP",
      notify_generic_webhook: "Generic Webhook",
      notify_webhook_url: "Webhook URL",
      notify_siem_host: "SIEM host or IP",
      notify_app_name: "App name",
      notify_smtp_host: "SMTP host",
      notify_from_address: "From address",
      notify_recipients: "Recipients, comma separated",
      notify_username: "Username",
      notify_password_env: "Password environment variable",
      notify_starttls: "STARTTLS",
      notify_send_resolved: "Send [RESOLVED] recovery notifications",
      notifications: "Alert Notifications (Slack / Teams)",
      slack_enabled: "Enable Slack notifications",
      teams_enabled: "Enable Teams notifications",
      webhook_url_hint: "Leave empty to fall back to the environment variable below. ${ENV_NAME} placeholders are expanded.",
      webhook_env_hint: "Environment variable used when no URL is set",
      flap_window: "Flap Guard Window (seconds)",
      flap_window_hint: "Alerts of the same kind are aggregated within this window",
      retry_max_attempts: "Retry Attempts",
      retry_max_attempts_hint: "Number of webhook delivery attempts (1 - 10)",
      save_notifications: "Save Notification Settings",
      send_test: "Send test notification",
      send_test_all: "Test all enabled channels",
      notifications_saved: "Notification settings saved",
      sending_test: "Sending test notification...",
      test_sent: "Test notification sent"
    },
    ja: {
      notify_destinations: "\u30A2\u30E9\u30FC\u30C8\u901A\u77E5\u3068\u9001\u4FE1\u5148",
      notify_enable_slack: "Slack \u3092\u6709\u52B9\u306B\u3059\u308B",
      notify_enable_teams: "Teams \u3092\u6709\u52B9\u306B\u3059\u308B",
      notify_enable_syslog: "Syslog \u3092\u6709\u52B9\u306B\u3059\u308B",
      notify_enable_webhook: "Webhook \u3092\u6709\u52B9\u306B\u3059\u308B",
      notify_enable_smtp: "SMTP \u3092\u6709\u52B9\u306B\u3059\u308B",
      notify_generic_webhook: "\u6C4E\u7528 Webhook",
      notify_webhook_url: "Webhook URL",
      notify_siem_host: "SIEM \u30DB\u30B9\u30C8\u540D\u307E\u305F\u306F IP \u30A2\u30C9\u30EC\u30B9",
      notify_app_name: "\u30A2\u30D7\u30EA\u30B1\u30FC\u30B7\u30E7\u30F3\u540D",
      notify_smtp_host: "SMTP \u30DB\u30B9\u30C8\u540D",
      notify_from_address: "\u9001\u4FE1\u5143\u30A2\u30C9\u30EC\u30B9",
      notify_recipients: "\u5B9B\u5148\uFF08\u30AB\u30F3\u30DE\u533A\u5207\u308A\uFF09",
      notify_username: "\u30E6\u30FC\u30B6\u30FC\u540D",
      notify_password_env: "\u30D1\u30B9\u30EF\u30FC\u30C9\u306E\u74B0\u5883\u5909\u6570\u540D",
      notify_starttls: "STARTTLS \u3092\u4F7F\u7528",
      notify_send_resolved: "\u5FA9\u65E7\u901A\u77E5 [RESOLVED] \u3092\u9001\u4FE1\u3059\u308B",
      notifications: "\u30A2\u30E9\u30FC\u30C8\u901A\u77E5\uFF08Slack / Teams\uFF09",
      slack_enabled: "Slack \u901A\u77E5\u3092\u6709\u52B9\u306B\u3059\u308B",
      teams_enabled: "Teams \u901A\u77E5\u3092\u6709\u52B9\u306B\u3059\u308B",
      webhook_url_hint: "\u7A7A\u306E\u5834\u5408\u306F\u4E0B\u306E\u74B0\u5883\u5909\u6570\u304B\u3089\u53D6\u5F97\u3057\u307E\u3059\u3002${\u74B0\u5883\u5909\u6570\u540D} \u5F62\u5F0F\u306E\u5C55\u958B\u306B\u3082\u5BFE\u5FDC\u3057\u307E\u3059\u3002",
      webhook_env_hint: "URL \u672A\u8A2D\u5B9A\u6642\u306B\u53C2\u7167\u3059\u308B\u74B0\u5883\u5909\u6570\u540D",
      flap_window: "\u30D5\u30E9\u30C3\u30D7\u6291\u5236\u30A6\u30A3\u30F3\u30C9\u30A6\uFF08\u79D2\uFF09",
      flap_window_hint: "\u540C\u7A2E\u306E\u30A2\u30E9\u30FC\u30C8\u3092\u3053\u306E\u671F\u9593\u5185\u3067\u96C6\u7D04\u3057\u3066\u901A\u77E5\u3057\u307E\u3059",
      retry_max_attempts: "\u30EA\u30C8\u30E9\u30A4\u56DE\u6570",
      retry_max_attempts_hint: "Webhook \u9001\u4FE1\u306E\u8A66\u884C\u56DE\u6570\uFF081\u301C10\uFF09",
      save_notifications: "\u901A\u77E5\u8A2D\u5B9A\u3092\u4FDD\u5B58",
      send_test: "\u30C6\u30B9\u30C8\u901A\u77E5\u3092\u9001\u4FE1",
      send_test_all: "\u6709\u52B9\u306A\u30C1\u30E3\u30F3\u30CD\u30EB\u3092\u4E00\u62EC\u30C6\u30B9\u30C8",
      notifications_saved: "\u901A\u77E5\u8A2D\u5B9A\u3092\u4FDD\u5B58\u3057\u307E\u3057\u305F",
      sending_test: "\u30C6\u30B9\u30C8\u901A\u77E5\u3092\u9001\u4FE1\u4E2D\u2026",
      test_sent: "\u30C6\u30B9\u30C8\u901A\u77E5\u3092\u9001\u4FE1\u3057\u307E\u3057\u305F"
    }
  });
  window.applyLanguage?.();
  fetch("/api/notifications").then(async (response) => {
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return await response.json();
  }).then((data) => {
    if (data.error) throw new Error(data.error);
    populateNotifications(data);
  }).catch((error) => showNotifyToast(false, String(error)));
})();
