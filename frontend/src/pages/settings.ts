interface SettingsFormValues {
    interval: number;
    community: string;
    timezone: string;
    error_rate: number;
    spike: number;
    warn: number;
    crit: number;
    days: number;
}

function element<T extends HTMLElement>(id: string): T {
    return document.getElementById(id) as T;
}

function t(key: string): string {
    return typeof window.t === "function" ? window.t(key) : key;
}

export function populateForm(values: SettingsFormValues): void {
    (element<HTMLInputElement>("interval").value = String(values.interval));
    element<HTMLInputElement>("community").value = values.community;
    element<HTMLSelectElement>("timezone").value = values.timezone || "utc";
    try { localStorage.setItem("tracepulse-timezone", element<HTMLSelectElement>("timezone").value); } catch { /* ignore */ }
    element<HTMLInputElement>("error_rate").value = String(values.error_rate);
    element<HTMLInputElement>("spike").value = String(values.spike);
    element<HTMLInputElement>("warn_t").value = String(values.warn);
    element<HTMLInputElement>("crit_t").value = String(values.crit);
    element<HTMLInputElement>("days").value = String(values.days);
    updateBar();
}

export function updateBar(): void {
    const warning = parseInt(element<HTMLInputElement>("warn_t").value, 10) || 80;
    const critical = parseInt(element<HTMLInputElement>("crit_t").value, 10) || 60;
    const bar = element<HTMLElement>("tbar");
    bar.style.setProperty("--warn", `${warning}%`);
    bar.style.setProperty("--crit", `${critical}%`);
    element<HTMLElement>("bar-label").textContent = `crit < ${critical} <= warn < ${warning} <= online`;
}

function showToast(ok: boolean, message?: string): void {
    const success = element<HTMLElement>("toast-ok");
    const failure = element<HTMLElement>("toast-err");
    success.style.display = "none";
    failure.style.display = "none";
    if (ok) {
        success.style.display = "inline-block";
        window.setTimeout(() => { success.style.display = "none"; }, 3000);
    } else {
        failure.textContent = `x ${message || "Unknown error"}`;
        failure.style.display = "inline-block";
        window.setTimeout(() => { failure.style.display = "none"; }, 5000);
    }
}

export function saveSettings(): void {
    const timezone = element<HTMLSelectElement>("timezone").value;
    const payload = {
        interval_seconds: parseInt(element<HTMLInputElement>("interval").value, 10),
        default_community: element<HTMLInputElement>("community").value,
        timezone,
        error_rate_threshold: parseFloat(element<HTMLInputElement>("error_rate").value),
        spike_threshold: parseInt(element<HTMLInputElement>("spike").value, 10),
        health_warning_threshold: parseInt(element<HTMLInputElement>("warn_t").value, 10),
        health_critical_threshold: parseInt(element<HTMLInputElement>("crit_t").value, 10),
        history_days: parseInt(element<HTMLInputElement>("days").value, 10),
    };
    fetch("/api/settings", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload) })
        .then((response) => response.json())
        .then((data: { ok?: boolean; error?: string }) => {
            if (data.ok) {
                try { localStorage.setItem("tracepulse-timezone", timezone); } catch { /* ignore */ }
                showToast(true);
            } else showToast(false, data.error);
        })
        .catch((error: unknown) => showToast(false, String(error)));
}

export function resetDefaults(): void {
    populateForm({ interval: 30, community: "public", timezone: "utc", error_rate: 0.05, spike: 10, warn: 80, crit: 60, days: 7 });
}

window.populateForm = populateForm;
window.resetDefaults = resetDefaults;
window.saveSettings = saveSettings;
window.updateBar = updateBar;
populateForm(window.INIT);
