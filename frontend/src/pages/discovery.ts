interface DiscoveryDevice {
    ip: string;
    name?: string;
    community?: string;
    status?: string;
}

interface ScanStatus {
    status?: string;
    error?: string;
    scanned?: number;
    total?: number;
    elapsed?: number;
    devices?: DiscoveryDevice[];
}

interface RegisterResult {
    error?: string;
    registered: string[];
    skipped: string[];
    errors: string[];
}

const scanTimeoutSeconds = 300;
const pollIntervalMs = 1500;
let currentJobId: string | null = null;
let pollTimer: number | null = null;
let elapsedTimer: number | null = null;
let scanStartedAt: number | null = null;
let cancelled = false;

function t(key: string): string { return typeof window.t === "function" ? window.t(key) : key; }
function el<T extends HTMLElement>(id: string): T { return document.getElementById(id) as T; }
function isEnterprise(): boolean { return !!window.TRACEPULSE_WEB_EDITION?.enterprise; }
function existingIps(): Set<string> { return window.DISCOVERY_EXISTING || new Set<string>(); }

function cidrHostCount(cidr: string): number | null {
    const match = cidr.match(/^\d+\.\d+\.\d+\.\d+\/(\d+)$/);
    if (!match) return null;
    const prefix = parseInt(match[1], 10);
    if (prefix < 0 || prefix > 32) return null;
    if (prefix === 32) return 1;
    if (prefix === 31) return 2;
    return Math.pow(2, 32 - prefix) - 2;
}

function cidrEntries(value: string): string[] { return value.split(",").map((entry) => entry.trim()).filter(Boolean); }

export function updateHint(): void {
    const hint = el<HTMLElement>("host-hint");
    const entries = cidrEntries(el<HTMLInputElement>("cidr").value.trim());
    const counts = entries.map(cidrHostCount);
    if (entries.length === 0 || counts.some((count) => count === null)) { hint.innerHTML = ""; return; }
    const total = counts.reduce((sum: number, count) => sum + (count || 0), 0);
    if (!total) { hint.innerHTML = ""; return; }
    const smallestPrefix = Math.min(...entries.map((entry) => parseInt(entry.split("/")[1], 10)));
    const maxPrefix = isEnterprise() ? 0 : 24;
    if (!isEnterprise() && smallestPrefix < maxPrefix) {
        hint.textContent = `Community版では単一サブネット（/${maxPrefix}以上）のみスキャン可能です。現在: /${smallestPrefix}`;
        hint.className = "host-hint warn";
        return;
    }
    const seconds = Math.ceil(total * 0.5 / 256);
    hint.textContent = `${total.toLocaleString()} hosts • est. ~${seconds >= 60 ? `${Math.ceil(seconds / 60)} min` : `${seconds}s`}`;
    hint.className = "host-hint";
}

function applyPageLanguage(_lang?: string): void {
    const cidr = document.getElementById("cidr") as HTMLInputElement | null;
    if (cidr && isEnterprise()) cidr.placeholder = t("cidr_range_placeholder_enterprise");
    window.refreshTopologyTexts?.();
}

export async function startScan(): Promise<void> {
    const cidr = el<HTMLInputElement>("cidr").value.trim();
    const community = el<HTMLInputElement>("community").value.trim() || "public";
    const seedIp = (document.getElementById("seed-ip") as HTMLInputElement | null)?.value.trim() || "";
    if (!cidr) { showScanError("Please enter a CIDR range."); return; }
    const entries = cidrEntries(cidr);
    const prefixes = entries.map((entry) => parseInt(entry.split("/")[1] || "33", 10));
    const smallestPrefix = Math.min(...prefixes);
    if (!isEnterprise() && (entries.length > 1 || smallestPrefix < 24)) {
        showScanError("Community版では単一サブネット（/24以上）のみスキャン可能です。");
        return;
    }
    const total = entries.map(cidrHostCount).reduce((sum: number, count) => sum + (count || 0), 0);
    cancelled = false;
    currentJobId = null;
    stopPolling();
    el<HTMLElement>("scan-btn").style.display = "none";
    el<HTMLElement>("cancel-btn").style.display = "";
    el<HTMLElement>("scan-error").style.display = "none";
    el<HTMLElement>("results-section").style.display = "none";
    el<HTMLElement>("topology-section").style.display = "none";
    setProgress(0, 0, 0);
    el<HTMLElement>("scan-progress").style.display = "block";
    try {
        const response = await fetch("/api/discovery/scan", { method: "POST", headers: { "Content-Type": "application/x-www-form-urlencoded" }, body: `cidr=${encodeURIComponent(cidr)}&community=${encodeURIComponent(community)}&max_hosts=${encodeURIComponent(total || 65534)}` });
        const data = await response.json() as { error?: string; job_id?: string; total?: number };
        if (data.error || !data.job_id) { finishScanError(data.error || "Failed to start scan"); return; }
        currentJobId = data.job_id;
        scanStartedAt = Date.now();
        if (seedIp && typeof window.startTopologyDiscovery === "function") window.startTopologyDiscovery(seedIp, community);
        startPolling(data.total || total || 0);
    } catch (error: unknown) { finishScanError(`Failed to start scan: ${String(error)}`); }
}

