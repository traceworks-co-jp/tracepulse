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
  var topologyState = { data: null, positions: {}, scale: 1, tx: 0, ty: 0, selected: null, drag: null, pan: null };
  var nodeHeight = 44;
  var layerGap = 150;
  var columnGap = 200;
  var rowGap = 96;
  function t(key) {
    return typeof window.t === "function" ? window.t(key) : key;
  }
  function tf(key, params) {
    return typeof window.tf === "function" ? window.tf(key, params) : key;
  }
  function esc(value) {
    return window.esc ? window.esc(value) : String(value ?? "");
  }
  function svgElement(tag) {
    return document.createElementNS("http://www.w3.org/2000/svg", tag);
  }
  function edgeClass(edge) {
    if (edge.health_status === "Critical") return " crit";
    if (edge.health_status === "Warning") return " warn";
    return edge.protocol?.includes("CDP") ? " cdp" : "";
  }
  function setPlaceholder(key, params = {}, text = null) {
    const box = document.getElementById("topology-empty");
    if (!box) return;
    box.textContent = text || (key ? tf(key, params) : "");
    box.style.display = key || text ? "flex" : "none";
  }
  function computeLayout(nodes, edges, seedIp) {
    const adjacency = {};
    nodes.forEach((node) => {
      adjacency[node.id] = [];
    });
    edges.forEach((edge) => {
      if (adjacency[edge.source] && adjacency[edge.target]) {
        adjacency[edge.source].push(edge.target);
        adjacency[edge.target].push(edge.source);
      }
    });
    const seed = nodes.find((node) => node.seed) || nodes.find((node) => node.id === seedIp) || nodes[0];
    if (!seed) return;
    const depth = { [seed.id]: 0 };
    const queue = [seed.id];
    while (queue.length) {
      const id = queue.shift();
      (adjacency[id] || []).forEach((next) => {
        if (depth[next] === void 0) {
          depth[next] = depth[id] + 1;
          queue.push(next);
        }
      });
    }
    const layers = {};
    nodes.forEach((node) => {
      const level = String(depth[node.id] ?? 1);
      (layers[level] || (layers[level] = [])).push(node.id);
    });
    topologyState.positions = {};
    Object.entries(layers).forEach(([level, ids]) => {
      const perRow = Math.min(ids.length, Math.max(4, Math.ceil(Math.sqrt(ids.length * 2))));
      ids.forEach((id, index) => {
        const row2 = Math.floor(index / perRow);
        const columns = Math.min(perRow, ids.length - row2 * perRow);
        topologyState.positions[id] = { x: (index % perRow - (columns - 1) / 2) * columnGap, y: Number(level) * layerGap + row2 * rowGap };
      });
    });
  }
  function applyTransform(root) {
    root.setAttribute("transform", `translate(${topologyState.tx},${topologyState.ty}) scale(${topologyState.scale})`);
  }
  function drawTopology() {
    const svg = document.getElementById("topology-canvas");
    const data = topologyState.data;
    if (!svg || !data) return;
    svg.innerHTML = "";
    const root = svgElement("g");
    svg.appendChild(root);
    const edges = svgElement("g");
    const nodes = svgElement("g");
    root.append(edges, nodes);
    (data.edges || []).forEach((edge) => {
      const from = topologyState.positions[edge.source];
      const to = topologyState.positions[edge.target];
      if (!from || !to) return;
      const group = svgElement("g");
      group.setAttribute("class", "topo-edge");
      const line = svgElement("line");
      line.setAttribute("class", `topo-edge-line${edgeClass(edge)}`);
      const hit = svgElement("line");
      hit.setAttribute("class", "topo-edge-hit");
      const label = svgElement("text");
      label.setAttribute("class", "topo-edge-label");
      label.textContent = edge.label || "";
      group.append(line, hit, label);
      edges.appendChild(group);
      const dx = to.x - from.x;
      const dy = to.y - from.y;
      const length = Math.hypot(dx, dy) || 1;
      const nx = -dy / length;
      const ny = dx / length;
      const offset = 0;
      [line, hit].forEach((item) => {
        item.setAttribute("x1", String(from.x + nx * offset));
        item.setAttribute("y1", String(from.y + ny * offset));
        item.setAttribute("x2", String(to.x + nx * offset));
        item.setAttribute("y2", String(to.y + ny * offset));
      });
      label.setAttribute("x", String(from.x + dx * 0.68));
      label.setAttribute("y", String(from.y + dy * 0.68 - 8));
      hit.addEventListener("click", (event) => {
        event.stopPropagation();
        openEdgeInspector(edge, group);
      });
    });
    (data.nodes || []).forEach((node) => {
      const position = topologyState.positions[node.id];
      if (!position) return;
      const group = svgElement("g");
      group.setAttribute("class", `topo-node ${node.kind === "switch" ? "switch" : "endpoint"}${node.seed ? " seed" : ""}${node.status?.toLowerCase() === "offline" ? " offline" : ""}`);
      const label = node.label || node.ip || node.id;
      const width = Math.max(104, label.length * 7.4 + 28);
      const shape = node.kind === "switch" ? svgElement("rect") : svgElement("circle");
      if (shape.tagName === "rect") {
        const rect = shape;
        rect.setAttribute("width", String(width));
        rect.setAttribute("height", String(nodeHeight));
        rect.setAttribute("x", String(-width / 2));
        rect.setAttribute("y", String(-nodeHeight / 2));
        rect.setAttribute("rx", "6");
      } else shape.setAttribute("r", String(Math.max(28, width / 3.2)));
      shape.setAttribute("class", "topo-node-shape");
      const title = svgElement("text");
      title.setAttribute("class", "topo-node-label");
      title.setAttribute("y", "-6");
      title.textContent = label;
      const sub = svgElement("text");
      sub.setAttribute("class", "topo-node-sub");
      sub.setAttribute("y", "10");
      sub.textContent = node.ip || node.status || "";
      group.append(shape, title, sub);
      group.setAttribute("transform", `translate(${position.x},${position.y})`);
      nodes.appendChild(group);
      group.addEventListener("mousedown", (event) => {
        event.stopPropagation();
        topologyState.drag = { id: node.id, x: event.clientX, y: event.clientY, originX: position.x, originY: position.y };
      });
      group.addEventListener("click", (event) => {
        event.stopPropagation();
        openNodeInspector(node, group);
      });
    });
    applyTransform(root);
    if (svg.dataset.bound !== "1") {
      svg.dataset.bound = "1";
      svg.addEventListener("wheel", (event) => {
        event.preventDefault();
        topoZoom(event.deltaY < 0 ? 1.1 : 0.9, event.clientX, event.clientY);
      }, { passive: false });
      svg.addEventListener("mousedown", (event) => {
        if (event.target === svg) {
          topologyState.pan = { x: event.clientX, y: event.clientY, tx: topologyState.tx, ty: topologyState.ty };
          svg.classList.add("panning");
        }
      });
      window.addEventListener("mousemove", (event) => {
        if (topologyState.drag) {
          const position = topologyState.positions[topologyState.drag.id];
          if (position) {
            position.x = topologyState.drag.originX + (event.clientX - topologyState.drag.x) / topologyState.scale;
            position.y = topologyState.drag.originY + (event.clientY - topologyState.drag.y) / topologyState.scale;
            drawTopology();
          }
        } else if (topologyState.pan) {
          topologyState.tx = topologyState.pan.tx + event.clientX - topologyState.pan.x;
          topologyState.ty = topologyState.pan.ty + event.clientY - topologyState.pan.y;
          const currentRoot = svg.firstElementChild;
          if (currentRoot) applyTransform(currentRoot);
        }
      });
      window.addEventListener("mouseup", () => {
        topologyState.drag = null;
        topologyState.pan = null;
        svg.classList.remove("panning");
      });
    }
    svg.onclick = () => {
      closeInspector();
      clearSelection();
    };
  }
  function clearSelection() {
    topologyState.selected?.classList.remove("selected");
    topologyState.selected = null;
  }
  function clearInspector() {
    document.getElementById("topology-inspector")?.classList.remove("open");
  }
  function closeInspector() {
    clearInspector();
    clearSelection();
  }
  function row(label, value) {
    return `<div class='inspector-row'><span>${esc(label)}</span><span>${esc(value || "-")}</span></div>`;
  }
  function openInspector(title, html, group) {
    clearSelection();
    group.classList.add("selected");
    topologyState.selected = group;
    const titleBox = document.getElementById("inspector-title");
    const body = document.getElementById("inspector-body");
    if (titleBox) titleBox.textContent = title;
    if (body) body.innerHTML = html;
    document.getElementById("topology-inspector")?.classList.add("open");
  }
  function openNodeInspector(node, group) {
    let html = row(t("ip_address"), node.ip) + row(t("hostname"), node.hostname) + row(t("status"), node.status);
    (node.interfaces || []).forEach((iface) => {
      html += `<div class='inspector-if'><span>${esc(iface.if_name || `if-${iface.if_index}`)}</span><span>${esc(iface.link_status)}</span></div>`;
    });
    openInspector(node.label || node.ip || node.id, html, group);
  }
  function openEdgeInspector(edge, group) {
    const localPort = edge.local_port || (edge.local_if_index ? `if-${edge.local_if_index}` : "");
    const html = row(t("topology_protocol"), edge.protocol) + row(t("topology_device"), edge.local_ip) + row(t("topology_port"), localPort) + row(t("topology_device"), edge.remote_hostname || edge.remote_ip) + row(t("ip_address"), edge.remote_ip) + row(t("topology_port"), edge.remote_port);
    openInspector(edge.label || t("topology_link"), html, group);
  }
  function topoZoom(factor, clientX, clientY) {
    const svg = document.getElementById("topology-canvas");
    if (!svg) return;
    const rect = svg.getBoundingClientRect();
    const x = clientX === void 0 ? rect.width / 2 : clientX - rect.left;
    const y = clientY === void 0 ? rect.height / 2 : clientY - rect.top;
    topologyState.scale = Math.min(4, Math.max(0.15, topologyState.scale * factor));
    topologyState.tx = x - (x - topologyState.tx) * factor;
    topologyState.ty = y - (y - topologyState.ty) * factor;
    const root = svg.firstElementChild;
    if (root) applyTransform(root);
  }
  function topoFit() {
    const svg = document.getElementById("topology-canvas");
    const points = Object.values(topologyState.positions);
    if (!svg || !points.length) return;
    const rect = svg.getBoundingClientRect();
    const xs = points.map((point) => point.x);
    const ys = points.map((point) => point.y);
    const minX = Math.min(...xs), maxX = Math.max(...xs), minY = Math.min(...ys), maxY = Math.max(...ys);
    const scale = Math.min(2, Math.max(0.15, Math.min(rect.width / (maxX - minX + 220), rect.height / (maxY - minY + 220))));
    topologyState.scale = scale;
    topologyState.tx = rect.width / 2 - (minX + maxX) / 2 * scale;
    topologyState.ty = rect.height / 2 - (minY + maxY) / 2 * scale;
    const root = svg.firstElementChild;
    if (root) applyTransform(root);
  }
  function topoRelayout() {
    if (!topologyState.data) return;
    computeLayout(topologyState.data.nodes || [], topologyState.data.edges || [], topologyState.data.seed_ip);
    drawTopology();
    topoFit();
  }
  function refreshTopologyTexts() {
    if (topologyState.data) drawTopology();
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
  async function startTopologyDiscovery(seedIp, community) {
    const section = document.getElementById("topology-section");
    const result = document.getElementById("topology-result");
    if (!section || !result) return;
    section.style.display = "block";
    setPlaceholder("topology_running", { ip: seedIp });
    try {
      const response = await fetch("/api/discovery/topology", { method: "POST", headers: { "Content-Type": "application/x-www-form-urlencoded" }, body: `seed_ip=${encodeURIComponent(seedIp)}&community=${encodeURIComponent(community)}` });
      const data = await response.json();
      if (data.error) {
        setPlaceholder(null, {}, data.error);
        return;
      }
      topologyState.data = data;
      renderTopologySummary(data);
      updateHiddenIndicator(data);
      computeLayout(data.nodes || [], data.edges || [], data.seed_ip);
      setPlaceholder(data.nodes?.length ? null : "topology_no_neighbors", { ip: data.seed_ip || "-" });
      drawTopology();
      topoFit();
    } catch (error) {
      setPlaceholder(null, {}, String(error));
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
