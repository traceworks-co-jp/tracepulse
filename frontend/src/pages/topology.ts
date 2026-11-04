import { TopologyMap, topologyRiskColor, type TopologySeverity, type TopologyMapNode, type TopologyMapEdge } from "../shared/topology-map";

interface TopologyInterface {
    if_index?: number;
    if_name?: string;
    link_status?: string;
    health_status?: string;
    predictive_warning?: boolean;
    metrics?: { in_errors_delta?: number; out_errors_delta?: number; in_discards_delta?: number; out_discards_delta?: number; late_collisions_delta?: number };
}

interface TopologyNode extends TopologyMapNode {
    device_id?: number;
    id: string;
    label?: string;
    ip?: string;
    hostname?: string;
    kind?: string;
    status?: string;
    seed?: boolean;
    interfaces?: TopologyInterface[];
}

interface TopologyEdge extends TopologyMapEdge {
    link_status?: string;
    health?: { severity: TopologySeverity; stale: boolean };
    source_ip?: string;
    source_port?: string | null;
    source_if_index?: number | null;
    target_ip?: string | null;
    target_port?: string | null;
    neighbor_name?: string | null;
    id?: string;
    source: string;
    target: string;
    label?: string;
    protocol?: string;
    health_status?: string;
    local_ip?: string;
    local_port?: string;
    local_if_index?: number;
    remote_ip?: string;
    remote_port?: string;
    remote_hostname?: string;
}

interface TopologyData {
    seed_ip?: string;
    nodes?: TopologyNode[];
    edges?: TopologyEdge[];
    warnings?: string[];
    total_nodes?: number;
    hidden_nodes?: number;
    progress?: TopologyProgress;
}

interface TopologyProgress {
    state: string;
    total: number;
    completed: number;
    running: number;
    queued: number;
    partial: number;
    retry_after_seconds: number;
    last_activity_at: string | null;
    devices: {
        device_id: number; name: string; ip: string; state: string; updated_at: string | null;
        checks?: { scope: string; status: string; reason_code: string }[]
    }[];
}
let topologyGeneration = 0;
let topologyController: AbortController | null = null;
let topologyBusy = false;
let topologyCooldownTimer: number | null = null;
let topologyProgressView: { progress: TopologyProgress; started: number; changed: number; retryAt: number } | null = null;
const topologyState: { data: TopologyData | null } = { data: null };
let map: TopologyMap<TopologyNode, TopologyEdge> | null = null;

function t(key: string): string { return typeof window.t === "function" ? window.t(key) : key; }
function tf(key: string, params: Record<string, string | number>): string { return typeof window.tf === "function" ? window.tf(key, params) : key; }
function esc(value: unknown): string { return window.esc ? window.esc(value) : String(value ?? ""); }

function setPlaceholder(key: string | null, params: Record<string, string | number> = {}, text: string | null = null): void {
    const box = document.getElementById("topology-empty");
    if (!box) return;
    box.textContent = text || (key ? tf(key, params) : "");
    box.style.display = key || text ? "flex" : "none";
}

function drawTopology(): void {
    const svg = document.getElementById("topology-canvas") as SVGSVGElement | null;
    const data = topologyState.data;
    if (!svg || !data) return;
    if (!map) map = new TopologyMap(svg, { onNodeSelect: openNodeInspector, onEdgeSelect: openEdgeInspector, onBackground: closeInspector });
    map.render((data.nodes || []).map(node => ({
        ...node, seed: node.seed || node.ip === data.seed_ip,
        kind: node.kind || (node.device_id !== undefined ? "switch" : "endpoint"),
        stale: node.stale || node.status === "stale", stateLabel: node.stateLabel || (node.health ? `${t(`topology_risk_${node.health.severity || "unknown"}`)}${node.health.stale ? ` / ${t("topology_risk_stale")}` : ""}` : undefined),
    })), (data.edges || []).map(edge => ({
        ...edge, stale: edge.stale || edge.link_status === "stale",
        stroke: edge.stroke || (edge.health ? topologyRiskColor(edge.health.severity) : undefined),
        label: edge.label || (edge.source_port || edge.target_port ? `${edge.source_port || "?"} ↔ ${edge.target_port || "?"}` : ""),
        local_ip: edge.local_ip || edge.source_ip, local_port: edge.local_port || edge.source_port || undefined,
        local_if_index: edge.local_if_index ?? edge.source_if_index ?? undefined,
        remote_ip: edge.remote_ip || edge.target_ip || undefined, remote_port: edge.remote_port || edge.target_port || undefined,
        remote_hostname: edge.remote_hostname || edge.neighbor_name || undefined,
    })));
}

