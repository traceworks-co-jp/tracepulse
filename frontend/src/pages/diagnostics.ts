function t(key: string): string { return typeof window.t === "function" ? window.t(key) : key; }

export function startDiagnostics(event: SubmitEvent): boolean {
    event.preventDefault();
    const form = event.target as HTMLFormElement;
    const button = document.getElementById("diag-run-btn") as HTMLButtonElement | null;
    const progress = document.getElementById("diag-progress");
    if (button) button.disabled = true;
    if (progress) progress.textContent = t("diagnostics_running");
    const url = `${form.action}?${new URLSearchParams(new FormData(form) as any).toString()}`;
    fetch(url, { cache: "no-store" })
        .then((response) => response.text())
        .then((html) => { document.open(); document.write(html); document.close(); })
        .catch((error: unknown) => {
            if (progress) progress.textContent = String(error);
            if (button) button.disabled = false;
        });
    return false;
}

function collectHardwareOids(): string[] {
    return Array.from(document.querySelectorAll<HTMLInputElement>(".hardware-oid-input"))
        .map((input) => {
            const oid = input.value.trim();
            const type = input.getAttribute("data-sensor-type") || "temperature";
            return oid ? `${type}|${oid}` : "";
        })
        .filter(Boolean);
}

export function saveOidOverrides(): void {
    const payload = {
        cpu_oid_override: (document.getElementById("cpu-oid-override") as HTMLInputElement)?.value || "",
        memory_oid_override: (document.getElementById("memory-oid-override") as HTMLInputElement)?.value || "",
        hardware_oid_overrides: collectHardwareOids(),
    };
    fetch("/api/diagnostics/oids", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload) })
        .then((response) => response.json())
        .then((data: { ok?: boolean; error?: string }) => {
            const element = document.getElementById("oid-save-result");
            if (!element) return;
            if (data.ok) {
                element.className = "diag-note";
                element.textContent = t("settings_saved");
                window.setTimeout(() => window.location.reload(), 300);
            } else {
                element.textContent = data.error || "Save failed";
                element.className = "diag-note diag-error";
            }
        })
        .catch((error: unknown) => {
            const element = document.getElementById("oid-save-result");
            if (element) { element.textContent = String(error); element.className = "diag-note diag-error"; }
        });
}

window.saveOidOverrides = saveOidOverrides;
window.startDiagnostics = startDiagnostics;
