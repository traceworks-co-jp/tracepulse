interface NotificationSettings {
    slack: { enabled?: boolean; webhook_url?: string; webhook_url_env?: string };
    teams: { enabled?: boolean; webhook_url?: string; webhook_url_env?: string };
    flap_guard: { window_seconds?: number };
    retry: { max_attempts?: number };
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
    field<HTMLInputElement>("flap_window").value = String(settings.flap_guard.window_seconds || 0);
    field<HTMLInputElement>("retry_attempts").value = String(settings.retry.max_attempts || 1);
}

function notificationsPayload(): NotificationSettings {
    return {
        slack: { enabled: field<HTMLInputElement>("slack_enabled").checked, webhook_url: field<HTMLInputElement>("slack_url").value.trim(), webhook_url_env: field<HTMLInputElement>("slack_url_env").value.trim() },
        teams: { enabled: field<HTMLInputElement>("teams_enabled").checked, webhook_url: field<HTMLInputElement>("teams_url").value.trim(), webhook_url_env: field<HTMLInputElement>("teams_url_env").value.trim() },
        flap_guard: { window_seconds: parseInt(field<HTMLInputElement>("flap_window").value, 10) },
        retry: { max_attempts: parseInt(field<HTMLInputElement>("retry_attempts").value, 10) },
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
