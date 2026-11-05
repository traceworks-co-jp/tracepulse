import { credentialFields, clearCredentials, readCredentials, type SnmpCredentials } from "../shared/snmp-credentials";
import { esc } from "../shared/escape";
import { addSecretVisibility } from "../shared/secret-visibility";
import { deviceStatusClass } from "../shared/status";
import { showOperationProgress } from "../shared/operation-progress";

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

type RegistrationError = "registration_failed" | "registration_network_error" | "registration_denied" | "registration_limit";
interface RegistrationRequest extends DiscoveryDevice { snmp_credentials?: SnmpCredentials; }
interface ResultRow { device: DiscoveryDevice; row: HTMLTableRowElement; checkbox: HTMLInputElement; cell: HTMLTableCellElement; community: string; error?: RegistrationError; }
interface RegistrationOutcome { registered: string[]; skipped: string[]; failures: Map<string, RegistrationError>; }
let resultRows = new Map<string, ResultRow>();
const registeringIps = new Set<string>();
let registrationBusy = false;
let resultGeneration = 0;
let registrationSummary: { registered: number; skipped: number; failed: number } | null = null;

const scanTimeoutSeconds = 300;
const pollIntervalMs = 1500;
let currentJobId: string | null = null;
let pollTimer: number | null = null;
let elapsedTimer: number | null = null;
let scanStartedAt: number | null = null;
let cancelled = false;
let scanCommunity = "";
let scanGeneration = 0;

function t(key: string): string { return typeof window.t === "function" ? window.t(key) : key; }
function el<T extends HTMLElement>(id: string): T { return document.getElementById(id) as T; }
function maxDiscoveryCidrs(): number | null { return window.TRACEPULSE_DISCOVERY_LIMITS?.maxCidrs ?? null; }
function existingIps(): Set<string> { return window.DISCOVERY_EXISTING ??= new Set<string>(); }

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
    const maxCidrs = maxDiscoveryCidrs();
    if (maxCidrs !== null && (entries.length > maxCidrs || smallestPrefix < 24)) {
        hint.textContent = `This deployment accepts up to ${maxCidrs} CIDR range(s) at /24 or narrower. Current: /${smallestPrefix}`;
        hint.className = "host-hint warn";
        return;
    }
    const seconds = Math.ceil(total * 0.5 / 256);
    hint.textContent = `${total.toLocaleString()} hosts • est. ~${seconds >= 60 ? `${Math.ceil(seconds / 60)} min` : `${seconds}s`}`;
    hint.className = "host-hint";
}

function applyPageLanguage(_lang?: string): void {
    window.refreshTopologyTexts?.();
    updateRegistrationRows();
    showRegistrationSummary();
}

export async function startScan(): Promise<void> {
    const cidr = el<HTMLInputElement>("cidr").value.trim();
    const community = el<HTMLInputElement>("community").value.trim() || "public";
    const seedIp = (document.getElementById("seed-ip") as HTMLInputElement | null)?.value.trim() || "";
    if (!cidr) { showScanError("Please enter a CIDR range."); return; }
    const entries = cidrEntries(cidr);
    const prefixes = entries.map((entry) => parseInt(entry.split("/")[1] || "33", 10));
    const smallestPrefix = Math.min(...prefixes);
    const maxCidrs = maxDiscoveryCidrs();
    if (maxCidrs !== null && (entries.length > maxCidrs || smallestPrefix < 24)) {
        showScanError(`This deployment accepts up to ${maxCidrs} CIDR range(s) at /24 or narrower.`);
        return;
    }
    const total = entries.map(cidrHostCount).reduce((sum: number, count) => sum + (count || 0), 0);
    const generation = ++scanGeneration;
    scanCommunity = community;
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
        const response = await fetch("/api/discovery/scan", { method: "POST", headers: { "Content-Type": "application/x-www-form-urlencoded", "X-CSRF-Token": window.tracepulseCsrf || "" }, body: `cidr=${encodeURIComponent(cidr)}&community=${encodeURIComponent(community)}&max_hosts=${encodeURIComponent(total || 65534)}` });
        const data = await response.json() as { error?: string; job_id?: string; total?: number };
        if (generation !== scanGeneration) return;
        if (data.error || !data.job_id) { finishScanError(data.error || "Failed to start scan"); return; }
        currentJobId = data.job_id;
        scanStartedAt = Date.now();
        if (seedIp && typeof window.startTopologyDiscovery === "function") window.startTopologyDiscovery(seedIp, community);
        startPolling(data.total || total || 0, generation);
    } catch (error: unknown) { if (generation === scanGeneration) finishScanError(`Failed to start scan: ${String(error)}`); }
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

