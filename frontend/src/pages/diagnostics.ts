function t(key: string): string { return typeof window.t === "function" ? window.t(key) : key; }

let diagnosticSocket: WebSocket | null = null;
let diagnosticLines: string[] = [];
let diagnosticResults: Array<{ type: string; data: Record<string, unknown> }> = [];
let diagnosticToastTimer: number | undefined;
let diagnosticTimeoutTimer: number | undefined;
let diagnosticLastPhase = "connecting";

function diagnosticTimeoutMs(kind: string): number {
    if (kind === "mtr") return 360_000;
    if (kind === "path_mtu") return 30_000;
    if (kind === "latency_breakdown") return 15_000;
    return 15_000;
}

function showDiagnosticToast(message: string): void {
    let toast = document.getElementById("diagnostic-copy-toast");
    if (!toast) {
        toast = document.createElement("div");
        toast.id = "diagnostic-copy-toast";
        toast.textContent = message;
        document.body.appendChild(toast);
    } else {
        toast.textContent = message;
    }
    toast.classList.add("visible");
    if (diagnosticToastTimer !== undefined) window.clearTimeout(diagnosticToastTimer);
    diagnosticToastTimer = window.setTimeout(() => toast?.classList.remove("visible"), 2000);
}

function ensureDiagnosticPanel(): HTMLElement {
    let panel = document.getElementById("active-diagnostic-panel");
    if (panel) return panel;
    panel = document.createElement("aside");
    panel.id = "active-diagnostic-panel";
    const advancedTabs = window.TRACEPULSE_ENTERPRISE_DIAGNOSTICS ? `<button data-kind="latency_breakdown">Latency</button><button data-kind="mtr">MTR &amp; MTU</button>` : "";
    panel.innerHTML = `<header><strong>Active Diagnostics</strong><button type="button" aria-label="Close" onclick="closeActiveDiagnostic()">×</button></header><nav class="diag-tabs"><button data-kind="ping">Ping</button><button data-kind="traceroute">Traceroute</button><button data-kind="port">Port Check</button>${advancedTabs}</nav><div class="diag-target" id="active-diagnostic-target"></div><div id="active-diagnostic-output"></div><footer><button type="button" onclick="copyActiveDiagnostic()">Copy</button></footer>`;
    const style = document.createElement("style");
    style.textContent = "#active-diagnostic-panel{position:fixed;z-index:1000;top:0;right:0;width:380px;max-width:100vw;height:100vh;background:#07111f;color:#dbeafe;box-shadow:-8px 0 24px #0008;transform:translateX(100%);transition:transform .2s ease;display:flex;flex-direction:column;font-family:monospace}#active-diagnostic-panel.open{transform:translateX(0)}#active-diagnostic-panel header,#active-diagnostic-panel footer{padding:1rem;border-bottom:1px solid #334155;display:flex;justify-content:space-between}.diag-tabs{display:flex;border-bottom:1px solid #334155}.diag-tabs button{flex:1;padding:.55rem .25rem;background:#0f172a;border:0;border-right:1px solid #334155;color:#94a3b8;font-size:.7rem;cursor:pointer}.diag-tabs button.active{background:#0ea5e9;color:#fff}#active-diagnostic-target{padding:.75rem 1rem;color:#38bdf8}#active-diagnostic-output{flex:1;overflow:auto;padding:1rem;color:#a7f3d0}.diag-progress-line{margin-bottom:.45rem}.diag-error-line{margin-bottom:.6rem;padding:.65rem;border-left:3px solid #ef4444;background:#3f1212;color:#fca5a5;white-space:pre-wrap}.diag-result-card{margin:.5rem 0;padding:.75rem;border:1px solid #334155;border-radius:4px;background:#0f1b2d}.diag-error-card{border-color:#ef4444}.diag-result-card h3{margin:0 0 .65rem;color:#7dd3fc;font-size:.9rem}.diag-result-row{display:flex;justify-content:space-between;gap:1rem;padding:.25rem 0;border-top:1px solid #1e293b;font-family:system-ui,sans-serif;font-size:.78rem}.diag-result-label{text-transform:capitalize;color:#94a3b8}.diag-result-row strong{color:#f8fafc;text-align:right}#active-diagnostic-output.connecting{display:flex;align-items:center;gap:.65rem;color:#94a3b8}#active-diagnostic-output.connecting::before{content:\"\";width:1rem;height:1rem;border:2px solid #475569;border-top-color:#38bdf8;border-radius:50%;animation:diagnostic-spin .8s linear infinite}@keyframes diagnostic-spin{to{transform:rotate(360deg)}}#active-diagnostic-panel footer{border-top:1px solid #334155;border-bottom:0}#active-diagnostic-panel footer button{width:100%;padding:.6rem;background:#0ea5e9;border:0;color:#fff}#diagnostic-copy-toast{position:fixed;z-index:1200;right:1.25rem;bottom:1.25rem;padding:.65rem 1rem;border:1px solid #475569;border-radius:4px;background:#0f172a;color:#f8fafc;box-shadow:0 4px 14px #0008;opacity:0;transform:translateY(8px);transition:opacity .15s ease,transform .15s ease;pointer-events:none}#diagnostic-copy-toast.visible{opacity:1;transform:translateY(0)}";
    style.textContent += ".diag-port-dialog-backdrop{position:fixed;inset:0;z-index:1300;display:grid;place-items:center;background:#0008}.diag-port-dialog{width:min(320px,calc(100vw - 2rem));padding:1rem;border:1px solid #475569;border-radius:6px;background:#0f172a;color:#f8fafc;box-shadow:0 8px 24px #000}.diag-port-dialog h3{margin:0 0 1rem}.diag-port-dialog label{display:flex;flex-direction:column;gap:.5rem;color:#94a3b8}.diag-port-dialog input{padding:.55rem;background:#020617;border:1px solid #475569;color:#fff}.diag-port-dialog div{display:flex;justify-content:flex-end;gap:1rem;margin-top:1.25rem}.diag-port-dialog button{padding:.6rem 1rem;border:0;background:#334155;color:#fff;cursor:pointer;min-width:7rem}.diag-port-dialog button[type=submit]{background:#0ea5e9}";
    style.textContent += ".diag-error-hint{color:#fecaca;line-height:1.4}.diag-result-message{display:block!important;color:#cbd5e1;line-height:1.5}.diag-result-message strong{display:block!important;margin-top:.35rem;text-align:left!important;font-weight:400}.diag-result-verdict strong{color:#4ade80;font-size:.95rem}";
    style.textContent += ".diag-result-verdict.failed strong{color:#f87171}.diag-result-verdict.partial strong{color:#fbbf24}.diag-result-card{max-width:100%;overflow-wrap:anywhere}";
    panel.querySelectorAll<HTMLButtonElement>(".diag-tabs button").forEach((button) => button.addEventListener("click", () => {
        const kind = button.dataset.kind || "ping";
        if (kind === "port" || kind === "latency_breakdown") {
            openActiveDiagnosticWithPortPrompt(currentDiagnosticTarget, kind, currentDiagnosticPort ?? 80);
        } else {
            openActiveDiagnostic(currentDiagnosticTarget, kind as "ping" | "traceroute" | "mtr", currentDiagnosticPort);
        }
    }));
    document.head.appendChild(style);
    document.body.appendChild(panel);
    return panel;
}