export async function runTopologyOnly(): Promise<void> {
    const seedIp = el<HTMLInputElement>("seed-ip").value.trim();
    const community = el<HTMLInputElement>("community").value.trim() || "public";
    if (!seedIp) { showScanError(t("topology_seed_required")); return; }
    if (typeof window.startTopologyDiscovery !== "function") {
        showScanError("Topology discovery is unavailable.");
        return;
    }
    el<HTMLElement>("scan-error").style.display = "none";
    await window.startTopologyDiscovery(seedIp, community);
}

function startPolling(total: number): void {
    elapsedTimer = window.setInterval(() => {
        if (!scanStartedAt) return;
        const elapsed = Math.floor((Date.now() - scanStartedAt) / 1000);
        el<HTMLElement>("progress-elapsed").textContent = `Elapsed: ${elapsed}s`;
        if (elapsed >= scanTimeoutSeconds) { cancelScan(); finishScanError(`Scan timed out after ${elapsed}s.`); }
    }, 1000);
    pollTimer = window.setInterval(async () => {
        if (!currentJobId || cancelled) return;
        try {
            const response = await fetch(`/api/discovery/scan/${currentJobId}`);
            const data = await response.json() as ScanStatus;
            if (cancelled) return;
            if (data.error) { finishScanError(data.error); return; }
            setProgress(data.scanned || 0, data.total || total, data.elapsed || 0);
            if (data.status === "done") finishScanDone(data.devices || [], data.scanned || 0, data.total || total, data.elapsed || 0);
            else if (data.status === "error") finishScanError(data.error || "Scan failed");
        } catch (error: unknown) { finishScanError(`Polling error: ${String(error)}`); }
    }, pollIntervalMs);
}

function setProgress(scanned: number, total: number, elapsed: number): void {
    const percent = total > 0 ? Math.round(scanned / total * 100) : 0;
    el<HTMLElement>("progress-bar").style.width = `${percent}%`;
    el<HTMLElement>("progress-stats").textContent = `${scanned} / ${total || "?"} (${percent}%)`;
    el<HTMLElement>("progress-elapsed").textContent = `${t("elapsed")}: ${elapsed}s`;
    el<HTMLElement>("progress-label").textContent = t("scanning");
}

function stopPolling(): void {
    if (pollTimer !== null) window.clearInterval(pollTimer);
    if (elapsedTimer !== null) window.clearInterval(elapsedTimer);
    pollTimer = null; elapsedTimer = null;
}

export function cancelScan(): void {
    cancelled = true; currentJobId = null; stopPolling();
    el<HTMLElement>("scan-btn").style.display = "";
    el<HTMLElement>("cancel-btn").style.display = "none";
    el<HTMLElement>("scan-progress").style.display = "none";
}

function finishScanError(message: string): void { stopPolling(); el<HTMLElement>("scan-btn").style.display = ""; el<HTMLElement>("cancel-btn").style.display = "none"; el<HTMLElement>("scan-progress").style.display = "none"; showScanError(message); }
function finishScanDone(devices: DiscoveryDevice[], scanned: number, total: number, elapsed: number): void { stopPolling(); currentJobId = null; el<HTMLElement>("scan-btn").style.display = ""; el<HTMLElement>("cancel-btn").style.display = "none"; el<HTMLElement>("scan-progress").style.display = "none"; renderResults(devices, scanned, total, elapsed); }
function showScanError(message: string): void { const error = el<HTMLElement>("scan-error"); error.textContent = `! ${message}`; error.style.display = "block"; }