function clearSelection(): void { map?.select(null); }
function clearInspector(): void { document.getElementById("topology-inspector")?.classList.remove("open"); }
function closeInspector(): void { clearInspector(); clearSelection(); }
function row(label: unknown, value: unknown, valueClass = ""): string { return `<div class='inspector-row'><span>${esc(label)}</span><span class='${valueClass}'>${esc(value || "-")}</span></div>`; }
function inspectorStateClass(state: string | undefined): string {
    const normalized = state?.trim().toLowerCase();
    const tone = normalized === "online" || normalized === "up" ? "good" : normalized === "offline" || normalized === "down" ? "bad" : "unknown";
    return `inspector-state ${tone}`;
}
function openInspector(title: string, html: string): void { const titleBox = document.getElementById("inspector-title"); const body = document.getElementById("inspector-body"); if (titleBox) titleBox.textContent = title; if (body) body.innerHTML = html; document.getElementById("topology-inspector")?.classList.add("open"); }
function openNodeInspector(node: TopologyNode): void {
    let html = row(t("ip_address"), node.ip) + row(t("hostname"), node.hostname || node.label) + row(t("status"), node.stateLabel || node.status, inspectorStateClass(node.status));
    (node.interfaces || []).forEach((iface) => {
        html += `<div class='inspector-if'><span>${esc(iface.if_name || `if-${iface.if_index}`)}</span><span class='${inspectorStateClass(iface.link_status)}'>${esc(iface.link_status || "-")}</span></div>`;
    });
    openInspector(node.label || node.ip || node.id, html);
}
function openEdgeInspector(edge: TopologyEdge): void { const localPort = edge.local_port || (edge.local_if_index ? `if-${edge.local_if_index}` : ""); const html = row(t("topology_protocol"), edge.protocol) + row(t("topology_device"), edge.local_ip) + row(t("topology_port"), localPort) + row(t("topology_device"), edge.remote_hostname || edge.remote_ip) + row(t("ip_address"), edge.remote_ip) + row(t("topology_port"), edge.remote_port); openInspector(edge.label || t("topology_link"), html); }

