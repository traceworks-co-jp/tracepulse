import { esc as escapeHtml } from "../shared/escape";
import {
    fmtNum,
    fmtTime,
    formatBps,
    formatTracePulseClock,
    formatTracePulseTime,
    parseTracePulseTime,
    tracePulseTimeZone,
} from "../shared/format";

const palette = ["#38bdf8", "#4ade80", "#f59e0b", "#f87171", "#a78bfa", "#34d399"];
const chartColors = ["#38bdf8", "#4ade80", "#f59e0b", "#f87171", "#a78bfa", "#34d399", "#fb923c", "#e879f9"];
const analyticsRefreshOptions = [5, 10, 30, 60];
let analyticsPollingMs = loadAnalyticsPollingMs();
let deviceRefreshSeconds = 30;
let errorBreakdownSelectedIf: number | null = null;
let analyticsWindow = "60s";
let analyticsPaused = false;
let analyticsTimer: number | null = null;
let deviceRefreshTimer: number | null = null;
let ifaceSelected = new Set<string>();
let lastDeviceDetail: DeviceDetail | null = null;
let ifaceSelectionReady = false;
let resizeTimer: number | null = null;

function loadAnalyticsPollingMs(): number {
    try {
        const seconds = Number(localStorage.getItem("tracepulse-traffic-refresh-seconds"));
        return analyticsRefreshOptions.includes(seconds) ? seconds * 1000 : 5_000;
    } catch {
        return 5_000;
    }
}

function saveAnalyticsPollingMs(value: number): void {
    try { localStorage.setItem("tracepulse-traffic-refresh-seconds", String(value / 1000)); } catch { /* ignore */ }
}

interface ErrorBreakdown {
    fcs_errors?: number;
    fcs_errors_delta?: number;
    alignment_errors?: number;
    alignment_errors_delta?: number;
    frame_too_longs?: number;
    frame_too_longs_delta?: number;
    internal_mac_receive_errors?: number;
    internal_mac_receive_errors_delta?: number;
}

interface DeviceInterface {
    if_index: number;
    if_name?: string;
    link_status?: string;
    health_status?: string;
    metrics?: InterfaceMetrics;
    error_breakdown?: ErrorBreakdown;
    predictive_status?: PredictiveStatus;
    sampled_at?: string;
}

interface DeviceDetail {
    name?: string;
    ip?: string;
    status?: string;
    error?: string;
    interfaces?: DeviceInterface[];
    if_series?: InterfaceSeries[];
    metrics?: SystemMetric[];
    hardware_sensors?: HardwareSensor[];
    spikes?: InterfaceSpike[];
    alerts?: AlertRow[];
}

interface SettingsResponse {
    polling?: {
        interval_seconds?: number;
    };
    display?: {
        timezone?: string;
    };
    alert?: {
        spike_threshold?: number;
    };
}

interface InterfaceMetrics {
    in_errors?: number;
    in_errors_delta?: number;
    out_errors?: number;
    out_errors_delta?: number;
    in_discards?: number;
    in_discards_delta?: number;
    out_discards?: number;
    out_discards_delta?: number;
    late_collisions?: number;
    late_collisions_delta?: number;
    bandwidth_utilization?: number;
}

interface PredictiveStatus {
    dom_warning?: boolean;
    trend_warning?: boolean;
    rx_optical_power_dbm?: number | null;
}

interface HardwareSensor {
    sensor_type?: string;
    value?: string | number | null;
    status?: string | number | null;
    status_text?: string | null;
    unit?: string | null;
    name?: string | null;
    source?: string | null;
    index?: string | number | null;
    is_alarm?: boolean;
}

interface InterfaceSpike {
    latest_sampled_at?: string;
    if_name?: string;
    if_index?: number;
    link_status?: string;
    in_errors_delta?: number;
    out_errors_delta?: number;
    in_discards_delta?: number;
    out_discards_delta?: number;
    total_delta?: number;
}

interface AlertRow {
    at?: string;
    interface?: string;
    type?: string;
    severity?: string;
    details?: string;
}

interface ChartPoint {
    t: string;
    v: number;
}

interface ChartSeries {
    label: string;
    color: string;
    points: ChartPoint[];
}

interface InterfaceSeriesPoint {
    t: string;
    bw?: number;
    in_err?: number;
    in_dis?: number;
}

interface InterfaceSeries {
    if_index: number;
    points: InterfaceSeriesPoint[];
}

interface SystemMetric {
    t: string;
    cpu?: number | null;
    memory?: number | null;
    memory_bytes?: number | null;
}

function t(key: string): string {
    return typeof window.t === "function" ? window.t(key) : key;
}

function ifaceSelectionKey(): string {
    return `tracepulse-iface-selection:${window.DEVICE_IP || ""}`;
}

function saveSelectedInterfaces(): void {
    try {
        localStorage.setItem(ifaceSelectionKey(), JSON.stringify(Array.from(ifaceSelected)));
    } catch {
        // Ignore unavailable localStorage.
    }
}

function syncSelectedInterfaces(ifaces: DeviceInterface[]): void {
    const known = new Set(ifaces.map((iface) => String(iface.if_index)));
    if (!ifaceSelectionReady) {
        let stored: unknown = null;
        try {
            stored = JSON.parse(localStorage.getItem(ifaceSelectionKey()) || "null");
        } catch {
            stored = null;
        }

        ifaceSelected = new Set();
        if (Array.isArray(stored) && stored.length > 0) {
            stored.forEach((value) => {
                const key = String(value);
                if (known.has(key)) ifaceSelected.add(key);
            });
        }

        if (ifaceSelected.size === 0) {
            known.forEach((value) => ifaceSelected.add(value));
        }
        ifaceSelectionReady = true;
    } else {
        Array.from(ifaceSelected).forEach((value) => {
            if (!known.has(String(value))) ifaceSelected.delete(value);
        });
    }

    saveSelectedInterfaces();
}

