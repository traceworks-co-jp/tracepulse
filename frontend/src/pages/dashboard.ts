import { esc as escapeHtml } from "../shared/escape";
import { fmtTime, formatTracePulseClock } from "../shared/format";
import { initializeOnboarding } from "../shared/onboarding";
import { bulkActions, deviceLabel, metadataText } from "../shared/device-metadata";
import { initializeDeviceFilters } from "../shared/device-filters";
import { deviceStatusClass as statusClass } from "../shared/status";

interface DashboardDevice {
    id?: number | null;
    display_name?: string | null;
    tags?: string[];
    ip: string;
    name?: string;
    status?: string;
    last_seen?: string;
    last_seen_at?: string;
    error_spike?: boolean;
    health_score?: number | null;
    health_status?: string | null;
}

const columns = ["ip", "display_name", "name", "tags", "status", "health_score", "last_seen"] as const;
let sortColumn: (typeof columns)[number] = "ip";
let sortAscending = true;
const deletingIps = new Set<string>();
const selectedIds = new Set<number>();
let searchQuery = "";
let matchesFilters: (device: DashboardDevice) => boolean = () => true;

function visibleDevices(): DashboardDevice[] {
    const query = searchQuery.trim().toLocaleLowerCase();
    return currentDevices().filter((device) => matchesFilters(device) && (!query ||
        [device.ip, device.name, device.display_name, ...(device.tags || [])]
            .some((value) => value?.toLocaleLowerCase().includes(query))));
}

function renderSelection(): void {
    const count = document.getElementById("device-selection-count");
    if (count) count.textContent = `${selectedIds.size} ${metadataText("selected")}`;
    const button = document.getElementById("bulk-rename") as HTMLButtonElement | null;
    if (button) { button.textContent = metadataText("bulk"); button.disabled = selectedIds.size === 0; }
    const all = document.getElementById("select-visible-devices") as HTMLInputElement | null;
    const visible = visibleDevices().filter((device) => typeof device.id === "number");
    if (all) {
        all.checked = visible.length > 0 && visible.every((device) => selectedIds.has(device.id!));
        all.indeterminate = !all.checked && visible.some((device) => selectedIds.has(device.id!));
        all.setAttribute("aria-label", metadataText("all"));
    }
    const search = document.getElementById("device-search") as HTMLInputElement | null;
    if (search) { search.placeholder = metadataText("search"); search.setAttribute("aria-label", metadataText("search")); }
}

function t(key: string): string {
    return typeof window.t === "function" ? window.t(key) : key;
}

function ipToNumber(ip: string): number {
    const parts = ip.split(".");
    if (parts.length !== 4) return 0;
    return parts.reduce((value, part) => value * 256 + (parseInt(part, 10) || 0), 0);
}

function statusOrder(status?: string): number {
    const order: Record<string, number> = { critical: 0, offline: 1, warning: 2, unknown: 3, online: 4 };
    return order[status || ""] ?? 5;
}


function compareValue(a: DashboardDevice, b: DashboardDevice, column: (typeof columns)[number]): number {
    if (column === "ip") return ipToNumber(a.ip) - ipToNumber(b.ip);
    if (column === "status") return statusOrder(a.status) - statusOrder(b.status);
    if (column === "health_score") return (a.health_score ?? -1) - (b.health_score ?? -1);
    const left = String(column === "display_name" ? deviceLabel(a) : column === "tags" ? (a.tags || []).join(", ") : a[column] || "").toLowerCase();
    const right = String(column === "display_name" ? deviceLabel(b) : column === "tags" ? (b.tags || []).join(", ") : b[column] || "").toLowerCase();
    return left < right ? -1 : left > right ? 1 : 0;
}

function currentDevices(): DashboardDevice[] {
    return window.DEVICES || [];
}

function renderAttention(devices: DashboardDevice[] = currentDevices()): void {
    const element = document.getElementById("device-attention");
    if (!element) return;
    const attention = devices.filter((device) => device.error_spike || device.status === "offline" || device.status === "critical" || device.status === "warning");
    element.classList.toggle("has-items", attention.length > 0);
    if (attention.length === 0) {
        element.innerHTML = "";
        return;
    }
    const items = attention.map((device) => {
        const reasons: string[] = [];
        if (device.error_spike) reasons.push(t("error_spikes"));
        if (device.status === "offline") reasons.push(t("offline"));
        else if (device.status === "critical") reasons.push("Critical");
        else if (device.status === "warning") reasons.push(t("warning"));
        return `<li class='attention-item'><a href='/device/${encodeURIComponent(device.ip)}'>${escapeHtml(deviceLabel(device))}</a><span class='attention-reason'>${escapeHtml(reasons.join(" / "))}</span></li>`;
    }).join("");
    element.innerHTML = `<h2>${t("attention_devices")}</h2><ul class='attention-list'>${items}</ul>`;
}

