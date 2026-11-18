interface NotificationSettings {
    slack: { enabled?: boolean; webhook_url?: string; webhook_url_env?: string };
    teams: { enabled?: boolean; webhook_url?: string; webhook_url_env?: string };
    webhook: { enabled?: boolean; webhook_url?: string; webhook_url_env?: string };
    syslog: { enabled?: boolean; host?: string; port?: number; transport?: string; format?: string; app_name?: string };
    smtp: { enabled?: boolean; host?: string; port?: number; from?: string; to?: string[]; username?: string; password_env?: string; starttls?: boolean };
    flap_guard: { window_seconds?: number };
    retry: { max_attempts?: number };
    send_resolved?: boolean;
}

declare global {
    interface Window {
        t?: (key: string) => string;
        applyLanguage?: (language?: string) => void;
        saveNotifications?: () => Promise<boolean>;
        testNotification?: (channel: string) => void;
    }
}

function field<T extends HTMLElement>(id: string): T { return document.getElementById(id) as T; }
function t(key: string): string { return typeof window.t === "function" ? window.t(key) : key; }

function showNotifyToast(ok: boolean, message: string): void {
    const success = field<HTMLElement>("ntoast-ok");
    const failure = field<HTMLElement>("ntoast-err");
    success.style.display = "none";
    failure.style.display = "none";
    const target = ok ? success : failure;
    target.textContent = `${ok ? "+" : "x"} ${message}`;
    target.style.display = "inline-block";
    window.setTimeout(() => { target.style.display = "none"; }, ok ? 5000 : 8000);
}

function populateNotifications(settings: NotificationSettings): void {
    field<HTMLInputElement>("slack_enabled").checked = !!settings.slack.enabled;
    field<HTMLInputElement>("slack_url").value = settings.slack.webhook_url || "";
    field<HTMLInputElement>("slack_url_env").value = settings.slack.webhook_url_env || "";
    field<HTMLInputElement>("teams_enabled").checked = !!settings.teams.enabled;
    field<HTMLInputElement>("teams_url").value = settings.teams.webhook_url || "";
    field<HTMLInputElement>("teams_url_env").value = settings.teams.webhook_url_env || "";
    field<HTMLInputElement>("webhook_enabled").checked = !!settings.webhook?.enabled;
    field<HTMLInputElement>("webhook_url").value = settings.webhook?.webhook_url || "";
    field<HTMLInputElement>("webhook_url_env").value = settings.webhook?.webhook_url_env || "";
    field<HTMLInputElement>("syslog_enabled").checked = !!settings.syslog?.enabled;
    field<HTMLInputElement>("syslog_host").value = settings.syslog?.host || "";
    field<HTMLInputElement>("syslog_port").value = String(settings.syslog?.port || 514);
    field<HTMLSelectElement>("syslog_transport").value = settings.syslog?.transport || "udp";
    field<HTMLSelectElement>("syslog_format").value = settings.syslog?.format || "cef";
    field<HTMLInputElement>("syslog_app_name").value = settings.syslog?.app_name || "tracepulse-enterprise";
    field<HTMLInputElement>("smtp_enabled").checked = !!settings.smtp?.enabled;
    field<HTMLInputElement>("smtp_host").value = settings.smtp?.host || "";
    field<HTMLInputElement>("smtp_port").value = String(settings.smtp?.port || 587);
    field<HTMLInputElement>("smtp_from").value = settings.smtp?.from || "";
    field<HTMLInputElement>("smtp_to").value = (settings.smtp?.to || []).join(", ");
    field<HTMLInputElement>("smtp_username").value = settings.smtp?.username || "";
    field<HTMLInputElement>("smtp_password_env").value = settings.smtp?.password_env || "";
    field<HTMLInputElement>("smtp_starttls").checked = settings.smtp?.starttls !== false;
    field<HTMLInputElement>("flap_window").value = String(settings.flap_guard.window_seconds || 0);
    field<HTMLInputElement>("retry_attempts").value = String(settings.retry.max_attempts || 1);
    field<HTMLInputElement>("send_resolved").checked = settings.send_resolved !== false;
}