export function isInterfaceSelected(ifIndex: unknown): boolean {
    return ifaceSelected.has(String(ifIndex));
}

export function selectAllInterfaces(checked: boolean): void {
    const ifaces = lastDeviceDetail?.interfaces || [];
    ifaceSelected = new Set();
    if (checked) {
        ifaces.forEach((iface) => ifaceSelected.add(String(iface.if_index)));
    }
    saveSelectedInterfaces();
    if (lastDeviceDetail) renderDetail(lastDeviceDetail);
}

export function toggleInterfaceSelection(ifIndex: unknown, checked: boolean): void {
    const key = String(ifIndex);
    if (checked) {
        ifaceSelected.add(key);
    } else {
        ifaceSelected.delete(key);
    }
    saveSelectedInterfaces();
    if (lastDeviceDetail) renderDetail(lastDeviceDetail);
}

function renderInterfaceSelection(ifaces: DeviceInterface[] = []): void {
    const box = document.getElementById("iface-filter");
    const note = document.getElementById("iface-filter-note");
    if (!box) return;

    box.innerHTML = ifaces
        .map((iface) => {
            const id = `iface-select-${iface.if_index}`;
            const label = iface.if_name || `if-${iface.if_index}`;
            const checked = isInterfaceSelected(iface.if_index) ? " checked" : "";
            return `<label class='iface-filter-item' for='${id}'><input id='${id}' type='checkbox' value='${iface.if_index}'${checked} onchange='toggleInterfaceSelection(this.value,this.checked)'><span>${escapeHtml(label)} (if-${iface.if_index})</span></label>`;
        })
        .join("");

    if (note) {
        note.textContent = ifaces.length ? `${Array.from(ifaceSelected).length} / ${ifaces.length} ${t("selected")}` : `0 / 0 ${t("selected")}`;
    }
}

function includeTrafficItem(item: ProtocolShare | TopTalker | null | undefined): boolean {
    if (!item) return false;
    if (!Object.prototype.hasOwnProperty.call(item, "if_index")) return true;
    if (typeof window.isInterfaceSelected !== "function") return true;
    return window.isInterfaceSelected(item.if_index);
}

function trafficProtocolsEmptyHtml(): string {
    return `<p class='traffic-empty'>${t("traffic_protocols_empty")}</p>`;
}