export function renderTable(): void {
    const tbody = document.getElementById("dash-tbody");
    if (!tbody) return;
    const validIds = new Set(currentDevices().map((device) => device.id));
    selectedIds.forEach((id) => { if (!validIds.has(id)) selectedIds.delete(id); });
    renderSelection();
    const sorted = visibleDevices().slice().sort((a, b) => {
        if (a.error_spike && !b.error_spike) return -1;
        if (!a.error_spike && b.error_spike) return 1;
        const result = compareValue(a, b, sortColumn);
        return sortAscending ? result : -result;
    });

    if (sorted.length === 0) {
        tbody.innerHTML = currentDevices().length > 0
            ? `<tr><td colspan=9 class=empty>${t("no_matching_devices")}</td></tr>`
            : `<tr><td colspan=9 class=empty>${t("no_devices")} <a href=/discovery>${t("nav_discovery")}</a></td></tr>`;
        return;
    }

    tbody.innerHTML = sorted.map((device) => {
        const statusBadgeClass = statusClass(device.status);
        const rowClass = device.error_spike ? " class=spike-row" : "";
        const spikeBadge = device.error_spike ? "<span class=spike-badge>&#9888; Error Spike</span>" : "";
        const deleting = deletingIps.has(device.ip);
        const actionButton = `<button type="button" class="device-action-button" data-action-device="${escapeHtml(device.ip)}" aria-haspopup="dialog" ${deleting || typeof device.id !== "number" ? "disabled" : ""}>${escapeHtml(metadataText("actions"))}</button>`;
        const lastSeen = fmtTime(device.last_seen || device.last_seen_at);
        const healthBadgeClass = statusClass(device.health_status);
        const health = typeof device.health_score === "number" && Number.isFinite(device.health_score) ? `<span class="${healthBadgeClass}">${device.health_score}/100</span><span class="health-meter ${healthBadgeClass}" role="meter" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${Math.min(100, Math.max(0, device.health_score))}" aria-label="${escapeHtml(t("health_score"))}"><span class="health-meter-fill" style="width:${Math.min(100, Math.max(0, device.health_score))}%"></span></span>` : "-";
        const checkbox = typeof device.id === "number" ? `<input type="checkbox" data-device-id="${device.id}" aria-label="${escapeHtml(`${metadataText("select")} ${device.ip}`)}" ${selectedIds.has(device.id) ? "checked" : ""}>` : "";
        const tags = (device.tags || []).map((tag) => `<span class="device-tag">${escapeHtml(tag)}</span>`).join("");
        return `<tr${rowClass}><td>${checkbox}</td><td><a href="/device/${escapeHtml(encodeURIComponent(device.ip))}" style='color:#38bdf8;text-decoration:none'>${escapeHtml(device.ip)}</a></td><td>${escapeHtml(deviceLabel(device))}${spikeBadge}</td><td>${escapeHtml(device.name || "-")}</td><td>${tags ? `<div class="device-tags">${tags}</div>` : "-"}</td><td><span class="${escapeHtml(statusBadgeClass)}">${escapeHtml(device.status)}</span></td><td>${health}</td><td>${lastSeen}</td><td><div class='row-actions'>${actionButton}</div></td></tr>`;
    }).join("");
    tbody.querySelectorAll<HTMLButtonElement>("[data-action-device]").forEach((button) => {
        button.onclick = () => {
            const device = currentDevices().find((item) => item.ip === button.dataset.actionDevice);
            if (device) bulkActions([device], refresh, true);
        };
    });
    tbody.querySelectorAll<HTMLInputElement>("[data-device-id]").forEach((checkbox) => {
        checkbox.onchange = () => {
            const id = Number(checkbox.dataset.deviceId);
            if (checkbox.checked) selectedIds.add(id); else selectedIds.delete(id);
            renderSelection();
        };
    });
}

export function updateSortIndicators(): void {
    columns.forEach((column) => {
        const header = document.getElementById(`th-${column}`);
        const icon = document.getElementById(`sort-${column}`);
        if (!header) return;
        if (column === sortColumn) {
            header.className = sortAscending ? "sort-asc" : "sort-desc";
            if (icon) icon.textContent = sortAscending ? "▴" : "▾";
        } else {
            header.className = "";
            if (icon) icon.textContent = "";
        }
    });
}

export function sortBy(column: string): void {
    if (!columns.includes(column as (typeof columns)[number])) return;
    sortAscending = sortColumn === column ? !sortAscending : true;
    sortColumn = column as (typeof columns)[number];
    updateSortIndicators();
    renderTable();
}

