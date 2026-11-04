"use strict";
var TracePulseTopology = (() => {
  var __defProp = Object.defineProperty;
  var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __hasOwnProp = Object.prototype.hasOwnProperty;
  var __export = (target, all) => {
    for (var name in all)
      __defProp(target, name, { get: all[name], enumerable: true });
  };
  var __copyProps = (to, from, except, desc) => {
    if (from && typeof from === "object" || typeof from === "function") {
      for (let key of __getOwnPropNames(from))
        if (!__hasOwnProp.call(to, key) && key !== except)
          __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
    }
    return to;
  };
  var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

  // frontend/src/pages/topology.ts
  var topology_exports = {};
  __export(topology_exports, {
    exportTopology: () => exportTopology,
    refreshTopologyTexts: () => refreshTopologyTexts,
    startTopologyDiscovery: () => startTopologyDiscovery,
    topoFit: () => topoFit,
    topoRelayout: () => topoRelayout,
    topoZoom: () => topoZoom
  });

  // frontend/src/shared/topology-map.ts
  var topologyRiskColor = (severity) => ({
    critical: "#ef4444",
    warning: "#fbbf24",
    stale: "#94a3b8",
    healthy: "#34d399",
    unknown: "#94a3b8"
  })[severity];
  var styles = `
svg.topology-map{display:block;width:100%;height:100%;min-height:300px;touch-action:none;--map-fill:#152337;--map-text:#eef5fb;--map-muted:#b0becf;--map-border:#94a3b8}
html[data-theme=light] svg.topology-map{--map-fill:#fff;--map-text:#172b3a;--map-muted:#4e6478;--map-border:#64748b}
.topology-map .topo-node{cursor:grab}.topology-map .topo-node:focus{outline:none}
.topology-map .topo-node .topo-node-shape{fill:var(--map-fill);stroke:var(--node-stroke,var(--map-border));stroke-width:2}
.topology-map .topo-node.selected .topo-node-shape,.topology-map .topo-node:focus-visible .topo-node-shape{stroke:var(--node-stroke,var(--map-border));stroke-width:4;filter:drop-shadow(0 0 3px var(--node-stroke,var(--map-border)))}
.topology-map .topo-node.is-stale .topo-node-shape{stroke-dasharray:7 5}
.topology-map .topo-node-pulse{fill:none;stroke:var(--node-stroke,#ef4444);stroke-width:6;animation:topology-map-pulse 2.8s ease-in-out infinite alternate;pointer-events:none}
.topology-map .topo-node.is-stale .topo-node-pulse{stroke-dasharray:7 5}
@keyframes topology-map-pulse{from{opacity:.12}to{opacity:.5}}
@media(prefers-reduced-motion:reduce){.topology-map .topo-node-pulse{animation:none;opacity:.2}}
.topology-map .topo-node-label{fill:var(--map-text);font-size:13px;text-anchor:middle;pointer-events:none}
.topology-map .topo-node-sub,.topology-map .topo-node-state,.topology-map .topo-node-badge{fill:var(--map-muted);font-size:11px;text-anchor:middle;pointer-events:none}
.topology-map .topo-node-state{fill:var(--node-stroke,var(--map-muted));font-weight:600}
html[data-theme=light] .topology-map .topo-node-state{fill:var(--map-text)}
.topology-map .topo-edge .topo-edge-line{stroke:var(--edge-stroke,#71869c);stroke-width:var(--edge-width,2)}
.topology-map .topo-edge.is-stale .topo-edge-line,.topology-map .topo-edge.cdp .topo-edge-line{stroke-dasharray:7 5}
.topology-map .topo-edge.selected .topo-edge-line{stroke:var(--edge-stroke,#71869c);stroke-width:var(--edge-width,4)}
.topology-map .topo-edge-hit{stroke:transparent;stroke-width:14;cursor:pointer}
.topology-map .topo-edge-label{fill:var(--map-muted);font-size:10px;text-anchor:middle;pointer-events:none}
.topology-map .topology-ghost{opacity:.13}.topology-map .topology-context{opacity:.65}
.topology-map .topology-ghost:focus,.topology-map .topology-context:focus{opacity:1}
`;
  function svgElement(tag) {
    return document.createElementNS("http://www.w3.org/2000/svg", tag);
  }
  function widthFor(node) {
    return Math.min(280, Math.max(128, (node.label || node.ip || node.id).length * 7.4 + 28, (node.stateLabel?.length || 0) * 11 + 28));
  }
  var TopologyMap = class {
    constructor(svg, options = {}) {
      this.svg = svg;
      this.options = options;
      this.nodes = [];
      this.edges = [];
      this.positions = /* @__PURE__ */ new Map();
      this.scale = 1;
      this.tx = 0;
      this.ty = 0;
      this.selected = null;
      this.root = null;
      this.drag = null;
      this.pan = null;
      this.suppressClick = false;
      this.controller = new AbortController();
      if (!document.getElementById("topology-map-styles")) {
        const style = document.createElement("style");
        style.id = "topology-map-styles";
        style.textContent = styles;
        document.head.append(style);
      }
      svg.classList.add("topology-map");
      svg.setAttribute("role", "group");
      const signal = this.controller.signal;
      svg.addEventListener("wheel", (event) => {
        event.preventDefault();
        this.zoom(event.deltaY < 0 ? 1.1 : 0.9, event.clientX, event.clientY);
      }, { passive: false, signal });
      svg.addEventListener("pointerdown", (event) => {
        if (event.button !== 0 || !event.isPrimary || event.target !== svg) return;
        svg.setPointerCapture(event.pointerId);
        this.pan = { x: event.clientX, y: event.clientY, tx: this.tx, ty: this.ty };
      }, { signal });
      svg.addEventListener("click", (event) => {
        if (this.suppressClick) {
          this.suppressClick = false;
          return;
        }
        if (event.target === svg) {
          this.select(null);
          this.options.onBackground?.();
        }
      }, { signal });
      window.addEventListener("pointermove", (event) => {
        if (!event.isPrimary) return;
        if (this.drag) {
          const deltaX = event.clientX - this.drag.x, deltaY = event.clientY - this.drag.y;
          if (Math.hypot(deltaX, deltaY) > 3) this.drag.moved = true;
          this.positions.set(this.drag.id, { x: this.drag.origin.x + deltaX / this.scale, y: this.drag.origin.y + deltaY / this.scale });
          this.draw();
        } else if (this.pan) {
          this.tx = this.pan.tx + event.clientX - this.pan.x;
          this.ty = this.pan.ty + event.clientY - this.pan.y;
          this.transform();
        }
      }, { signal });
      const release = (event) => {
        if (!event.isPrimary) return;
        const drag = this.drag;
        this.suppressClick = !!drag;
        this.drag = null;
        this.pan = null;
        if (svg.hasPointerCapture(event.pointerId)) svg.releasePointerCapture(event.pointerId);
        if (drag && !drag.moved && event.type === "pointerup") {
          const node = this.nodes.find((item) => item.id === drag.id);
          if (node) {
            this.svg.querySelectorAll("[data-node]").forEach((group) => {
              if (group.dataset.node === node.id) group.focus();
            });
            this.select(node.id);
            this.options.onNodeSelect?.(node);
          }
        }
      };
      window.addEventListener("pointerup", release, { signal });
      window.addEventListener("pointercancel", release, { signal });
    }
    render(nodes, edges, preserveLayout = true) {
      const first = this.nodes.length === 0;
      this.nodes = nodes;
      this.edges = edges;
      const layout = this.layout();
      for (const node of nodes) if (!preserveLayout || !this.positions.has(node.id)) this.positions.set(node.id, layout.get(node.id));
      const focused = this.svg.contains(document.activeElement) ? document.activeElement.dataset.node : void 0;
      this.draw();
      if (first || !preserveLayout) this.fit();
      if (focused) this.svg.querySelectorAll("[data-node]").forEach((group) => {
        if (group.dataset.node === focused) group.focus();
      });
    }
    layout() {
      const adjacency = new Map(this.nodes.map((node) => [node.id, []]));
      for (const edge of this.edges) {
        adjacency.get(edge.source)?.push(edge.target);
        adjacency.get(edge.target)?.push(edge.source);
      }
      const seed = this.nodes.find((node) => node.seed) || this.nodes[0];
      const depth = /* @__PURE__ */ new Map();
      if (seed) {
        depth.set(seed.id, 0);
        const queue = [seed.id];
        for (let index = 0; index < queue.length; index++) for (const next of adjacency.get(queue[index]) || []) {
          if (!depth.has(next)) {
            depth.set(next, depth.get(queue[index]) + 1);
            queue.push(next);
          }
        }
      }
      const layers = /* @__PURE__ */ new Map();
      for (const node of this.nodes) {
        const level = depth.get(node.id) ?? 1;
        const layer = layers.get(level) || [];
        layer.push(node.id);
        layers.set(level, layer);
      }
      const positions = /* @__PURE__ */ new Map();
      const gap = Math.max(200, ...this.nodes.map((node) => widthFor(node) + 36));
      let nextY = 0;
      for (const [, ids] of [...layers].sort(([first], [second]) => first - second)) {
        const perRow = Math.min(ids.length, Math.max(4, Math.ceil(Math.sqrt(ids.length * 2))));
        ids.forEach((id, index) => {
          const row2 = Math.floor(index / perRow), columns = Math.min(perRow, ids.length - row2 * perRow);
          positions.set(id, { x: (index % perRow - (columns - 1) / 2) * gap, y: nextY + row2 * 120 });
        });
        nextY += Math.ceil(ids.length / perRow) * 120 + 30;
      }
      return positions;
    }
    draw() {
      this.svg.replaceChildren();
      this.root = svgElement("g");
      this.svg.append(this.root);
      const edges = svgElement("g"), nodes = svgElement("g");
      this.root.append(edges, nodes);
      for (const edge of this.edges) {
        const from = this.positions.get(edge.source), to = this.positions.get(edge.target);
        if (!from || !to) continue;
        const group = svgElement("g");
        group.setAttribute("class", `topo-edge${edge.stale ? " is-stale" : ""}${edge.protocol?.includes("CDP") ? " cdp" : ""} ${edge.className || ""}`);
        const stroke = edge.stroke || (edge.health_status === "Critical" ? topologyRiskColor("critical") : edge.health_status === "Warning" ? topologyRiskColor("warning") : void 0);
        if (stroke) group.style.setProperty("--edge-stroke", stroke);
        const line = svgElement("line"), hit = svgElement("line");
        line.setAttribute("class", "topo-edge-line");
        hit.setAttribute("class", "topo-edge-hit");
        if (edge.strokeWidth !== void 0 && Number.isFinite(edge.strokeWidth)) {
          group.style.setProperty("--edge-width", String(Math.min(8, Math.max(1.5, edge.strokeWidth))));
          for (const item of [line, hit]) item.setAttribute("vector-effect", "non-scaling-stroke");
        }
        for (const item of [line, hit]) {
          item.setAttribute("x1", String(from.x));
          item.setAttribute("y1", String(from.y));
          item.setAttribute("x2", String(to.x));
          item.setAttribute("y2", String(to.y));
        }
        const label = svgElement("text");
        label.setAttribute("class", "topo-edge-label");
        label.textContent = edge.label || "";
        label.setAttribute("x", String(from.x + (to.x - from.x) * 0.68));
        label.setAttribute("y", String(from.y + (to.y - from.y) * 0.68 - 8));
        const title = svgElement("title");
        title.textContent = edge.tooltip || edge.label || edge.protocol || "";
        group.append(title, line, hit, label);
        edges.append(group);
        hit.addEventListener("click", (event) => {
          event.stopPropagation();
          this.select(null);
          this.svg.querySelectorAll(".topo-edge.selected").forEach((item) => item.classList.remove("selected"));
          group.classList.add("selected");
          this.options.onEdgeSelect?.(edge);
        });
      }
      for (const node of this.nodes) {
        const position = this.positions.get(node.id);
        if (!position) continue;
        const group = svgElement("g");
        group.dataset.node = node.id;
        group.setAttribute("class", `topo-node ${node.kind === "switch" ? "switch" : "endpoint"}${node.stale || node.health?.stale ? " is-stale" : ""}${this.selected === node.id ? " selected" : ""} ${node.className || ""}`);
        group.setAttribute("transform", `translate(${position.x},${position.y})`);
        group.setAttribute("role", "button");
        group.setAttribute("tabindex", "0");
        group.setAttribute("aria-pressed", String(this.selected === node.id));
        group.setAttribute("aria-label", node.tooltip || `${node.label || node.id} ${node.stateLabel || ""}`);
        const stroke = node.stroke || (node.health ? topologyRiskColor(node.health.severity || "unknown") : node.status?.toLowerCase() === "offline" ? topologyRiskColor("critical") : void 0);
        if (stroke) group.style.setProperty("--node-stroke", stroke);
        const width = widthFor(node), height = node.badgeLabel ? 82 : node.stateLabel ? 66 : 44;
        const shape = node.kind === "switch" ? svgElement("rect") : svgElement("circle");
        shape.setAttribute("class", "topo-node-shape");
        if (shape instanceof SVGRectElement) {
          shape.setAttribute("x", String(-width / 2));
          shape.setAttribute("y", String(-height / 2));
          shape.setAttribute("width", String(width));
          shape.setAttribute("height", String(height));
          shape.setAttribute("rx", "6");
        } else shape.setAttribute("r", String(Math.max(height / 2, width / 3.2)));
        const title = svgElement("title");
        title.textContent = node.tooltip || `${node.label || node.id} (${node.ip || ""})`;
        group.append(title);
        if (node.health?.severity === "critical" && !node.className?.split(/\s+/).includes("is-muted")) {
          const pulse = shape.cloneNode();
          pulse.setAttribute("class", "topo-node-pulse");
          group.append(pulse);
        }
        group.append(shape);
        for (const [className, content, offset] of [["topo-node-label", node.label || node.ip || node.id, -height / 2 + 18], ["topo-node-sub", node.ip || "", -height / 2 + 34], ["topo-node-state", node.stateLabel || "", -height / 2 + 50], ["topo-node-badge", node.badgeLabel || "", -height / 2 + 66]]) {
          if (!content) continue;
          const label = svgElement("text");
          label.setAttribute("class", className);
          label.setAttribute("y", String(offset));
          const limit = Math.max(12, Math.floor((width - 20) / (className === "topo-node-label" ? 7.4 : 8)));
          label.textContent = content.length > limit ? `${content.slice(0, limit - 3)}...` : content;
          group.append(label);
        }
        nodes.append(group);
        const choose = () => {
          if (this.suppressClick) {
            this.suppressClick = false;
            return;
          }
          group.focus();
          this.select(node.id);
          this.options.onNodeSelect?.(node);
        };
        group.addEventListener("pointerdown", (event) => {
          if (event.button !== 0 || !event.isPrimary) return;
          event.stopPropagation();
          this.svg.setPointerCapture(event.pointerId);
          this.suppressClick = false;
          this.drag = { id: node.id, x: event.clientX, y: event.clientY, origin: { ...position }, moved: false };
        });
        group.addEventListener("click", (event) => {
          event.stopPropagation();
          choose();
        });
        group.addEventListener("keydown", (event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            this.suppressClick = false;
            choose();
          }
        });
        group.addEventListener("focus", () => this.options.onNodeFocus?.(node));
        group.addEventListener("pointerenter", (event) => {
          if (event.pointerType === "mouse" && !this.svg.contains(document.activeElement)) this.options.onNodeHover?.(node);
        });
      }
      this.transform();
    }
    select(id) {
      this.selected = id;
      this.svg.querySelectorAll("[data-node]").forEach((group) => {
        const selected = group.dataset.node === id;
        group.classList.toggle("selected", selected);
        group.setAttribute("aria-pressed", String(selected));
      });
    }
    transform() {
      this.root?.setAttribute("transform", `translate(${this.tx},${this.ty}) scale(${this.scale})`);
    }
    zoom(factor, clientX, clientY) {
      const bounds = this.svg.getBoundingClientRect(), next = Math.min(4, Math.max(0.05, this.scale * factor)), ratio = next / this.scale;
      const positionX = clientX === void 0 ? bounds.width / 2 : clientX - bounds.left, positionY = clientY === void 0 ? bounds.height / 2 : clientY - bounds.top;
      this.tx = positionX - (positionX - this.tx) * ratio;
      this.ty = positionY - (positionY - this.ty) * ratio;
      this.scale = next;
      this.transform();
    }
    fit() {
      const points = this.nodes.map((node) => this.positions.get(node.id)).filter((position) => !!position);
      if (!points.length) return;
      const bounds = this.svg.getBoundingClientRect(), positionsX = points.map((point) => point.x), positionsY = points.map((point) => point.y);
      const minX = Math.min(...positionsX), maxX = Math.max(...positionsX), minY = Math.min(...positionsY), maxY = Math.max(...positionsY);
      const padding = Math.max(220, ...this.nodes.map((node) => widthFor(node) + 48));
      this.scale = Math.min(2, Math.max(0.05, Math.min(bounds.width / (maxX - minX + padding), bounds.height / (maxY - minY + 160))));
      this.tx = bounds.width / 2 - (minX + maxX) / 2 * this.scale;
      this.ty = bounds.height / 2 - (minY + maxY) / 2 * this.scale;
      this.transform();
    }
    relayout() {
      this.positions = this.layout();
      this.draw();
      this.fit();
    }
    destroy() {
      this.controller.abort();
      this.svg.replaceChildren();
    }
  };

  // frontend/src/pages/topology.ts
  var topologyGeneration = 0;
  var topologyController = null;
  var topologyBusy = false;
  var topologyCooldownTimer = null;
  var topologyProgressView = null;
  var topologyState = { data: null };
  var map = null;
  function t(key) {
    return typeof window.t === "function" ? window.t(key) : key;
  }
  function tf(key, params) {
    return typeof window.tf === "function" ? window.tf(key, params) : key;
  }
  function esc(value) {
    return window.esc ? window.esc(value) : String(value ?? "");
  }
  function setPlaceholder(key, params = {}, text = null) {
    const box = document.getElementById("topology-empty");
    if (!box) return;
    box.textContent = text || (key ? tf(key, params) : "");
    box.style.display = key || text ? "flex" : "none";
  }
  function drawTopology() {
    const svg = document.getElementById("topology-canvas");
    const data = topologyState.data;
    if (!svg || !data) return;
    if (!map) map = new TopologyMap(svg, { onNodeSelect: openNodeInspector, onEdgeSelect: openEdgeInspector, onBackground: closeInspector });
    map.render((data.nodes || []).map((node) => ({
      ...node,
      seed: node.seed || node.ip === data.seed_ip,
      kind: node.kind || (node.device_id !== void 0 ? "switch" : "endpoint"),
      stale: node.stale || node.status === "stale",
      stateLabel: node.stateLabel || (node.health ? `${t(`topology_risk_${node.health.severity || "unknown"}`)}${node.health.stale ? ` / ${t("topology_risk_stale")}` : ""}` : void 0)
    })), (data.edges || []).map((edge) => ({
      ...edge,
      stale: edge.stale || edge.link_status === "stale",
      stroke: edge.stroke || (edge.health ? topologyRiskColor(edge.health.severity) : void 0),
      label: edge.label || (edge.source_port || edge.target_port ? `${edge.source_port || "?"} \u2194 ${edge.target_port || "?"}` : ""),
      local_ip: edge.local_ip || edge.source_ip,
      local_port: edge.local_port || edge.source_port || void 0,
      local_if_index: edge.local_if_index ?? edge.source_if_index ?? void 0,
      remote_ip: edge.remote_ip || edge.target_ip || void 0,
      remote_port: edge.remote_port || edge.target_port || void 0,
      remote_hostname: edge.remote_hostname || edge.neighbor_name || void 0
    })));
  }
  function clearSelection() {
    map?.select(null);
  }
  function clearInspector() {
    document.getElementById("topology-inspector")?.classList.remove("open");
  }
  function closeInspector() {
    clearInspector();
    clearSelection();
  }
  function row(label, value, valueClass = "") {
    return `<div class='inspector-row'><span>${esc(label)}</span><span class='${valueClass}'>${esc(value || "-")}</span></div>`;
  }
  function inspectorStateClass(state) {
    const normalized = state?.trim().toLowerCase();
    const tone = normalized === "online" || normalized === "up" ? "good" : normalized === "offline" || normalized === "down" ? "bad" : "unknown";
    return `inspector-state ${tone}`;
  }
  function openInspector(title, html) {
    const titleBox = document.getElementById("inspector-title");
    const body = document.getElementById("inspector-body");
    if (titleBox) titleBox.textContent = title;
    if (body) body.innerHTML = html;
    document.getElementById("topology-inspector")?.classList.add("open");
  }
  function openNodeInspector(node) {
    let html = row(t("ip_address"), node.ip) + row(t("hostname"), node.hostname || node.label) + row(t("status"), node.stateLabel || node.status, inspectorStateClass(node.status));
    (node.interfaces || []).forEach((iface) => {
      html += `<div class='inspector-if'><span>${esc(iface.if_name || `if-${iface.if_index}`)}</span><span class='${inspectorStateClass(iface.link_status)}'>${esc(iface.link_status || "-")}</span></div>`;
    });
    openInspector(node.label || node.ip || node.id, html);
  }
  function openEdgeInspector(edge) {
    const localPort = edge.local_port || (edge.local_if_index ? `if-${edge.local_if_index}` : "");
    const html = row(t("topology_protocol"), edge.protocol) + row(t("topology_device"), edge.local_ip) + row(t("topology_port"), localPort) + row(t("topology_device"), edge.remote_hostname || edge.remote_ip) + row(t("ip_address"), edge.remote_ip) + row(t("topology_port"), edge.remote_port);
    openInspector(edge.label || t("topology_link"), html);
  }
  function topoZoom(factor, clientX, clientY) {
    map?.zoom(factor, clientX, clientY);
  }
  function topoFit() {
    map?.fit();
  }
  function topoRelayout() {
    map?.relayout();
  }
  function refreshTopologyTexts() {
    if (topologyState.data) drawTopology();
    renderCollectionProgress();
  }
  function renderTopologySummary(data) {
    const result = document.getElementById("topology-result");
    if (!result) return;
    const summary = `<div class='topology-edge'>${esc(tf("topology_summary", { nodes: data.total_nodes || data.nodes?.length || 0, edges: data.edges?.length || 0, ip: data.seed_ip || "-" }))}</div>`;
    result.innerHTML = summary + (data.warnings || []).map((warning) => `<div class='topology-edge'>! ${esc(warning)}</div>`).join("");
  }
  function updateHiddenIndicator(data) {
    const box = document.getElementById("topology-hidden");
    if (!box) return;
    const hidden = data.hidden_nodes || 0;
    box.textContent = hidden ? tf("topology_hidden_nodes", { count: hidden }) : "";
    box.style.display = hidden ? "block" : "none";
  }
  function topologyButton() {
    return document.querySelector("button[onclick*='runTopologyOnly']");
  }
  function collectionBox() {
    let box = document.getElementById("topology-progress");
    if (!box) {
      const section = document.getElementById("topology-section");
      if (!section) return null;
      box = document.createElement("div");
      box.id = "topology-progress";
      box.style.cssText = "padding:12px 0;line-height:1.7;overflow-wrap:anywhere";
      section.prepend(box);
    }
    return box;
  }
  function renderCollectionProgress() {
    if (!topologyProgressView) return;
    const box = collectionBox();
    if (!box) return;
    const { progress, started, changed, retryAt } = topologyProgressView;
    const expanded = box.querySelector("details")?.open ?? false;
    box.replaceChildren();
    const label = document.createElement("p");
    label.setAttribute("role", "status");
    label.setAttribute("aria-live", "polite");
    label.textContent = `${t(`topology_progress_${progress.state}`)} \xB7 ${tf("topology_progress_counts", { completed: progress.completed, total: progress.total, running: progress.running, queued: progress.queued })} \xB7 ${tf("topology_progress_elapsed", { seconds: Math.floor((Date.now() - started) / 1e3) })}`;
    const bar = document.createElement("progress");
    bar.max = Math.max(1, progress.total);
    bar.value = progress.completed;
    bar.setAttribute("aria-label", t("topology_progress_label"));
    bar.style.cssText = "display:block;width:100%;height:12px;accent-color:#2dd4bf;margin:8px 0";
    box.append(label, bar);
    if (topologyBusy && Date.now() - changed >= 3e4) {
      const warning = document.createElement("p");
      warning.setAttribute("role", "status");
      warning.textContent = t("topology_progress_waiting");
      box.append(warning);
    }
    if (progress.last_activity_at) {
      const updated = document.createElement("p");
      updated.textContent = `${t("topology_progress_updated")}: ${new Date(progress.last_activity_at).toLocaleString()}`;
      box.append(updated);
    }
    const remaining = Math.max(0, Math.ceil((retryAt - Date.now()) / 1e3));
    if (remaining > 0) {
      const cooldown = document.createElement("p");
      cooldown.textContent = tf("topology_progress_cooldown", { seconds: remaining });
      box.append(cooldown);
    }
    const details = document.createElement("details");
    details.open = expanded;
    const summary = document.createElement("summary");
    summary.textContent = t("topology_progress_devices");
    details.append(summary);
    const list = document.createElement("ul");
    list.style.cssText = "padding-left:20px;max-height:180px;overflow:auto";
    for (const device of progress.devices) {
      const item = document.createElement("li");
      item.textContent = `${device.name || device.ip} (${device.ip}) \u2014 ${t(`topology_progress_${device.state}`)}`;
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
    details.append(list);
    box.append(details);
    const button = topologyButton();
    if (button) button.disabled = topologyBusy || remaining > 0;
  }
  function displayTopology(data) {
    if (!data.nodes?.length && topologyState.data?.nodes?.length) return;
    topologyState.data = data;
    renderTopologySummary(data);
    updateHiddenIndicator(data);
    setPlaceholder(data.nodes?.length ? null : data.progress?.queued || data.progress?.running ? "topology_progress_queued" : "topology_no_neighbors", { ip: data.seed_ip || "-" });
    drawTopology();
  }
  async function startTopologyDiscovery(seedIp, community) {
    const section = document.getElementById("topology-section");
    const result = document.getElementById("topology-result");
    if (!section || !result) return;
    const generation = ++topologyGeneration;
    topologyController?.abort();
    const controller = new AbortController();
    topologyController = controller;
    if (topologyCooldownTimer !== null) window.clearInterval(topologyCooldownTimer);
    topologyProgressView = null;
    topologyBusy = true;
    section.style.display = "block";
    const button = topologyButton();
    if (button) button.disabled = true;
    const box = collectionBox();
    if (box) {
      box.textContent = t("topology_progress_starting");
      box.setAttribute("role", "status");
    }
    if (!topologyState.data?.nodes?.length) setPlaceholder("topology_running", { ip: seedIp });
    const started = Date.now();
    let changed = started;
    let signature = "";
    const timeout = window.setTimeout(() => controller.abort(), 3e5);
    const ticker = window.setInterval(renderCollectionProgress, 1e3);
    try {
      let response = await fetch("/api/discovery/topology", {
        method: "POST",
        headers: { "Content-Type": "application/x-www-form-urlencoded", "X-CSRF-Token": window.tracepulseCsrf || "" },
        body: `seed_ip=${encodeURIComponent(seedIp)}&community=${encodeURIComponent(community)}`,
        signal: controller.signal
      });
      if (response.status === 429) response = await fetch("/api/topology", { cache: "no-store", signal: controller.signal });
      while (true) {
        const data = await response.json();
        if (generation !== topologyGeneration) return;
        if (!response.ok || data.error) throw new Error("topology status unavailable");
        displayTopology(data);
        const progress = data.progress;
        if (!progress) break;
        const next = JSON.stringify([progress.completed, progress.running, progress.queued, progress.devices, progress.last_activity_at]);
        if (next !== signature) {
          signature = next;
          changed = Date.now();
        }
        topologyProgressView = { progress, started, changed, retryAt: Date.now() + progress.retry_after_seconds * 1e3 };
        renderCollectionProgress();
        if (!progress.running && !progress.queued) break;
        await new Promise((resolve) => window.setTimeout(resolve, 1500));
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
        topologyBusy = false;
        topologyController = null;
        if (button) button.disabled = false;
        renderCollectionProgress();
        if (topologyProgressView) topologyCooldownTimer = window.setInterval(() => {
          renderCollectionProgress();
          if (topologyProgressView && Date.now() >= topologyProgressView.retryAt && topologyCooldownTimer !== null) {
            window.clearInterval(topologyCooldownTimer);
            topologyCooldownTimer = null;
          }
        }, 1e3);
      }
    }
  }
  function csvCell(value) {
    return `"${String(value ?? "").replace(/"/g, '""')}"`;
  }
  function exportTopology(format) {
    if (!topologyState.data) return;
    const data = topologyState.data;
    const rows = [["record_type", "id", "label", "ip", "hostname", "kind_or_protocol", "status", "source", "source_port", "target", "target_port"]];
    (data.nodes || []).forEach((node) => rows.push(["node", node.id, node.label, node.ip, node.hostname, node.kind, node.status, "", "", "", ""]));
    (data.edges || []).forEach((edge) => rows.push(["edge", edge.id, edge.label, edge.remote_ip, edge.remote_hostname, edge.protocol, "", edge.source, edge.local_port || (edge.local_if_index ? `if-${edge.local_if_index}` : ""), edge.target, edge.remote_port]));
    const content = format === "csv" ? rows.map((row2) => row2.map(csvCell).join(",")).join("\r\n") : JSON.stringify(data, null, 2);
    const blob = new Blob([content], { type: format === "csv" ? "text/csv" : "application/json" });
    const link = document.createElement("a");
    link.href = URL.createObjectURL(blob);
    link.download = `tracepulse-topology-${Date.now()}.${format}`;
    link.click();
    URL.revokeObjectURL(link.href);
  }
  window.closeInspector = closeInspector;
  window.exportTopology = exportTopology;
  window.refreshTopologyTexts = refreshTopologyTexts;
  window.startTopologyDiscovery = startTopologyDiscovery;
  window.topoFit = topoFit;
  window.topoRelayout = topoRelayout;
  window.topoZoom = topoZoom;
  return __toCommonJS(topology_exports);
})();