export function renderTrafficProtocols(data: FlowAnalytics | null): void {
    const box = document.getElementById("traffic-protocols");
    const section = document.getElementById("traffic-protocols-section");
    if (!box || !section) return;

    const allProtocols = Array.isArray(data?.protocols) ? data.protocols : [];
    const allTalkers = Array.isArray(data?.top_talkers) ? data.top_talkers : [];
    section.style.display = allProtocols.length || allTalkers.length ? "block" : "none";
    if (section.style.display === "none") return;

    const protocols = allProtocols.filter(includeTrafficItem);
    const talkers = allTalkers.filter(includeTrafficItem);
    if (protocols.length === 0 && talkers.length === 0) {
        box.innerHTML = trafficProtocolsEmptyHtml();
        return;
    }

    let cursor = 0;
    const stops = protocols.map((item, index) => {
        const start = cursor;
        cursor += Number(item.percentage || 0);
        return `${palette[index % palette.length]} ${start}% ${cursor}%`;
    });

    const legend = protocols
        .map((item, index) => {
            const color = palette[index % palette.length];
            const percent = Number(item.percentage || 0).toFixed(1);
            return `<div class='traffic-legend-item'><span class='traffic-legend-swatch' style='background:${color}'></span>${escapeHtml(item.protocol)} ${percent}% (${formatBps(item.bps)})</div>`;
        })
        .join("");

    const rows = talkers
        .map((item) => {
            const flags = (item.tcp_flags || []).join(", ");
            const ingress = item.ingress_if_index === null || item.ingress_if_index === undefined ? "-" : item.ingress_if_index;
            const egress = item.egress_if_index === null || item.egress_if_index === undefined ? "-" : item.egress_if_index;
            return `<tr><td>${escapeHtml(item.source_ip)}:${item.source_port || 0}</td><td>${escapeHtml(item.destination_ip)}:${item.destination_port || 0}</td><td>${escapeHtml(item.protocol)}</td><td>${escapeHtml(item.app_name || "-")}</td><td>${formatBps(item.bps)}</td><td>${formatBps(item.pps)}</td><td>${escapeHtml(flags || "-")}</td><td>${escapeHtml(item.ingress_if_name || `if-${ingress}`)}</td><td>${escapeHtml(item.egress_if_name || `if-${egress}`)}</td></tr>`;
        })
        .join("");

    const summary = data?.summary || {};
    const applications = (data?.applications || []).map((item) => {
        const percentage = Number(item.percentage || 0);
        return `<div class='traffic-share-item'><div class='traffic-share-label'><span>${escapeHtml(item.app_name)}</span><span>${percentage.toFixed(1)}% · ${formatBps(item.bps)}</span></div><div class='traffic-share-track'><span style='width:${Math.min(100, Math.max(0, percentage))}%'></span></div></div>`;
    }).join("");
    const endpoints = (items: FlowEndpoint[]) => items.map((item) => `<div class='traffic-legend-item'><span>${escapeHtml(item.ip)}</span><span>${Number(item.percentage || 0).toFixed(1)}% · ${formatBps(item.bps)}</span></div>`).join("");
    const timeseries = (data?.timeseries || []).map((item) => `<tr><td>${escapeHtml(item.timestamp)}</td><td>${formatBps(item.udp_bps)}</td><td>${formatBps(item.tcp_bps)}</td><td>${formatBps(item.icmp_bps)}</td></tr>`).join("");

    box.innerHTML =
        `<section class='traffic-card traffic-summary-card'><div class='traffic-controls'><span class='traffic-card-label'>${t("traffic_protocols_window")}</span><button type='button' data-window='60s'>${t("traffic_protocols_live")}</button><button type='button' data-window='5m'>${t("traffic_protocols_last_5m")}</button><button type='button' data-window='1h'>${t("traffic_protocols_last_1h")}</button><button type='button' data-window='24h'>${t("traffic_protocols_last_24h")}</button><button type='button' id='traffic-pause'>${analyticsPaused ? t("traffic_protocols_resume") : t("traffic_protocols_pause")}</button><label class='traffic-card-label' for='traffic-refresh-interval'>${t("traffic_protocols_refresh_interval")}</label><select id='traffic-refresh-interval' class='traffic-refresh-select'>${analyticsRefreshOptions.map((seconds) => `<option value='${seconds}'${analyticsPollingMs === seconds * 1000 ? " selected" : ""}>${seconds}s</option>`).join("")}</select><span class='traffic-card-label'>${t("traffic_protocols_auto_refresh").replace("{seconds}", String(analyticsPollingMs / 1000))}</span></div><h3>${t("traffic_protocols_summary")} (${escapeHtml(analyticsWindow)})</h3><div class='traffic-kpi-row'><div><span>${t("traffic_protocols_total_bps")}</span><strong>${formatBps(summary.total_bps)}</strong></div><div><span>${t("traffic_protocols_packet_rate")}</span><strong>${formatBps(summary.total_pps)} pps</strong></div><div><span>${t("traffic_protocols_active_flows")}</span><strong>${fmtNum(summary.active_flows)}</strong></div><div><span>${t("traffic_protocols_top_protocol")}</span><strong>${escapeHtml(summary.top_protocol || "-")}</strong></div></div></section>` +
        `<section class='traffic-card traffic-timeseries-card'><h3>${t("traffic_protocols_timeseries")}</h3><div class='traffic-table-scroll traffic-timeseries-scroll'><table class='talker-table'><thead><tr><th>${t("traffic_protocols_time")}</th><th>UDP</th><th>TCP</th><th>ICMP</th></tr></thead><tbody>${timeseries}</tbody></table></div></section>` +
        `<section class='traffic-card traffic-protocol-card'><h3>${t("traffic_protocols_share")}</h3><div class='traffic-donut-wrap'><div class='traffic-donut' style='background:conic-gradient(${stops.join(",") || "#334155 0 100%"})'></div><div class='traffic-legend'>${legend}</div></div></section>` +
        `<section class='traffic-card traffic-applications-card'><h3>${t("traffic_protocols_applications")}</h3><div class='traffic-scroll-panel traffic-share-list'>${applications || trafficProtocolsEmptyHtml()}</div></section>` +
        `<section class='traffic-card traffic-endpoints-card'><h3>${t("traffic_protocols_top_sources")}</h3><div class='traffic-scroll-panel traffic-legend'>${endpoints(data?.top_sources || []) || trafficProtocolsEmptyHtml()}</div></section>` +
        `<section class='traffic-card traffic-endpoints-card'><h3>${t("traffic_protocols_top_destinations")}</h3><div class='traffic-scroll-panel traffic-legend'>${endpoints(data?.top_destinations || []) || trafficProtocolsEmptyHtml()}</div></section>` +
        `<section class='traffic-card traffic-talkers-card'><h3>${t("traffic_protocols_top_talkers")}</h3><div class='traffic-table-scroll'><table class='talker-table traffic-talkers-table'><thead><tr><th>${t("traffic_protocols_source")}</th><th>${t("traffic_protocols_destination")}</th><th>${t("traffic_protocols_protocol")}</th><th>${t("traffic_protocols_application")}</th><th>bps</th><th>pps</th><th>${t("traffic_protocols_tcp_flags")}</th><th>${t("traffic_protocols_in_if")}</th><th>${t("traffic_protocols_out_if")}</th></tr></thead><tbody>${rows}</tbody></table></div></section>`;
    box.querySelectorAll<HTMLButtonElement>("[data-window]").forEach((button) => button.addEventListener("click", () => {
        analyticsWindow = button.dataset.window || "60s";
        refreshTrafficProtocols();
    }));
    box.querySelector<HTMLButtonElement>("#traffic-pause")?.addEventListener("click", () => {
        analyticsPaused = !analyticsPaused;
        scheduleAnalyticsPolling();
        refreshTrafficProtocols();
    });
    box.querySelector<HTMLSelectElement>("#traffic-refresh-interval")?.addEventListener("change", (event) => {
        const seconds = Number((event.target as HTMLSelectElement).value);
        if (!analyticsRefreshOptions.includes(seconds)) return;
        analyticsPollingMs = seconds * 1000;
        saveAnalyticsPollingMs(analyticsPollingMs);
        scheduleAnalyticsPolling();
        renderTrafficProtocols(data);
    });
}

export function refreshTrafficProtocols(): void {
    if (analyticsPaused || document.visibilityState === "hidden") return;
    fetch(`/api/flow/analytics?window=${encodeURIComponent(analyticsWindow)}&limit=10`, { cache: "no-store" })
        .then((response) => response.json() as Promise<FlowAnalytics>)
        .then(renderTrafficProtocols)
        .catch(() => renderTrafficProtocols(null));
}