export function topoZoom(factor: number, clientX?: number, clientY?: number): void { map?.zoom(factor, clientX, clientY); }
export function topoFit(): void { map?.fit(); }
export function topoRelayout(): void { map?.relayout(); }
export function refreshTopologyTexts(): void { if (topologyState.data) drawTopology(); renderCollectionProgress(); }
function renderTopologySummary(data: TopologyData): void { const result = document.getElementById("topology-result"); if (!result) return; const summary = `<div class='topology-edge'>${esc(tf("topology_summary", { nodes: data.total_nodes || data.nodes?.length || 0, edges: data.edges?.length || 0, ip: data.seed_ip || "-" }))}</div>`; result.innerHTML = summary + (data.warnings || []).map((warning) => `<div class='topology-edge'>! ${esc(warning)}</div>`).join(""); }
function updateHiddenIndicator(data: TopologyData): void { const box = document.getElementById("topology-hidden"); if (!box) return; const hidden = data.hidden_nodes || 0; box.textContent = hidden ? tf("topology_hidden_nodes", { count: hidden }) : ""; box.style.display = hidden ? "block" : "none"; }
function topologyButton(): HTMLButtonElement | null {
    return document.querySelector<HTMLButtonElement>("button[onclick*='runTopologyOnly']");
}
function collectionBox(): HTMLElement | null {
    let box = document.getElementById("topology-progress");
    if (!box) {
        const section = document.getElementById("topology-section");
        if (!section) return null;
        box = document.createElement("div"); box.id = "topology-progress";
        box.style.cssText = "padding:12px 0;line-height:1.7;overflow-wrap:anywhere";
        section.prepend(box);
    }
    return box;
}
function renderCollectionProgress(): void {
    if (!topologyProgressView) return;
    const box = collectionBox(); if (!box) return;
    const { progress, started, changed, retryAt } = topologyProgressView;
    const expanded = box.querySelector("details")?.open ?? false;
    box.replaceChildren();
    const label = document.createElement("p"); label.setAttribute("role", "status"); label.setAttribute("aria-live", "polite");
    label.textContent = `${t(`topology_progress_${progress.state}`)} · ${tf("topology_progress_counts", { completed: progress.completed, total: progress.total, running: progress.running, queued: progress.queued })} · ${tf("topology_progress_elapsed", { seconds: Math.floor((Date.now() - started) / 1000) })}`;
    const bar = document.createElement("progress"); bar.max = Math.max(1, progress.total); bar.value = progress.completed;
    bar.setAttribute("aria-label", t("topology_progress_label")); bar.style.cssText = "display:block;width:100%;height:12px;accent-color:#2dd4bf;margin:8px 0";
    box.append(label, bar);
    if (topologyBusy && Date.now() - changed >= 30000) {
        const warning = document.createElement("p"); warning.setAttribute("role", "status"); warning.textContent = t("topology_progress_waiting"); box.append(warning);
    }
    if (progress.last_activity_at) {
        const updated = document.createElement("p"); updated.textContent = `${t("topology_progress_updated")}: ${new Date(progress.last_activity_at).toLocaleString()}`; box.append(updated);
    }
    const remaining = Math.max(0, Math.ceil((retryAt - Date.now()) / 1000));
    if (remaining > 0) { const cooldown = document.createElement("p"); cooldown.textContent = tf("topology_progress_cooldown", { seconds: remaining }); box.append(cooldown); }
    const details = document.createElement("details"); details.open = expanded;
    const summary = document.createElement("summary"); summary.textContent = t("topology_progress_devices"); details.append(summary);
    const list = document.createElement("ul"); list.style.cssText = "padding-left:20px;max-height:180px;overflow:auto";
    for (const device of progress.devices) {
        const item = document.createElement("li"); item.textContent = `${device.name || device.ip} (${device.ip}) — ${t(`topology_progress_${device.state}`)}`;
        if (device.checks?.length) {
            const checks = document.createElement("ul");
            for (const check of device.checks) {
                const detail = document.createElement("li");
                const scopeKey = `topology_scope_${check.scope}`;
                detail.textContent = `${t(scopeKey) === scopeKey ? t("topology_scope_other") : t(scopeKey)}: ${t(`topology_check_${check.reason_code}`)}`;
                checks.append(detail);
            }
            item.append(checks);
        }
        list.append(item);
    }
    details.append(list); box.append(details);
    const button = topologyButton(); if (button) button.disabled = topologyBusy || remaining > 0;
}
function displayTopology(data: TopologyData): void {
    if (!data.nodes?.length && topologyState.data?.nodes?.length) return;
    topologyState.data = data; renderTopologySummary(data); updateHiddenIndicator(data);
    setPlaceholder(data.nodes?.length ? null : data.progress?.queued || data.progress?.running ? "topology_progress_queued" : "topology_no_neighbors", { ip: data.seed_ip || "-" });
    drawTopology();
}
export async function startTopologyDiscovery(seedIp: string, community: string): Promise<void> {
    const section = document.getElementById("topology-section"); const result = document.getElementById("topology-result");
    if (!section || !result) return;
    const generation = ++topologyGeneration;
    topologyController?.abort();
    const controller = new AbortController(); topologyController = controller;
    if (topologyCooldownTimer !== null) window.clearInterval(topologyCooldownTimer);
    topologyProgressView = null; topologyBusy = true; section.style.display = "block";
    const button = topologyButton(); if (button) button.disabled = true;
    const box = collectionBox(); if (box) { box.textContent = t("topology_progress_starting"); box.setAttribute("role", "status"); }
    if (!topologyState.data?.nodes?.length) setPlaceholder("topology_running", { ip: seedIp });
    const started = Date.now(); let changed = started; let signature = "";
    const timeout = window.setTimeout(() => controller.abort(), 300000);
    const ticker = window.setInterval(renderCollectionProgress, 1000);
    try {
        let response = await fetch("/api/discovery/topology", {
            method: "POST", headers: { "Content-Type": "application/x-www-form-urlencoded", "X-CSRF-Token": window.tracepulseCsrf || "" },
            body: `seed_ip=${encodeURIComponent(seedIp)}&community=${encodeURIComponent(community)}`, signal: controller.signal,
        });
        if (response.status === 429) response = await fetch("/api/topology", { cache: "no-store", signal: controller.signal });
        while (true) {
            const data = await response.json() as TopologyData & { error?: string };
            if (generation !== topologyGeneration) return;
            if (!response.ok || data.error) throw new Error("topology status unavailable");
            displayTopology(data);
            const progress = data.progress;
            if (!progress) break;
            const next = JSON.stringify([progress.completed, progress.running, progress.queued, progress.devices, progress.last_activity_at]);
            if (next !== signature) { signature = next; changed = Date.now(); }
            topologyProgressView = { progress, started, changed, retryAt: Date.now() + progress.retry_after_seconds * 1000 };
            renderCollectionProgress();
            if (!progress.running && !progress.queued) break;
            await new Promise(resolve => window.setTimeout(resolve, 1500));
            if (controller.signal.aborted) throw new Error("topology status timed out");
            response = await fetch("/api/topology", { cache: "no-store", signal: controller.signal });
        }
        if (!topologyProgressView && box) box.replaceChildren();
    } catch {
        if (generation !== topologyGeneration) return;
        const message = controller.signal.aborted ? t("topology_progress_timeout") : t("topology_progress_failed");
        if (box) box.textContent = message;
        if (!topologyState.data?.nodes?.length) setPlaceholder(null, {}, message);
        topologyProgressView = null;
    } finally {
        window.clearTimeout(timeout);
        window.clearInterval(ticker);
        if (generation === topologyGeneration) {
            topologyBusy = false; topologyController = null;
            if (button) button.disabled = false;
            renderCollectionProgress();
            if (topologyProgressView) topologyCooldownTimer = window.setInterval(() => {
                renderCollectionProgress();
                if (topologyProgressView && Date.now() >= topologyProgressView.retryAt && topologyCooldownTimer !== null) {
                    window.clearInterval(topologyCooldownTimer); topologyCooldownTimer = null;
                }
            }, 1000);
        }
    }
}

