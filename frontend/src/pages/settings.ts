import { initializeOnboarding } from "../shared/onboarding";

const metrics = ["cpu", "memory", "bandwidth", "error_rate"] as const;
type Metric = typeof metrics[number];
interface MetricThresholds { warning: number; critical: number; }

interface SettingsFormValues {
    interval: number;
    community: string;
    timezone: string;
    error_rate: number;
    spike: number;
    warn: number;
    crit: number;
    days: number;
    thresholds?: Record<Metric, MetricThresholds>;
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
    for (const metric of metrics) {
        const thresholds = values.thresholds?.[metric] || (metric === "error_rate" ? { warning: 5, critical: 10 } : { warning: 80, critical: 90 });
        element<HTMLInputElement>(`${metric}_warning`).value = String(thresholds.warning);
        element<HTMLInputElement>(`${metric}_critical`).value = String(thresholds.critical);
    }
    element<HTMLInputElement>("spike").value = String(values.spike);
    element<HTMLInputElement>("warn_t").value = String(values.warn);
    element<HTMLInputElement>("crit_t").value = String(values.crit);
    element<HTMLInputElement>("days").value = String(values.days);
    updateBar();
}

export function updateBar(): void {
    const warning = element<HTMLInputElement>("warn_t").valueAsNumber;
    const critical = element<HTMLInputElement>("crit_t").valueAsNumber;
    const valid = Number.isInteger(warning) && Number.isInteger(critical)
        && warning > critical && warning <= 100 && critical >= 0;
    element<HTMLElement>("bar-label").hidden = !valid;
    if (!valid) return;
    const bar = element<HTMLElement>("tbar");
    bar.style.setProperty("--warn", `${warning}%`);
    bar.style.setProperty("--crit", `${critical}%`);
    const ranges = { warning, critical, upper: warning - 1 };
    for (const status of ["healthy", "warning", "critical"]) {
        element<HTMLElement>(`legend-${status}`).textContent = window.tf?.(`score_legend_${status}`, ranges) || t(`score_legend_${status}`);
    }
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
    const thresholds = {} as Record<Metric, MetricThresholds>;
    for (const metric of metrics) {
        const warningInput = element<HTMLInputElement>(`${metric}_warning`);
        const criticalInput = element<HTMLInputElement>(`${metric}_critical`);
        const warning = warningInput.valueAsNumber;
        const critical = criticalInput.valueAsNumber;
        if (!warningInput.checkValidity() || !criticalInput.checkValidity() || !Number.isFinite(warning) || !Number.isFinite(critical) || warning >= critical) {
            showToast(false, t("metric_threshold_invalid"));
            warningInput.focus();
            return;
        }
        thresholds[metric] = { warning, critical };
    }
    const payload = {
        interval_seconds: parseInt(element<HTMLInputElement>("interval").value, 10),
        default_community: element<HTMLInputElement>("community").value,
        timezone,
        ...thresholds,
        spike_threshold: parseInt(element<HTMLInputElement>("spike").value, 10),
        health_warning_threshold: parseInt(element<HTMLInputElement>("warn_t").value, 10),
        health_critical_threshold: parseInt(element<HTMLInputElement>("crit_t").value, 10),
        history_days: parseInt(element<HTMLInputElement>("days").value, 10),
    };
    fetch("/api/settings", { method: "POST", headers: { "Content-Type": "application/json", "X-CSRF-Token": window.tracepulseCsrf || "" }, body: JSON.stringify(payload) })
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
window.addEventListener("tracepulse-language-change", updateBar);
document.addEventListener("DOMContentLoaded", () => initializeOnboarding());