function scheduleAnalyticsPolling(): void {
    if (analyticsTimer !== null) window.clearInterval(analyticsTimer);
    analyticsTimer = analyticsPaused ? null : window.setInterval(refreshTrafficProtocols, analyticsPollingMs);
}

function counterClass(delta: number, spikeThreshold: number): string {
    if (delta >= spikeThreshold) return "crit";
    if (delta > 0) return "warn";
    return "none";
}

function formatCounter(total: unknown, delta: unknown): string {
    const numericDelta = Number(delta || 0);
    const totalText = `<span class='counter-total'>${fmtNum(total)}</span>`;
    if (!numericDelta) return totalText;
    return `${totalText}<span class='counter-delta ${counterClass(numericDelta, window.TRACEPULSE_SPIKE_THRESHOLD || 10)}'> (+${fmtNum(numericDelta)})</span>`;
}

function formatLateCollisions(total: unknown, delta: unknown): string {
    const numericDelta = Number(delta || 0);
    const totalText = `<span class='counter-total'>${fmtNum(total)}</span>`;
    if (!numericDelta) return totalText;
    return `${totalText}<span class='counter-delta duplex'> (+${fmtNum(numericDelta)})</span>`;
}

function errorBreakdownPopoverHtml(errorBreakdown: ErrorBreakdown = {}, metrics: InterfaceMetrics = {}): string {
    return "<div class='err-pop'>" +
        `<div class='err-pop-title'>${t("error_breakdown_hover_title")}</div>` +
        `<div class='err-pop-row'><span class='n'>${t("in_errors")}</span><span class='v'>+${fmtNum(metrics.in_errors_delta)}</span></div>` +
        `<div class='err-pop-row'><span class='n'>${t("out_errors")}</span><span class='v'>+${fmtNum(metrics.out_errors_delta)}</span></div>` +
        `<div class='err-pop-row'><span class='n'>${t("in_discards")}</span><span class='v'>+${fmtNum(metrics.in_discards_delta)}</span></div>` +
        `<div class='err-pop-row'><span class='n'>${t("out_discards")}</span><span class='v'>+${fmtNum(metrics.out_discards_delta)}</span></div>` +
        `<div class='err-pop-row'><span class='n'>${t("late_collisions")}</span><span class='v'>+${fmtNum(metrics.late_collisions_delta)}</span></div>` +
        `<div class='err-pop-divider'></div>` +
        `<div class='err-pop-row'><span class='n'>${t("error_breakdown_fcs_errors")}</span><span class='v'>+${fmtNum(errorBreakdown.fcs_errors_delta)}</span></div>` +
        `<div class='err-pop-row'><span class='n'>${t("error_breakdown_alignment_errors")}</span><span class='v'>+${fmtNum(errorBreakdown.alignment_errors_delta)}</span></div>` +
        `<div class='err-pop-row'><span class='n'>${t("error_breakdown_frame_too_longs")}</span><span class='v'>+${fmtNum(errorBreakdown.frame_too_longs_delta)}</span></div>` +
        `<div class='err-pop-row'><span class='n'>${t("error_breakdown_internal_mac_receive_errors")}</span><span class='v'>+${fmtNum(errorBreakdown.internal_mac_receive_errors_delta)}</span></div>` +
        "</div>";
}

function formatCounterWithBreakdown(total: unknown, delta: unknown, errorBreakdown?: ErrorBreakdown, metrics?: InterfaceMetrics, flipPopover = false): string {
    return `<span class='err-cell${flipPopover ? " flip" : ""}' tabindex='0'>${formatCounter(total, delta)}${errorBreakdownPopoverHtml(errorBreakdown, metrics)}</span>`;
}

function diagnosticBadge(status: string | undefined): string {
    const key = status || "healthy";
    return `<span class='diagnostic-badge ${escapeHtml(key)}'>${t(`diagnostic_${key}`)}</span>`;
}

