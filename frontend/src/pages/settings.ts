import { initializeOnboarding } from "../shared/onboarding";

const metrics = ["cpu", "memory", "bandwidth", "error_rate"] as const;
type Metric = typeof metrics[number];
interface MetricThresholds { warning: number; critical: number; }
const validatedMetrics = new Set<Metric>();
let toastTimer: number | undefined;

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

function fieldError(input: HTMLInputElement, message: string): void {
    const id = `${input.id}-error`;
    let error = document.getElementById(id);
    if (!error) {
        error = document.createElement("p");
        error.id = id;
        error.className = "settings-field-error";
        error.setAttribute("aria-live", "polite");
        input.after(error);
        const descriptions = new Set((input.getAttribute("aria-describedby") || "").split(" ").filter(Boolean));
        descriptions.add(id);
        input.setAttribute("aria-describedby", [...descriptions].join(" "));
    }
    error.textContent = message;
    error.hidden = !message;
    input.setAttribute("aria-invalid", String(Boolean(message)));
}

function validateMetric(metric: Metric): boolean {
    validatedMetrics.add(metric);
    const warning = element<HTMLInputElement>(`${metric}_warning`);
    const critical = element<HTMLInputElement>(`${metric}_critical`);
    let warningError = !warning.value.trim() || !Number.isFinite(warning.valueAsNumber) || !warning.checkValidity() ? t("metric_threshold_range") : "";
    let criticalError = !critical.value.trim() || !Number.isFinite(critical.valueAsNumber) || !critical.checkValidity() ? t("metric_threshold_range") : "";
    if (!warningError && !criticalError && warning.valueAsNumber >= critical.valueAsNumber) {
        warningError = t("metric_warning_order");
        criticalError = t("metric_critical_order");
    }
    fieldError(warning, warningError);
    fieldError(critical, criticalError);
    return !warningError && !criticalError;
}

export function populateForm(values: SettingsFormValues): void {
    validatedMetrics.clear();
    (element<HTMLInputElement>("interval").value = String(values.interval));
    element<HTMLSelectElement>("timezone").value = values.timezone || "utc";
    try { localStorage.setItem("tracepulse-timezone", element<HTMLSelectElement>("timezone").value); } catch { /* ignore */ }
    for (const metric of metrics) {
        const thresholds = values.thresholds?.[metric] || (metric === "error_rate" ? { warning: 5, critical: 10 } : { warning: 80, critical: 90 });
        element<HTMLInputElement>(`${metric}_warning`).value = String(thresholds.warning);
        element<HTMLInputElement>(`${metric}_critical`).value = String(thresholds.critical);
        fieldError(element<HTMLInputElement>(`${metric}_warning`), "");
        fieldError(element<HTMLInputElement>(`${metric}_critical`), "");
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
    window.clearTimeout(toastTimer);
    const success = element<HTMLElement>("toast-ok");
    const failure = element<HTMLElement>("toast-err");
    success.style.display = "none";
    failure.style.display = "none";
    if (ok) {
        success.style.display = "inline-block";
        toastTimer = window.setTimeout(() => { success.style.display = "none"; }, 3000);
    } else {
        failure.textContent = message || t("settings_save_failed");
        failure.style.display = "inline-block";
        toastTimer = window.setTimeout(() => { failure.style.display = "none"; }, 5000);
    }
}

export function saveSettings(): void {
    const timezone = element<HTMLSelectElement>("timezone").value;
    const thresholds = {} as Record<Metric, MetricThresholds>;
    let firstInvalid: HTMLInputElement | undefined;
    for (const metric of metrics) {
        const warningInput = element<HTMLInputElement>(`${metric}_warning`);
        const criticalInput = element<HTMLInputElement>(`${metric}_critical`);
        const warning = warningInput.valueAsNumber;
        const critical = criticalInput.valueAsNumber;
        if (!validateMetric(metric) && !firstInvalid) {
            firstInvalid = warningInput.getAttribute("aria-invalid") === "true" ? warningInput : criticalInput;
        }
        thresholds[metric] = { warning, critical };
    }
    if (firstInvalid) {
        showToast(false, t("settings_invalid_input"));
        firstInvalid.scrollIntoView({ block: "center" });
        firstInvalid.focus({ preventScroll: true });
        return;
    }
    const payload = {
        interval_seconds: parseInt(element<HTMLInputElement>("interval").value, 10),
        timezone,
        ...thresholds,
        spike_threshold: parseInt(element<HTMLInputElement>("spike").value, 10),
        health_warning_threshold: parseInt(element<HTMLInputElement>("warn_t").value, 10),
        health_critical_threshold: parseInt(element<HTMLInputElement>("crit_t").value, 10),
        history_days: parseInt(element<HTMLInputElement>("days").value, 10),
    };
    fetch("/api/settings", { method: "POST", headers: { "Content-Type": "application/json", ...window.tracepulseRequestHeaders?.() }, body: JSON.stringify(payload) })
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
for (const metric of metrics) {
    for (const level of ["warning", "critical"]) {
        const input = element<HTMLInputElement>(`${metric}_${level}`);
        input.addEventListener("blur", () => validateMetric(metric));
        input.addEventListener("input", () => { if (validatedMetrics.has(metric)) validateMetric(metric); });
    }
}
window.addEventListener("tracepulse-language-change", () => {
    updateBar();
    validatedMetrics.forEach(metric => validateMetric(metric));
});
document.addEventListener("DOMContentLoaded", () => initializeOnboarding());