function renderResults(devices: DiscoveryDevice[], scanned: number, total: number, elapsed: number): void {
    const tbody = el<HTMLTableSectionElement>("results-body");
    tbody.innerHTML = "";
    el<HTMLElement>("results-title").textContent = `${t("scan_results_label")} - ${devices.length} ${t("found")} (scanned ${scanned}/${total}, ${elapsed}s)`;
    if (devices.length === 0) tbody.innerHTML = `<tr><td colspan='5' class='empty'>${t("no_snmp_devices_found")}</td></tr>`;
    devices.forEach((device) => {
        const registered = existingIps().has(device.ip);
        const row = document.createElement("tr");
        row.innerHTML = `<td><input type='checkbox' class='row-cb' data-ip='${device.ip}' data-name='${device.name || ""}' data-community='${device.community || "public"}'${registered ? " disabled" : " onchange='updateRegisterBtn()'"}></td><td>${device.ip}</td><td>${device.name || ""}</td><td><span class='${device.status === "online" ? "status-online" : "status-unknown"}'>${device.status || "unknown"}</span></td><td>${registered ? "registered" : ""}</td>`;
        tbody.appendChild(row);
    });
    el<HTMLElement>("results-section").style.display = "block";
    (el<HTMLInputElement>("select-all")).checked = false;
    updateRegisterBtn();
}

export function updateRegisterBtn(): void { const count = document.querySelectorAll(".row-cb:checked").length; el<HTMLElement>("register-action").style.display = count > 0 ? "flex" : "none"; el<HTMLElement>("select-count").textContent = count ? `${count} ${t("selected")}` : ""; if (count) el<HTMLButtonElement>("register-btn").textContent = `+ ${t("register")} ${count} device${count > 1 ? "s" : ""}`; }
export function toggleAll(master: HTMLInputElement): void { document.querySelectorAll<HTMLInputElement>(".row-cb:not(:disabled)").forEach((checkbox) => { checkbox.checked = master.checked; }); updateRegisterBtn(); }

export async function registerSelected(): Promise<void> {
    const selected = Array.from(document.querySelectorAll<HTMLInputElement>(".row-cb:checked")).map((checkbox) => ({ ip: checkbox.dataset.ip, name: checkbox.dataset.name, community: checkbox.dataset.community }));
    if (!selected.length) { window.alert(t("select_at_least_one_device")); return; }
    const button = el<HTMLButtonElement>("register-btn"); button.disabled = true; button.textContent = t("registering");
    try {
        const response = await fetch("/api/discovery/register", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(selected) });
        const data = await response.json() as RegisterResult;
        const result = el<HTMLElement>("register-result");
        result.className = data.error || data.registered.length === 0 ? "reg-error" : "reg-success";
        result.textContent = data.error || `Registered: ${data.registered.join(", ")}. Skipped: ${data.skipped.join(", ")}.`;
        result.style.display = "block";
        data.registered.forEach((ip) => existingIps().add(ip));
        updateRegisterBtn();
    } catch (error: unknown) { el<HTMLElement>("register-result").textContent = String(error); }
    finally { button.disabled = false; updateRegisterBtn(); }
}

export function showManual(): void { el<HTMLElement>("manual-box").style.display = "block"; }
export function hideManual(): void { el<HTMLElement>("manual-box").style.display = "none"; el<HTMLElement>("manual-result").textContent = ""; }
export async function addManual(): Promise<void> {
    const ip = el<HTMLInputElement>("manual-ip").value.trim();
    const name = el<HTMLInputElement>("manual-name").value.trim() || `device-${ip.split(".").pop()}`;
    const community = el<HTMLInputElement>("manual-community").value.trim() || "public";
    const result = el<HTMLElement>("manual-result");
    if (!ip) { result.textContent = "! IP address is required."; return; }
    try {
        const response = await fetch("/api/discovery/register", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify([{ ip, name, community }]) });
        const data = await response.json() as RegisterResult;
        result.textContent = data.error || data.skipped.length ? `${ip} is already registered.` : `${ip} registered successfully.`;
        if (data.registered.length) { existingIps().add(ip); el<HTMLInputElement>("manual-ip").value = ""; el<HTMLInputElement>("manual-name").value = ""; }
    } catch (error: unknown) { result.textContent = String(error); }
}

window.addManual = addManual; window.cancelScan = cancelScan; window.hideManual = hideManual; window.registerSelected = registerSelected; window.runTopologyOnly = runTopologyOnly; window.showManual = showManual; window.startScan = startScan; window.toggleAll = toggleAll; window.updateHint = updateHint; window.updateRegisterBtn = updateRegisterBtn;
window.applyPageLanguage = applyPageLanguage;