export function renderInterfaces(ifaces: DeviceInterface[] = []): void {
    const tbody = document.getElementById("if-tbody");
    if (!tbody) return;
    if (ifaces.length === 0) {
        tbody.innerHTML = `<tr><td colspan=11 class=no-data>${t("no_interface_data")}</td></tr>`;
        return;
    }
    const spikeThreshold = window.TRACEPULSE_SPIKE_THRESHOLD || 10;
    tbody.innerHTML = ifaces
        .map((iface, index) => {
            const metrics = iface.metrics || {};
            const linkClass = iface.link_status === "up" ? "link-up" : "link-down";
            const errorDelta = Math.max(metrics.in_errors_delta || 0, metrics.out_errors_delta || 0);
            const discardDelta = Math.max(metrics.in_discards_delta || 0, metrics.out_discards_delta || 0);
            const rowClass = errorDelta >= spikeThreshold || discardDelta >= spikeThreshold ? " row-crit" : errorDelta > 0 || discardDelta > 0 ? " row-warn" : "";
            const predictiveStatus = iface.predictive_status;
            const predictiveBadge = predictiveStatus && (predictiveStatus.dom_warning || predictiveStatus.trend_warning) ? "<span class='pred-badge'>[PRED]</span>" : "";
            const flipErrorPopover = index >= ifaces.length - 2;
            const rxPower = predictiveStatus?.rx_optical_power_dbm;
            const domMeter = rxPower !== null && rxPower !== undefined
                ? `<div class='dom-meter'><span>Rx Power: ${Number(rxPower).toFixed(1)} dBm</span><span class='dom-meter-track'><span class='dom-meter-fill ${predictiveStatus?.dom_warning ? "" : "ok"}' style='width:${Math.max(0, Math.min(100, (Number(rxPower) + 30) / 30 * 100))}%'></span></span></div>`
                : "";
            return `<tr class='${rowClass.trim()}'>` +
                `<td>${iface.if_index}</td>` +
                `<td>${escapeHtml(iface.if_name)}${predictiveBadge}</td>` +
                `<td><span class=${linkClass}>${escapeHtml(iface.link_status)}</span></td>` +
                `<td>${diagnosticBadge(iface.health_status)}</td>` +
                `<td>${formatCounterWithBreakdown(metrics.in_errors, metrics.in_errors_delta, iface.error_breakdown, metrics, flipErrorPopover)}</td>` +
                `<td>${formatCounterWithBreakdown(metrics.out_errors, metrics.out_errors_delta, iface.error_breakdown, metrics, flipErrorPopover)}</td>` +
                `<td>${formatCounter(metrics.in_discards, metrics.in_discards_delta)}</td>` +
                `<td>${formatCounter(metrics.out_discards, metrics.out_discards_delta)}</td>` +
                `<td>${formatLateCollisions(metrics.late_collisions, metrics.late_collisions_delta)}</td>` +
                `<td>${(Number(metrics.bandwidth_utilization || 0) * 100).toFixed(1)}%${domMeter}</td>` +
                `<td>${fmtTime(iface.sampled_at)}</td>` +
                "</tr>";
        })
        .join("");
}

export function renderHardwareStatus(sensors: HardwareSensor[] = []): void {
    const box = document.getElementById("hardware-status");
    if (!box) return;
    const visible = sensors.filter((sensor) => {
        const type = (sensor.sensor_type || "").toLowerCase();
        const hasValue = sensor.value !== null && sensor.value !== undefined || sensor.status !== null && sensor.status !== undefined;
        return (type === "temperature" || type === "power" || type === "fan") && hasValue;
    });
    if (visible.length === 0) {
        box.innerHTML = "<span class=no-data>No Sensors Detected</span>";
        return;
    }
    box.innerHTML = visible
        .map((sensor) => {
            const cls = sensor.is_alarm ? "hardware-card crit" : sensor.status !== null && sensor.status !== undefined && sensor.status !== 0 ? "hardware-card warn" : "hardware-card";
            const label = sensor.name || (sensor.source === "entity-physical" ? `component ${sensor.index}` : `sensor ${sensor.index}`);
            const displayValue = sensor.status_text
                ? sensor.status_text
                : sensor.value !== null && sensor.value !== undefined
                    ? `${sensor.value}${sensor.unit ? ` ${sensor.unit}` : ""}`
                    : sensor.status !== null && sensor.status !== undefined
                        ? sensor.status
                        : "N/A";
            return `<div class='${cls}'><div class='label'>${escapeHtml(label)}</div><div class='value'>${escapeHtml(displayValue)}</div></div>`;
        })
        .join("");
}

export function renderErrorBreakdownCard(ifaces: DeviceInterface[] = []): void {
    const box = document.getElementById("error-breakdown");
    if (!box) return;
    if (ifaces.length === 0) {
        box.innerHTML = `<p class='no-data'>${t("error_breakdown_no_data")}</p>`;
        return;
    }

    if (errorBreakdownSelectedIf === null || !ifaces.some((iface) => iface.if_index === errorBreakdownSelectedIf)) {
        errorBreakdownSelectedIf = ifaces[0].if_index;
    }

    const options = ifaces
        .map((iface) => {
            const selected = iface.if_index === errorBreakdownSelectedIf ? " selected" : "";
            return `<option value='${iface.if_index}'${selected}>${escapeHtml(iface.if_name || `if-${iface.if_index}`)} (if-${iface.if_index})</option>`;
        })
        .join("");

    const selected = ifaces.find((iface) => iface.if_index === errorBreakdownSelectedIf) || ifaces[0];
    const eb = selected.error_breakdown || {};
    const fcs = eb.fcs_errors_delta || 0;
    const align = eb.alignment_errors_delta || 0;
    const toolong = eb.frame_too_longs_delta || 0;
    const macrx = eb.internal_mac_receive_errors_delta || 0;
    const total = fcs + align + toolong + macrx;
    const pct = (value: number) => total > 0 ? value / total * 100 : 0;

    const bar =
        "<div class='breakdown-bar'>" +
        `<div class='breakdown-bar-seg fcs' style='width:${pct(fcs)}%'></div>` +
        `<div class='breakdown-bar-seg alignment' style='width:${pct(align)}%'></div>` +
        `<div class='breakdown-bar-seg frametoolong' style='width:${pct(toolong)}%'></div>` +
        `<div class='breakdown-bar-seg macreceive' style='width:${pct(macrx)}%'></div>` +
        "</div>";

    const legend =
        "<div class='breakdown-legend'>" +
        `<span><span class='swatch' style='background:#f87171'></span>${t("error_breakdown_fcs_errors")}</span>` +
        `<span><span class='swatch' style='background:#fbbf24'></span>${t("error_breakdown_alignment_errors")}</span>` +
        `<span><span class='swatch' style='background:#a78bfa'></span>${t("error_breakdown_frame_too_longs")}</span>` +
        `<span><span class='swatch' style='background:#38bdf8'></span>${t("error_breakdown_internal_mac_receive_errors")}</span>` +
        "</div>";

    const table =
        "<table class='breakdown-table'><thead><tr>" +
        `<th>${t("error_breakdown_fcs_errors")}</th>` +
        `<th>${t("error_breakdown_alignment_errors")}</th>` +
        `<th>${t("error_breakdown_frame_too_longs")}</th>` +
        `<th>${t("error_breakdown_internal_mac_receive_errors")}</th>` +
        "</tr></thead><tbody><tr>" +
        `<td>${fmtNum(eb.fcs_errors)} <span class='counter-delta warn'>(+${fmtNum(fcs)})</span></td>` +
        `<td>${fmtNum(eb.alignment_errors)} <span class='counter-delta warn'>(+${fmtNum(align)})</span></td>` +
        `<td>${fmtNum(eb.frame_too_longs)} <span class='counter-delta warn'>(+${fmtNum(toolong)})</span></td>` +
        `<td>${fmtNum(eb.internal_mac_receive_errors)} <span class='counter-delta warn'>(+${fmtNum(macrx)})</span></td>` +
        "</tr></tbody></table>";

    box.innerHTML =
        `<div class='error-breakdown-select'><label for='error-breakdown-if'>${t("error_breakdown_select_hint")}</label>` +
        `<select id='error-breakdown-if' onchange='onErrorBreakdownIfaceChange(this.value)'>${options}</select></div>` +
        bar + legend + table;
}