function notificationsPayload(): NotificationSettings {
    return {
        slack: { enabled: field<HTMLInputElement>("slack_enabled").checked, webhook_url: field<HTMLInputElement>("slack_url").value.trim(), webhook_url_env: field<HTMLInputElement>("slack_url_env").value.trim() },
        teams: { enabled: field<HTMLInputElement>("teams_enabled").checked, webhook_url: field<HTMLInputElement>("teams_url").value.trim(), webhook_url_env: field<HTMLInputElement>("teams_url_env").value.trim() },
        webhook: { enabled: field<HTMLInputElement>("webhook_enabled").checked, webhook_url: field<HTMLInputElement>("webhook_url").value.trim(), webhook_url_env: field<HTMLInputElement>("webhook_url_env").value.trim() },
        syslog: { enabled: field<HTMLInputElement>("syslog_enabled").checked, host: field<HTMLInputElement>("syslog_host").value.trim(), port: parseInt(field<HTMLInputElement>("syslog_port").value, 10), transport: field<HTMLSelectElement>("syslog_transport").value, format: field<HTMLSelectElement>("syslog_format").value, app_name: field<HTMLInputElement>("syslog_app_name").value.trim() },
        smtp: { enabled: field<HTMLInputElement>("smtp_enabled").checked, host: field<HTMLInputElement>("smtp_host").value.trim(), port: parseInt(field<HTMLInputElement>("smtp_port").value, 10), from: field<HTMLInputElement>("smtp_from").value.trim(), to: field<HTMLInputElement>("smtp_to").value.split(",").map((value) => value.trim()).filter(Boolean), username: field<HTMLInputElement>("smtp_username").value.trim() || undefined, password_env: field<HTMLInputElement>("smtp_password_env").value.trim() || undefined, starttls: field<HTMLInputElement>("smtp_starttls").checked },
        flap_guard: { window_seconds: parseInt(field<HTMLInputElement>("flap_window").value, 10) },
        retry: { max_attempts: parseInt(field<HTMLInputElement>("retry_attempts").value, 10) },
        send_resolved: field<HTMLInputElement>("send_resolved").checked,
    };
}

export function saveNotifications(): Promise<boolean> {
    return fetch("/api/notifications", { method: "POST", headers: { "Content-Type": "application/json", ...window.tracepulseRequestHeaders?.() }, body: JSON.stringify(notificationsPayload()) })
        .then((response) => response.json())
        .then((data: { ok?: boolean; error?: string }) => {
            if (data.ok) { showNotifyToast(true, t("notifications_saved")); return true; }
            showNotifyToast(false, data.error || "Unknown error"); return false;
        })
        .catch((error: unknown) => { showNotifyToast(false, String(error)); return false; });
}

export function testNotification(channel: string): void {
    saveNotifications().then((saved) => {
        if (!saved) return;
        showNotifyToast(true, t("sending_test"));
        fetch("/api/notifications/test", { method: "POST", headers: { "Content-Type": "application/json", ...window.tracepulseRequestHeaders?.() }, body: JSON.stringify({ channel }) })
            .then((response) => response.json())
            .then((data: { ok?: boolean; message?: string; error?: string }) => showNotifyToast(!!data.ok, data.ok ? data.message || t("test_sent") : data.error || "Unknown error"))
            .catch((error: unknown) => showNotifyToast(false, String(error)));
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
        test_sent: "Test notification sent",
    },
    ja: {
        notify_destinations: "アラート通知と送信先",
        notify_enable_slack: "Slack を有効にする",
        notify_enable_teams: "Teams を有効にする",
        notify_enable_syslog: "Syslog を有効にする",
        notify_enable_webhook: "Webhook を有効にする",
        notify_enable_smtp: "SMTP を有効にする",
        notify_generic_webhook: "汎用 Webhook",
        notify_webhook_url: "Webhook URL",
        notify_siem_host: "SIEM ホスト名または IP アドレス",
        notify_app_name: "アプリケーション名",
        notify_smtp_host: "SMTP ホスト名",
        notify_from_address: "送信元アドレス",
        notify_recipients: "宛先（カンマ区切り）",
        notify_username: "ユーザー名",
        notify_password_env: "パスワードの環境変数名",
        notify_starttls: "STARTTLS を使用",
        notify_send_resolved: "復旧通知 [RESOLVED] を送信する",
        notifications: "アラート通知（Slack / Teams）",
        slack_enabled: "Slack 通知を有効にする",
        teams_enabled: "Teams 通知を有効にする",
        webhook_url_hint: "空の場合は下の環境変数から取得します。${環境変数名} 形式の展開にも対応します。",
        webhook_env_hint: "URL 未設定時に参照する環境変数名",
        flap_window: "フラップ抑制ウィンドウ（秒）",
        flap_window_hint: "同種のアラートをこの期間内で集約して通知します",
        retry_max_attempts: "リトライ回数",
        retry_max_attempts_hint: "Webhook 送信の試行回数（1〜10）",
        save_notifications: "通知設定を保存",
        send_test: "テスト通知を送信",
        send_test_all: "有効なチャンネルを一括テスト",
        notifications_saved: "通知設定を保存しました",
        sending_test: "テスト通知を送信中…",
        test_sent: "テスト通知を送信しました",
    },
});
window.applyLanguage?.();
fetch("/api/notifications").then(async response => {
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return await response.json() as NotificationSettings & { error?: string };
}).then(data => {
    if (data.error) throw new Error(data.error);
    populateNotifications(data);
}).catch((error: unknown) => showNotifyToast(false, String(error)));