export function openActiveDiagnosticWithPortPrompt(target: string, kind: "port" | "latency_breakdown", initialPort = 80): void {
    const overlay = document.createElement("div");
    overlay.className = "diag-port-dialog-backdrop";
    overlay.style.cssText = "position:fixed;inset:0;z-index:1300;display:grid;place-items:center;background:rgba(0,0,0,.55);";
    overlay.innerHTML = `<form class="diag-port-dialog"><h3>${kind === "port" ? "Port Check" : "Latency Breakdown"}</h3><label>Port<input type="number" name="port" min="1" max="65535" value="${initialPort}" required></label><div><button type="button" data-cancel>Cancel</button><button type="submit">Run</button></div></form>`;
    const dialog = overlay.querySelector("form") as HTMLFormElement;
    dialog.style.cssText = "width:min(320px,calc(100vw - 2rem));padding:1rem;border:1px solid #475569;border-radius:6px;background:#0f172a;color:#f8fafc;box-shadow:0 8px 24px #000;";
    document.body.appendChild(overlay);
    const form = dialog;
    const input = form.elements.namedItem("port") as HTMLInputElement;
    input.focus();
    input.select();
    form.addEventListener("submit", (event) => {
        event.preventDefault();
        const port = Number(input.value);
        if (port >= 1 && port <= 65535) {
            overlay.remove();
            openActiveDiagnostic(target, kind, port);
        }
    });
    overlay.querySelector("[data-cancel]")?.addEventListener("click", () => overlay.remove());
}