export function onErrorBreakdownIfaceChange(value: string): void {
    errorBreakdownSelectedIf = parseInt(value, 10);
    if (window.LAST_DEVICE_DETAIL) renderErrorBreakdownCard(window.LAST_DEVICE_DETAIL.interfaces || []);
}

export function renderSpikes(spikes: InterfaceSpike[] = []): void {
    const tbody = document.getElementById("spike-tbody");
    if (!tbody) return;
    if (spikes.length === 0) {
        tbody.innerHTML = `<tr><td colspan=8 class=no-data>${t("no_interface_spikes")}</td></tr>`;
        return;
    }
    tbody.innerHTML = spikes
        .map((spike) => {
            const linkClass = spike.link_status === "up" ? "link-up" : "link-down";
            return "<tr>" +
                `<td>${fmtTime(spike.latest_sampled_at)}</td>` +
                `<td>${escapeHtml(spike.if_name)} (if-${spike.if_index || 0})</td>` +
                `<td><span class=${linkClass}>${escapeHtml(spike.link_status)}</span></td>` +
                `<td>${fmtNum(spike.in_errors_delta)}</td>` +
                `<td>${fmtNum(spike.out_errors_delta)}</td>` +
                `<td>${fmtNum(spike.in_discards_delta)}</td>` +
                `<td>${fmtNum(spike.out_discards_delta)}</td>` +
                `<td>${fmtNum(spike.total_delta)}</td>` +
                "</tr>";
        })
        .join("");
}

export function renderAlerts(alerts: AlertRow[] = []): void {
    const tbody = document.getElementById("alert-tbody");
    if (!tbody) return;
    if (alerts.length === 0) {
        tbody.innerHTML = `<tr><td colspan=5 class=no-data>${t("no_alerts")}</td></tr>`;
        return;
    }
    const severityClasses: Record<string, string> = { warning: "sev-warning", critical: "sev-critical", info: "sev-info" };
    tbody.innerHTML = alerts
        .map((alert) => {
            const severity = alert.severity || "";
            const severityClass = severityClasses[severity] || "";
            const iface = alert.interface || "-";
            return `<tr><td>${fmtTime(alert.at)}</td><td>${escapeHtml(iface)}</td><td>${escapeHtml(alert.type)}</td><td><span class=${severityClass}>${escapeHtml(severity)}</span></td><td>${escapeHtml(alert.details)}</td></tr>`;
        })
        .join("");
}

function formatAxisValue(value: number, yLabel: string): string {
    if (yLabel === "bytes") {
        const units = ["B", "KB", "MB", "GB", "TB"];
        let displayValue = value;
        let index = 0;
        while (displayValue >= 1024 && index + 1 < units.length) {
            displayValue /= 1024;
            index++;
        }
        return `${index === 0 ? Math.round(displayValue) : displayValue.toFixed(1)} ${units[index]}`;
    }
    if (yLabel === "%") {
        if (value < 1) return value.toFixed(2);
        if (value < 10) return value.toFixed(1);
    }
    return Math.round(value).toString();
}

