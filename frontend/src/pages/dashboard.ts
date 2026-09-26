import { esc as escapeHtml } from "../shared/escape";
import { fmtTime, formatTracePulseClock } from "../shared/format";

interface DashboardDevice {
    ip: string;
    name?: string;
    status?: string;
    community?: string;
    last_seen?: string;
    last_seen_at?: string;
    error_spike?: boolean;
}

const columns = ["ip", "name", "status", "community", "last_seen"] as const;
let sortColumn: (typeof columns)[number] = "ip";
let sortAscending = true;
const deletingIps = new Set<string>();

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
    const left = String(a[column] || "").toLowerCase();
    const right = String(b[column] || "").toLowerCase();
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
        if (device.status === "offline" || device.status === "critical") reasons.push(t("offline"));
        else if (device.status === "warning") reasons.push(t("warning"));
        return `<li class='attention-item'><a href='/device/${encodeURIComponent(device.ip)}'>${escapeHtml(device.name || device.ip)}</a><span class='attention-reason'>${escapeHtml(reasons.join(" / "))}</span></li>`;
    }).join("");
    element.innerHTML = `<h2>${t("attention_devices")}</h2><ul class='attention-list'>${items}</ul>`;
}

export function renderTable(): void {
    const tbody = document.getElementById("dash-tbody");
    if (!tbody) return;
    const sorted = currentDevices().slice().sort((a, b) => {
        if (a.error_spike && !b.error_spike) return -1;
        if (!a.error_spike && b.error_spike) return 1;
        const result = compareValue(a, b, sortColumn);
        return sortAscending ? result : -result;
    });

    if (sorted.length === 0) {
        tbody.innerHTML = `<tr><td colspan=6 class=empty>${t("no_devices")} <a href=/discovery>${t("nav_discovery")}</a></td></tr>`;
        return;
    }

    tbody.innerHTML = sorted.map((device) => {
        const classMap: Record<string, string> = { online: "status-online", offline: "status-offline", warning: "status-warning", critical: "status-critical" };
        const statusClass = classMap[device.status || ""] || "status-unknown";
        const rowClass = device.error_spike ? " class=spike-row" : "";
        const spikeBadge = device.error_spike ? "<span class=spike-badge>&#9888; Error Spike</span>" : "";
        const deleting = deletingIps.has(device.ip);
        const actionLabel = deleting ? t("unregistering") : t("unregister");
        const encodedIp = encodeURIComponent(device.ip);
        const encodedName = encodeURIComponent(device.name || device.ip);
        const actionButton = `<button class='btn-row-danger' data-ip='${encodedIp}' data-name='${encodedName}' onclick='unregisterDevice(this.dataset.ip,this.dataset.name)' ${deleting ? "disabled" : ""}>${actionLabel}</button>`;
        const lastSeen = fmtTime(device.last_seen || device.last_seen_at);
        return `<tr${rowClass}><td><a href='/device/${encodeURIComponent(device.ip)}' style='color:#38bdf8;text-decoration:none'>${escapeHtml(device.ip)}</a></td><td>${escapeHtml(device.name || "")}${spikeBadge}</td><td><span class=${statusClass}>${escapeHtml(device.status)}</span></td><td>${escapeHtml(device.community)}</td><td>${lastSeen}</td><td><div class='row-actions'>${actionButton}</div></td></tr>`;
    }).join("");
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
    let offline = 0;
    let spikes = 0;
    devices.forEach((device) => {
        if (device.status === "online") online++;
        else if (device.status === "warning") warning++;
        else if (device.status === "offline" || device.status === "critical") offline++;
        if (device.error_spike) spikes++;
    });
    const element = document.getElementById("summary-cards");
    if (!element) return;
    const spikeClass = spikes > 0 ? "card card-spike" : "card";
    element.innerHTML =
        `<div class='card card-online'><div class='card-num'>${online}</div><div class='card-label'>${t("online")}</div></div>` +
        `<div class='card card-warning'><div class='card-num'>${warning}</div><div class='card-label'>${t("warning")}</div></div>` +
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
    fetch(`/api/device/${encodeURIComponent(ip)}`, { method: "DELETE" })
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

export function refresh(): void {
    fetch("/api/devices")
        .then((response) => response.json() as Promise<DashboardDevice[]>)
        .then((devices) => {
            window.DEVICES = devices;
            renderTable();
            renderSummary(devices);
            renderAttention(devices);
            updateLastRefreshed();
        })
        .catch((error: unknown) => console.warn("refresh failed", error));
}

window.renderTable = renderTable;
window.renderSummary = renderSummary;
window.refreshDashboard = refresh;
window.sortBy = sortBy;
window.unregisterDevice = unregisterDevice;

renderTable();
renderAttention();
updateSortIndicators();
updateLastRefreshed();
window.setInterval(refresh, 30_000);