function startPolling(total: number, generation: number): void {
    elapsedTimer = window.setInterval(() => {
        if (!scanStartedAt) return;
        const elapsed = Math.floor((Date.now() - scanStartedAt) / 1000);
        el<HTMLElement>("progress-elapsed").textContent = `Elapsed: ${elapsed}s`;
        if (elapsed >= scanTimeoutSeconds) { cancelScan(); finishScanError(`Scan timed out after ${elapsed}s.`); }
    }, 1000);
    pollTimer = window.setInterval(async () => {
        if (!currentJobId || cancelled) return;
        const job = currentJobId;
        try {
            const response = await fetch(`/api/discovery/scan/${job}`);
            const data = await response.json() as ScanStatus;
            if (cancelled || generation !== scanGeneration || currentJobId !== job) return;
            if (data.error) { finishScanError(data.error); return; }
            setProgress(data.scanned || 0, data.total || total, data.elapsed || 0);
            if (data.status === "done") finishScanDone(data.devices || [], data.scanned || 0, data.total || total, data.elapsed || 0);
            else if (data.status === "error") finishScanError(data.error || "Scan failed");
        } catch (error: unknown) { if (!cancelled && generation === scanGeneration) finishScanError(`Polling error: ${String(error)}`); }
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
    scanGeneration++;
    cancelled = true; currentJobId = null; stopPolling();
    el<HTMLElement>("scan-btn").style.display = "";
    el<HTMLElement>("cancel-btn").style.display = "none";
    el<HTMLElement>("scan-progress").style.display = "none";
}

function finishScanError(message: string): void { stopPolling(); el<HTMLElement>("scan-btn").style.display = ""; el<HTMLElement>("cancel-btn").style.display = "none"; el<HTMLElement>("scan-progress").style.display = "none"; showScanError(message); }
function finishScanDone(devices: DiscoveryDevice[], scanned: number, total: number, elapsed: number): void { stopPolling(); currentJobId = null; el<HTMLElement>("scan-btn").style.display = ""; el<HTMLElement>("cancel-btn").style.display = "none"; el<HTMLElement>("scan-progress").style.display = "none"; renderResults(devices, scanned, total, elapsed); }
function showScanError(message: string): void { const error = el<HTMLElement>("scan-error"); error.textContent = `! ${message}`; error.style.display = "block"; }

function renderResults(devices: DiscoveryDevice[], scanned: number, total: number, elapsed: number): void {
    resultGeneration++;
    resultRows = new Map();
    registrationSummary = null;
    el<HTMLElement>("register-result").style.display = "none";
    const tbody = el<HTMLTableSectionElement>("results-body");
    tbody.innerHTML = "";
    el<HTMLElement>("results-title").textContent = `${t("scan_results_label")} - ${devices.length} ${t("found")} (scanned ${scanned}/${total}, ${elapsed}s)`;
    if (devices.length === 0) tbody.innerHTML = `<tr><td colspan='5' class='empty'>${t("no_snmp_devices_found")}</td></tr>`;
    devices.forEach((device) => {
        const row = document.createElement("tr");
        row.innerHTML = `<td><input type='checkbox' class='row-cb' aria-label="${esc(`${t("select_device")} ${device.ip}`)}"></td><td>${esc(device.ip)}</td><td>${esc(device.name || "")}</td><td><span class='${deviceStatusClass(device.status)}'>${esc(device.status || "unknown")}</span></td><td class='registration-cell'></td>`;
        const checkbox = row.querySelector<HTMLInputElement>("input")!;
        checkbox.dataset.ip = device.ip;
        checkbox.onchange = updateRegisterBtn;
        resultRows.set(device.ip, { device, row, checkbox, cell: row.cells[4], community: scanCommunity });
        tbody.appendChild(row);
    });
    el<HTMLElement>("results-section").style.display = "block";
    (el<HTMLInputElement>("select-all")).checked = false;
    updateRegistrationRows();
}

export function updateRegisterBtn(): void {
    const selectable = [...resultRows.values()].filter(state => !existingIps().has(state.device.ip));
    const count = selectable.filter(state => state.checkbox.checked).length;
    el<HTMLElement>("register-action").style.display = count > 0 ? "flex" : "none";
    el<HTMLElement>("select-count").textContent = count ? `${count} ${t("selected")}` : "";
    const button = el<HTMLButtonElement>("register-btn");
    button.disabled = registrationBusy || count === 0;
    button.textContent = registrationBusy ? t("registering") : `${t("register")} (${count})`;
    const master = el<HTMLInputElement>("select-all");
    master.disabled = registrationBusy || selectable.length === 0;
    master.checked = selectable.length > 0 && count === selectable.length;
    master.indeterminate = count > 0 && count < selectable.length;
}
export function toggleAll(master: HTMLInputElement): void { document.querySelectorAll<HTMLInputElement>(".row-cb:not(:disabled)").forEach((checkbox) => { checkbox.checked = master.checked; }); updateRegisterBtn(); }

function updateRegistrationRows(): void {
    document.querySelector(".discovery-results-scroll")?.setAttribute("aria-label", t("scan_results_label"));
    for (const state of resultRows.values()) {
        const ip = state.device.ip;
        const registered = existingIps().has(ip);
        state.checkbox.disabled = registered || registrationBusy;
        state.checkbox.setAttribute("aria-label", `${t("select_device")} ${ip}`);
        if (registered) { state.checkbox.checked = false; state.error = undefined; }
        state.cell.replaceChildren();
        if (registered) {
            const badge = document.createElement("span"); badge.className = "badge-registered"; badge.textContent = t("registered"); state.cell.append(badge);
        } else if (registeringIps.has(ip)) {
            const text = document.createElement("span"); text.setAttribute("role", "status"); text.textContent = t("registering");
            const progress = document.createElement("progress"); progress.className = "registration-row-progress"; progress.setAttribute("aria-label", t("registering"));
            state.cell.append(text, progress);
        } else {
            const button = document.createElement("button"); button.type = "button"; button.className = "btn btn-secondary btn-xs register-device"; button.textContent = t("register"); button.disabled = registrationBusy;
            button.onclick = () => {
                if (resultRows.get(ip) !== state) return;
                void registerDevices([{ ip, name: state.device.name, community: state.community }]);
            };
            state.cell.append(button);
            if (state.error) {
                const error = document.createElement("span"); error.className = "registration-error"; error.setAttribute("role", "alert"); error.textContent = t(state.error); state.cell.append(error);
            }
        }
    }
    updateRegisterBtn();
    document.querySelectorAll<HTMLButtonElement>("#manual-box button").forEach(button => { button.disabled = registrationBusy; });
}

function showRegistrationSummary(): void {
    if (!registrationSummary) return;
    const { registered, skipped, failed } = registrationSummary;
    const result = el<HTMLElement>("register-result");
    result.className = failed ? "reg-error" : "reg-success";
    result.textContent = t("registration_summary").replace("{registered}", String(registered)).replace("{skipped}", String(skipped)).replace("{failed}", String(failed));
    result.style.display = "block";
}

function registrationError(value: unknown): RegistrationError {
    return typeof value === "string" && /device registration limit reached|node limit|registration limit/i.test(value) ? "registration_limit" : "registration_failed";
}

async function requestRegistration(targets: RegistrationRequest[]): Promise<RegistrationOutcome> {
    const failed = (key: RegistrationError): RegistrationOutcome => ({ registered: [], skipped: [], failures: new Map(targets.map(device => [device.ip, key])) });
    try {
        const response = await fetch("/api/discovery/register", { method: "POST", headers: { "Content-Type": "application/json", "X-CSRF-Token": window.tracepulseCsrf || "" }, body: JSON.stringify(targets) });
        if (response.status === 401 || response.status === 403) return failed("registration_denied");
        const data = await response.json() as Partial<RegisterResult> | null;
        if (!response.ok || !data || data.error) return failed(registrationError(data?.error));
        const strings = (value: unknown): value is string[] => Array.isArray(value) && value.every(item => typeof item === "string");
        if (!strings(data.registered) || !strings(data.skipped) || !strings(data.errors)) return failed("registration_failed");
        const outcome: RegistrationOutcome = { registered: [], skipped: [], failures: new Map() };
        for (const target of targets) {
            const error = data.errors.find(message => message.startsWith(`${target.ip}:`));
            if (error) outcome.failures.set(target.ip, registrationError(error));
            else if (data.registered.includes(target.ip)) outcome.registered.push(target.ip);
            else if (data.skipped.includes(target.ip)) outcome.skipped.push(target.ip);
            else outcome.failures.set(target.ip, "registration_failed");
        }
        return outcome;
    } catch { return failed("registration_network_error"); }
}

async function registerDevices(targets: RegistrationRequest[], progressAnchor = el<HTMLElement>("register-result")): Promise<RegistrationOutcome | null> {
    if (registrationBusy || targets.some(target => registeringIps.has(target.ip))) return null;
    targets = targets.filter(target => !existingIps().has(target.ip));
    if (!targets.length) { updateRegistrationRows(); return null; }
    registrationBusy = true;
    const generation = resultGeneration;
    const rows = new Map(resultRows);
    for (const target of targets) { registeringIps.add(target.ip); const row = rows.get(target.ip); if (row) row.error = undefined; }
    updateRegistrationRows();
    const container = document.createElement("div");
    progressAnchor.before(container);
    const stop = showOperationProgress(container, t("registering"), seconds => t("registration_progress").replace("{count}", String(targets.length)).replace("{seconds}", String(seconds)));
    if (targets.length > 1) container.scrollIntoView({ block: "nearest" });
    try {
        const outcome = await requestRegistration(targets);
        for (const ip of [...outcome.registered, ...outcome.skipped]) existingIps().add(ip);
        if (generation === resultGeneration) {
            for (const [ip, error] of outcome.failures) { const state = rows.get(ip); if (state && resultRows.get(ip) === state) state.error = error; }
            registrationSummary = { registered: outcome.registered.length, skipped: outcome.skipped.length, failed: outcome.failures.size };
            showRegistrationSummary();
        }
        return outcome;
    } finally {
        stop(); container.remove(); registrationBusy = false;
        for (const target of targets) registeringIps.delete(target.ip);
        updateRegistrationRows();
    }
}

export async function registerSelected(): Promise<void> {
    const selected = [...resultRows.values()].filter(state => state.checkbox.checked && !existingIps().has(state.device.ip))
        .map(state => ({ ip: state.device.ip, name: state.device.name, community: state.community }));
    if (!selected.length) { window.alert(t("select_at_least_one_device")); return; }
    await registerDevices(selected);
}

export function showManual(): void { el<HTMLElement>("manual-box").style.display = "block"; }
export function hideManual(): void { el<HTMLElement>("manual-box").style.display = "none"; el<HTMLElement>("manual-result").textContent = ""; }
export async function addManual(): Promise<void> {
    const ip = el<HTMLInputElement>("manual-ip").value.trim();
    const name = el<HTMLInputElement>("manual-name").value.trim() || `device-${ip.split(".").pop()}`;
    const community = el<HTMLInputElement>("manual-community").value;
    const snmp_credentials = readCredentials(el<HTMLElement>("manual-box"));
    const result = el<HTMLElement>("manual-result");
    if (!ip) { result.textContent = t("registration_ip_required"); return; }
    if (existingIps().has(ip)) { result.textContent = `${ip} ${t("registered")}`; updateRegistrationRows(); return; }
    const outcome = await registerDevices([{ ip, name, community, snmp_credentials }], result);
    if (!outcome) return;
    const error = outcome.failures.get(ip);
    result.textContent = error ? t(error) : `${ip} ${outcome.registered.length ? t("registration_manual_success") : t("registered")}`;
    if (!error) { el<HTMLInputElement>("manual-ip").value = ""; el<HTMLInputElement>("manual-name").value = ""; el<HTMLInputElement>("manual-community").value = ""; clearCredentials(el<HTMLElement>("manual-box")); }
}

window.addManual = addManual; window.cancelScan = cancelScan; window.hideManual = hideManual; window.registerSelected = registerSelected; window.runTopologyOnly = runTopologyOnly; window.showManual = showManual; window.startScan = startScan; window.toggleAll = toggleAll; window.updateHint = updateHint; window.updateRegisterBtn = updateRegisterBtn;
window.applyPageLanguage = applyPageLanguage;
const manualCommunity = document.getElementById("manual-community");
manualCommunity?.parentElement?.insertAdjacentHTML("afterend", credentialFields("manual-snmp", false, false));
for (const id of ["community", "manual-community"]) {
    const input = document.getElementById(id);
    if (input instanceof HTMLInputElement) addSecretVisibility(input);
}