export function sparkline(svgId: string, series: ChartSeries[], yLabel: string): void {
    const svg = document.getElementById(svgId) as SVGElement | null;
    if (!svg) return;
    const rect = svg.getBoundingClientRect();
    const width = Math.max(200, Math.round(rect.width) || 800);
    const height = Math.max(80, Math.round(rect.height) || 160);
    svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
    const fontSize = width < 420 ? 8 : 9;
    const paddingLeft = width < 420 ? 30 : 40;
    const paddingRight = 8;
    const paddingTop = 8;
    const paddingBottom = 20;
    const chartWidth = width - paddingLeft - paddingRight;
    const chartHeight = height - paddingTop - paddingBottom;

    const allPoints = series.flatMap((item) => (item.points || []).filter((point) => point && point.v !== null && point.v !== undefined && !Number.isNaN(point.v)));
    if (allPoints.length === 0) {
        svg.classList.add("chart-empty");
        svg.innerHTML = `<text x='${width / 2}' y='${height / 2}' text-anchor='middle' fill='#64748b' font-size='13'>N/A</text>`;
        return;
    }
    svg.classList.remove("chart-empty");

    const times = allPoints.map((point) => parseTracePulseTime(point.t).getTime());
    const values = allPoints.map((point) => point.v);
    const timeMin = Math.min(...times);
    let timeMax = Math.max(...times);
    const valueMin = 0;
    const valueMax = yLabel === "%" ? 100 : Math.max(...values) * 1.15 || 1;
    if (timeMin === timeMax) timeMax = timeMin + 1;

    const tx = (time: string) => paddingLeft + (parseTracePulseTime(time).getTime() - timeMin) / (timeMax - timeMin) * chartWidth;
    const ty = (value: number) => paddingTop + chartHeight - (value - valueMin) / (valueMax - valueMin) * chartHeight;
    let output = "";

    for (let index = 0; index <= 4; index++) {
        const gridY = paddingTop + index * chartHeight / 4;
        const gridValue = (valueMax - valueMin) * (1 - index / 4) + valueMin;
        output += `<line x1='${paddingLeft}' y1='${gridY}' x2='${width - paddingRight}' y2='${gridY}' stroke='#1e293b' stroke-width='1'/>`;
        output += `<text x='${paddingLeft - 4}' y='${gridY + 4}' text-anchor='end' fill='#64748b' font-size='${fontSize}'>${formatAxisValue(gridValue, yLabel)}</text>`;
    }

    for (let index = 0; index <= 3; index++) {
        const labelTime = timeMin + (timeMax - timeMin) * index / 3;
        const x = paddingLeft + (labelTime - timeMin) / (timeMax - timeMin) * chartWidth;
        let anchor = "middle";
        let labelX = x;
        if (index === 0) {
            anchor = "start";
            labelX = paddingLeft + 2;
        } else if (index === 3) {
            anchor = "end";
            labelX = width - paddingRight - 2;
        }
        output += `<text x='${labelX}' y='${height - 4}' text-anchor='${anchor}' fill='#475569' font-size='${fontSize}'>${formatTracePulseClock(new Date(labelTime))}</text>`;
    }

    series.forEach((item) => {
        if (item.points.length === 0) return;
        const path = item.points.map((point, index) => `${index === 0 ? "M" : "L"}${tx(point.t).toFixed(1)} ${ty(point.v).toFixed(1)}`).join(" ");
        output += `<path d='${path}' fill='none' stroke='${item.color}' stroke-width='1.5' stroke-linejoin='round'/>`;
    });
    svg.innerHTML = output;
}

function interfaceLabel(ifIndex: number): string {
    return window.IFACE_LABELS?.[String(ifIndex)] || `if-${ifIndex}`;
}

export function renderBwChart(ifSeries: InterfaceSeries[] = []): void {
    const legend = document.getElementById("bw-legend");
    if (legend) legend.innerHTML = "";
    const ordered = [...ifSeries].sort((a, b) => a.if_index - b.if_index);
    const filtered = ordered.filter((series) => window.isInterfaceSelected?.(series.if_index));
    const chartSeries = filtered.map((series, index) => {
        const color = chartColors[index % chartColors.length];
        const label = interfaceLabel(series.if_index);
        if (legend) legend.innerHTML += `<span style='color:${color};margin-right:.5rem'>&#9632; ${escapeHtml(label)}</span>`;
        return { label, color, points: series.points.map((point) => ({ t: point.t, v: point.bw || 0 })) };
    });
    sparkline("bw-chart", chartSeries, "%");
}

export function renderErrChart(ifSeries: InterfaceSeries[] = []): void {
    const legend = document.getElementById("err-legend");
    if (legend) legend.innerHTML = "";
    const ordered = [...ifSeries].sort((a, b) => a.if_index - b.if_index);
    const chartSeries: ChartSeries[] = [];
    ordered.filter((series) => window.isInterfaceSelected?.(series.if_index)).forEach((series, index) => {
        const color = chartColors[index % chartColors.length];
        const label = interfaceLabel(series.if_index);
        if (legend && index < 4) legend.innerHTML += `<span style='color:${color};margin-right:.5rem'>&#9632; ${escapeHtml(label)}</span>`;
        chartSeries.push({
            label: `${label} in_err`,
            color,
            points: series.points.map((point) => ({ t: point.t, v: (point.in_err || 0) + (point.in_dis || 0) })),
        });
    });
    sparkline("err-chart", chartSeries, "count");
}

export function renderCpuChart(metrics: SystemMetric[] = []): void {
    sparkline("cpu-chart", [{
        label: "CPU %",
        color: "#38bdf8",
        points: metrics.filter((metric) => metric.cpu !== null && metric.cpu !== undefined && metric.cpu > 0).map((metric) => ({ t: metric.t, v: metric.cpu || 0 })),
    }], "%");
}

export function renderMemoryChart(metrics: SystemMetric[] = []): void {
    const memSeries: ChartSeries = { label: "Memory Usage (%)", color: "#a78bfa", points: [] };
    metrics.forEach((metric) => {
        if (metric && metric.memory !== null && metric.memory !== undefined && !Number.isNaN(metric.memory)) {
            memSeries.points.push({ t: metric.t, v: metric.memory });
        }
    });

    if (memSeries.points.length === 0 && metrics.length > 0) {
        const maxBytes = metrics.reduce((max, metric) => metric.memory_bytes && metric.memory_bytes > max ? metric.memory_bytes : max, 0);
        if (maxBytes > 0) {
            metrics.forEach((metric) => {
                if (metric.memory_bytes !== null && metric.memory_bytes !== undefined && metric.memory_bytes > 0) {
                    memSeries.points.push({ t: metric.t, v: Math.round(metric.memory_bytes / maxBytes * 100) });
                }
            });
        }
    }

    sparkline("memory-chart", [memSeries], "%");
}