let currentDiagnosticTarget = "";
let currentDiagnosticPort: number | undefined;

function scrollDiagnosticOutputToBottom(output: HTMLElement): void {
    window.requestAnimationFrame(() => {
        output.scrollTop = output.scrollHeight;
    });
}

function renderDiagnosticOutput(output: HTMLElement): void {
    output.replaceChildren();
    if (diagnosticLines.length) {
        const log = document.createElement("pre");
        log.style.cssText = "margin:0;white-space:pre-wrap;font:inherit";
        log.textContent = diagnosticLines.join("\n");
        output.appendChild(log);
    }
    for (const result of diagnosticResults) {
        const card = document.createElement("section");
        card.className = result.type.endsWith("result") ? "diag-result-card" : "diag-result-card diag-error-card";
        const title = document.createElement("h3");
        const titles: Record<string, string> = { ping_result: "Ping Result", traceroute_result: "Traceroute Result", port_result: "Port Check Result", latency_breakdown_result: "Latency Breakdown Result", path_mtu_result: "Path MTU Result" };
        title.textContent = titles[result.type] || result.type;
        card.appendChild(title);
        for (const [key, value] of Object.entries(result.data)) {
            if (value == null) continue;
            const row = document.createElement("div");
            const verdict = key === "verdict" || key === "status";
            const outcome = String(value).toUpperCase();
            row.className = key === "message" ? "diag-result-row diag-result-message" : verdict ? `diag-result-row diag-result-verdict${outcome === "PARTIAL" ? " partial" : ["OPEN", "REACHABLE", "RESPONSES RECEIVED", "HEALTHY"].includes(outcome) ? "" : " failed"}` : "diag-result-row";
            const label = document.createElement("span");
            label.className = "diag-result-label";
            label.textContent = key.replace(/_/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
            const valueElement = document.createElement("strong");
            valueElement.textContent = typeof value === "number" ? `${Number(value).toFixed(2)}${key.endsWith("_ms") ? " ms" : ""}` : String(value);
            row.append(label, valueElement);
            card.appendChild(row);
        }
        output.appendChild(card);
    }
}

export function openActiveDiagnostic(target: string, kind: "ping" | "traceroute" | "port" | "latency_breakdown" | "mtr" | "path_mtu", port?: number): void {
    const panel = ensureDiagnosticPanel();
    const output = document.getElementById("active-diagnostic-output");
    const targetLabel = document.getElementById("active-diagnostic-target");
    diagnosticLines = [];
    diagnosticResults = [];
    diagnosticLastPhase = "connecting";
    currentDiagnosticTarget = target;
    currentDiagnosticPort = port;
    if (output) {
        output.textContent = "Connecting...";
        output.classList.add("connecting");
    }
    const usesPort = kind === "port" || kind === "latency_breakdown";
    if (targetLabel) targetLabel.textContent = `${kind.toUpperCase()} ${target}${usesPort && port ? `:${port}` : ""}`;
    panel.querySelectorAll<HTMLButtonElement>(".diag-tabs button").forEach((button) => button.classList.toggle("active", button.dataset.kind === kind || (kind === "traceroute" && button.dataset.kind === "ping")));
    panel.classList.add("open");
    diagnosticSocket?.close();
    const protocol = location.protocol === "https:" ? "wss:" : "ws:";
    const socket = new WebSocket(`${protocol}//${location.host}/ws/diagnostics`);
    diagnosticSocket = socket;
    socket.onopen = () => {
        if (diagnosticSocket !== socket) return;
        if (output) {
            output.classList.remove("connecting");
            output.textContent = "";
        }
        socket.send(JSON.stringify({ kind, target, port, count: kind === "mtr" ? 10 : 5 }));
        if (diagnosticTimeoutTimer !== undefined) window.clearTimeout(diagnosticTimeoutTimer);
        diagnosticTimeoutTimer = window.setTimeout(() => {
            if (output) {
                output.classList.remove("connecting");
                output.textContent = `Failed - no response after 10 seconds\nTarget: ${target}${usesPort ? `:${port ?? "?"}` : ""}\nLast phase: ${diagnosticLastPhase}`;
                socket.close();
            }
        }, diagnosticTimeoutMs(kind));
    };
    socket.onmessage = (event) => {
        if (diagnosticSocket !== socket) return;
        const message = JSON.parse(event.data) as { line?: string; message_type?: string; data?: unknown; error?: string; done?: boolean };
        if (message.line) {
            diagnosticLines.push(message.line);
            diagnosticLastPhase = message.line;
        }
        if (message.message_type && message.data && typeof message.data === "object") diagnosticResults.push({ type: message.message_type, data: message.data as Record<string, unknown> });
        if (message.error) diagnosticLines.push(`Failed - ${message.error}`);
        if (output && (message.line || message.message_type || message.error)) {
            output.classList.remove("connecting");
            if ((message.done || message.message_type || message.error) && diagnosticTimeoutTimer !== undefined) window.clearTimeout(diagnosticTimeoutTimer);
            renderDiagnosticOutput(output);
            scrollDiagnosticOutputToBottom(output);
        }
    };
    socket.onerror = () => {
        if (diagnosticSocket !== socket) return;
        if (diagnosticTimeoutTimer !== undefined) window.clearTimeout(diagnosticTimeoutTimer);
        diagnosticLines.push("Failed - Enterprise license or diagnostic connection unavailable.");
        if (output) {
            output.classList.remove("connecting");
            renderDiagnosticOutput(output);
            scrollDiagnosticOutputToBottom(output);
        }
    };
    socket.onclose = () => {
        if (diagnosticSocket !== socket) return;
        if (diagnosticLines.length || diagnosticResults.length || !output || output.textContent?.startsWith("Failed")) return;
        if (diagnosticTimeoutTimer !== undefined) window.clearTimeout(diagnosticTimeoutTimer);
        diagnosticLines.push("Failed - diagnostic connection closed before a response.");
        output.classList.remove("connecting");
        renderDiagnosticOutput(output);
    };
}

export function closeActiveDiagnostic(): void {
    const socket = diagnosticSocket;
    diagnosticSocket = null;
    socket?.close();
    document.getElementById("active-diagnostic-panel")?.classList.remove("open");
}

export async function copyActiveDiagnostic(): Promise<void> {
    const report = `# TracePulse Enterprise Diagnostic Report\n\n${diagnosticLines.map((line) => `- ${line}`).join("\n")}\n\n${diagnosticResults.map((result) => `${result.type}\n${JSON.stringify(result.data, null, 2)}`).join("\n\n")}`;
    try {
        await navigator.clipboard.writeText(report);
        showDiagnosticToast("Copied");
    } catch {
        showDiagnosticToast("Copy failed");
    }
}

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
window.openActiveDiagnostic = openActiveDiagnostic;
window.openActiveDiagnosticWithPortPrompt = openActiveDiagnosticWithPortPrompt;
window.closeActiveDiagnostic = closeActiveDiagnostic;
window.copyActiveDiagnostic = copyActiveDiagnostic;