export function renderSummary(devices: DashboardDevice[] = currentDevices()): void {
    let online = 0;
    let warning = 0;
    let critical = 0;
    let offline = 0;
    let spikes = 0;
    devices.forEach((device) => {
        if (device.status === "online") online++;
        else if (device.status === "warning") warning++;
        else if (device.status === "critical") critical++;
        else if (device.status === "offline") offline++;
        if (device.error_spike) spikes++;
    });
    const element = document.getElementById("summary-cards");
    if (!element) return;
    const spikeClass = spikes > 0 ? "card card-spike" : "card";
    element.innerHTML =
        `<div class='card card-online'><div class='card-num'>${online}</div><div class='card-label'>${t("online")}</div></div>` +
        `<div class='card card-warning'><div class='card-num'>${warning}</div><div class='card-label'>${t("warning")}</div></div>` +
        `<div class='card card-offline'><div class='card-num'>${critical}</div><div class='card-label'>Critical</div></div>` +
        `<div class='card card-offline'><div class='card-num'>${offline}</div><div class='card-label'>${t("offline")}</div></div>` +
        `<div class='card'><div class='card-num'>${devices.length}</div><div class='card-label'>${t("total")}</div></div>` +
        `<div class='${spikeClass}'><div class='card-num'>${spikes}</div><div class='card-label'>${t("error_spikes")}</div></div>`;
}

export function unregisterDevice(encodedIp: string, encodedName: string): void {
    const ip = decodeURIComponent(encodedIp || "");
    const name = decodeURIComponent(encodedName || "");
    if (!window.confirm(`${t("unregister_confirm")}\n\n${ip}${name ? ` (${name})` : ""}`)) return;
    deletingIps.add(ip);
    renderTable();
    fetch(`/api/device/${encodeURIComponent(ip)}`, { method: "DELETE", headers: { "X-CSRF-Token": window.tracepulseCsrf || "" } })
        .then(async (response) => ({ ok: response.ok, status: response.status, body: await response.json() }))
        .then((result) => {
            if (!result.ok || result.body?.error) throw new Error(result.body?.error || `HTTP ${result.status}`);
            deletingIps.delete(ip);
            refresh();
            window.alert(t("unregister_success"));
        })
        .catch((error: unknown) => {
            deletingIps.delete(ip);
            renderTable();
            window.alert(`${t("unregister_failed")}: ${String(error)}`);
        });
}

function updateLastRefreshed(): void {
    const element = document.getElementById("last-refreshed");
    if (element) element.textContent = `${t("updated")}: ${formatTracePulseClock(new Date())}`;
}

function initializeManagement(): void {
    const table = document.getElementById("dash-table");
    if (!table || document.getElementById("device-search")) return;
    const toolbar = document.createElement("div");
    toolbar.className = "device-toolbar";
    const search = document.createElement("input");
    search.id = "device-search";
    search.type = "search";
    search.oninput = () => { searchQuery = search.value; renderTable(); };
    const count = document.createElement("span");
    count.id = "device-selection-count";
    count.setAttribute("aria-live", "polite");
    const rename = document.createElement("button");
    rename.id = "bulk-rename";
    rename.onclick = () => bulkActions([...selectedIds].flatMap((id) => {
        const device = currentDevices().find((item) => item.id === id);
        return device ? [device] : [];
    }), refresh);
    toolbar.append(search, count, rename);
    const error = document.createElement("p");
    error.id = "device-load-error";
    error.setAttribute("role", "alert");
    toolbar.append(error);
    table.parentElement!.before(toolbar);
    matchesFilters = initializeDeviceFilters(toolbar, renderTable);
    const header = document.createElement("th");
    const all = document.createElement("input");
    all.type = "checkbox";
    all.id = "select-visible-devices";
    all.onchange = () => {
        visibleDevices().forEach((device) => {
            if (typeof device.id !== "number") return;
            if (all.checked) selectedIds.add(device.id); else selectedIds.delete(device.id);
        });
        renderTable();
    };
    header.append(all);
    table.querySelector("thead tr")!.prepend(header);
}

export function refresh(): void {
    fetch("/api/devices")
        .then(async (response) => {
            const data: DashboardDevice[] | { error?: string } = await response.json();
            if (!response.ok || !Array.isArray(data)) throw new Error(!Array.isArray(data) ? data.error || "Device request failed" : "Device request failed");
            return data;
        })
        .then((devices) => {
            window.DEVICES = devices;
            const error = document.getElementById("device-load-error");
            if (error) error.textContent = "";
            renderTable();
            renderSummary(devices);
            renderAttention(devices);
            updateLastRefreshed();
        })
        .catch((error: unknown) => {
            const message = document.getElementById("device-load-error");
            if (message) message.textContent = String(error);
            console.warn("refresh failed", error);
        });
}

window.renderTable = renderTable;
window.renderSummary = renderSummary;
window.refreshDashboard = refresh;
window.sortBy = sortBy;
window.unregisterDevice = unregisterDevice;

initializeManagement();
renderTable();
renderSummary();
renderAttention();
updateSortIndicators();
updateLastRefreshed();
window.setInterval(refresh, 30_000);
document.addEventListener("DOMContentLoaded", () => initializeOnboarding(true));
window.addEventListener("tracepulse-language-change", () => { renderTable(); renderAttention(); });