export function renderSysChart(metrics: SystemMetric[] = []): void {
    renderCpuChart(metrics);
    renderMemoryChart(metrics);
}

function renderHeader(device: DeviceDetail): void {
    const title = document.getElementById("dev-title");
    if (title) title.textContent = `${device.name || ""} (${device.ip || ""})`;
    const status = document.getElementById("dev-status");
    if (status) {
        const classMap: Record<string, string> = { online: "status-online", offline: "status-offline", warning: "status-warning", critical: "status-critical" };
        status.className = classMap[device.status || ""] || "status-unknown";
        status.textContent = device.status || "unknown";
    }
    document.title = `TracePulse - ${device.name || device.ip || "Device"}`;
}

function updateLastRefreshed(): void {
    const element = document.getElementById("device-refresh");
    if (element) {
        element.textContent = `${t("device_auto_refresh").replace("{seconds}", String(deviceRefreshSeconds))} • ${t("updated")}: ${formatTracePulseClock(new Date())}`;
    }
}

function scheduleDeviceRefresh(): void {
    if (deviceRefreshTimer !== null) window.clearInterval(deviceRefreshTimer);
    deviceRefreshTimer = window.setInterval(loadDeviceDetail, deviceRefreshSeconds * 1000);
}

export function renderDetail(device: DeviceDetail): void {
    lastDeviceDetail = device;
    window.LAST_DEVICE_DETAIL = device;
    renderHeader(device);
    const ifaces = device.interfaces || [];
    window.IFACE_LABELS = {};
    ifaces.forEach((iface) => {
        window.IFACE_LABELS![String(iface.if_index)] = iface.if_name || `if-${iface.if_index}`;
    });
    syncSelectedInterfaces(ifaces);
    renderInterfaceSelection(ifaces);
    renderInterfaces(ifaces);
    renderErrorBreakdownCard(ifaces);
    refreshTrafficProtocols();
    renderBwChart(device.if_series || []);
    renderErrChart(device.if_series || []);
    renderSysChart(device.metrics || []);
    renderHardwareStatus(device.hardware_sensors || []);
    renderSpikes(device.spikes || []);
    renderAlerts(device.alerts || []);
    updateLastRefreshed();
}

export function loadDeviceDetail(): void {
    Promise.all([
        fetch("/api/settings", { cache: "no-store" }).then((response) => response.json() as Promise<SettingsResponse>).catch(() => null),
        fetch(`/api/device/${encodeURIComponent(window.DEVICE_IP || "")}`, { cache: "no-store" }).then((response) => response.json() as Promise<DeviceDetail>),
    ])
        .then(([settings, device]) => {
            const configuredInterval = Number(settings?.polling?.interval_seconds);
            if (Number.isFinite(configuredInterval) && configuredInterval >= 5 && configuredInterval <= 600) {
                deviceRefreshSeconds = configuredInterval;
                scheduleDeviceRefresh();
            }
            if (settings?.display?.timezone) {
                try {
                    localStorage.setItem("tracepulse-timezone", settings.display.timezone);
                } catch {
                    // Ignore unavailable localStorage.
                }
            }
            if (settings?.alert && typeof settings.alert.spike_threshold === "number") {
                window.TRACEPULSE_SPIKE_THRESHOLD = settings.alert.spike_threshold;
            }
            if (device.error) {
                const title = document.getElementById("dev-title");
                if (title) title.textContent = device.error;
                return;
            }
            renderDetail(device);
        })
        .catch((error: unknown) => {
            const title = document.getElementById("dev-title");
            if (title) title.textContent = `Error: ${String(error)}`;
        });
}

window.esc = escapeHtml;
window.fmtNum = fmtNum;
window.fmtTime = fmtTime;
window.formatBps = formatBps;
window.formatTracePulseClock = formatTracePulseClock;
window.formatTracePulseTime = formatTracePulseTime;
window.isInterfaceSelected = isInterfaceSelected;
window.loadDeviceDetail = loadDeviceDetail;
window.onErrorBreakdownIfaceChange = onErrorBreakdownIfaceChange;
window.parseTracePulseTime = parseTracePulseTime;
window.renderAlerts = renderAlerts;
window.renderBwChart = renderBwChart;
window.renderCpuChart = renderCpuChart;
window.renderErrChart = renderErrChart;
window.renderErrorBreakdownCard = renderErrorBreakdownCard;
window.renderHardwareStatus = renderHardwareStatus;
window.renderInterfaces = renderInterfaces;
window.renderDetail = renderDetail;
window.renderMemoryChart = renderMemoryChart;
window.renderSpikes = renderSpikes;
window.renderSysChart = renderSysChart;
window.renderTrafficProtocols = renderTrafficProtocols;
window.refreshTrafficProtocols = refreshTrafficProtocols;
window.selectAllInterfaces = selectAllInterfaces;
window.sparkline = sparkline;
window.toggleInterfaceSelection = toggleInterfaceSelection;
window.tracePulseTimeZone = tracePulseTimeZone;
window.applyPageLanguage = () => {
    updateLastRefreshed();
    refreshTrafficProtocols();
};

loadDeviceDetail();
scheduleDeviceRefresh();
scheduleAnalyticsPolling();
document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") {
        refreshTrafficProtocols();
        scheduleAnalyticsPolling();
    } else if (analyticsTimer !== null) {
        window.clearInterval(analyticsTimer);
        analyticsTimer = null;
    }
});
window.addEventListener("resize", () => {
    if (resizeTimer !== null) window.clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(() => {
        if (lastDeviceDetail) renderDetail(lastDeviceDetail);
    }, 200);
});