function csvCell(value: unknown): string { return `"${String(value ?? "").replace(/"/g, '""')}"`; }
export function exportTopology(format: string): void { if (!topologyState.data) return; const data = topologyState.data; const rows: unknown[][] = [["record_type", "id", "label", "ip", "hostname", "kind_or_protocol", "status", "source", "source_port", "target", "target_port"]]; (data.nodes || []).forEach((node) => rows.push(["node", node.id, node.label, node.ip, node.hostname, node.kind, node.status, "", "", "", ""])); (data.edges || []).forEach((edge) => rows.push(["edge", edge.id, edge.label, edge.remote_ip, edge.remote_hostname, edge.protocol, "", edge.source, edge.local_port || (edge.local_if_index ? `if-${edge.local_if_index}` : ""), edge.target, edge.remote_port])); const content = format === "csv" ? rows.map((row) => row.map(csvCell).join(",")).join("\r\n") : JSON.stringify(data, null, 2); const blob = new Blob([content], { type: format === "csv" ? "text/csv" : "application/json" }); const link = document.createElement("a"); link.href = URL.createObjectURL(blob); link.download = `tracepulse-topology-${Date.now()}.${format}`; link.click(); URL.revokeObjectURL(link.href); }
window.closeInspector = closeInspector; window.exportTopology = exportTopology; window.refreshTopologyTexts = refreshTopologyTexts; window.startTopologyDiscovery = startTopologyDiscovery; window.topoFit = topoFit; window.topoRelayout = topoRelayout; window.topoZoom = topoZoom;
