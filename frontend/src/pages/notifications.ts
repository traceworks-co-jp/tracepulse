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
    return fetch("/api/notifications", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(notificationsPayload()) })
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
        fetch("/api/notifications/test", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ channel }) })
            .then((response) => response.json())
            .then((data: { ok?: boolean; message?: string; error?: string }) => showNotifyToast(!!data.ok, data.ok ? data.message || t("test_sent") : data.error || "Unknown error"))
            .catch((error: unknown) => showNotifyToast(false, String(error)));
    });
}

window.saveNotifications = saveNotifications;
window.testNotification = testNotification;
fetch("/api/notifications").then((response) => response.json()).then((data: NotificationSettings & { error?: string }) => { if (!data.error) populateNotifications(data); }).catch(() => undefined);
