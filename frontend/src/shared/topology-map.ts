export type TopologySeverity = "critical" | "warning" | "stale" | "healthy" | "unknown";
export const topologyRiskColor = (severity: TopologySeverity): string => ({
    critical: "#ef4444", warning: "#fbbf24", stale: "#94a3b8", healthy: "#34d399", unknown: "#94a3b8",
})[severity];

export interface TopologyMapNode {
    id: string;
    label?: string | null;
    ip?: string | null;
    kind?: string;
    status?: string;
    seed?: boolean;
    className?: string;
    stateLabel?: string;
    badgeLabel?: string;
    tooltip?: string;
    stroke?: string;
    stale?: boolean;
    health?: { severity?: TopologySeverity; stale?: boolean };
}
export interface TopologyMapEdge {
    id?: string;
    source: string;
    target: string;
    label?: string;
    protocol?: string;
    className?: string;
    tooltip?: string;
    stroke?: string;
    strokeWidth?: number;
    stale?: boolean;
    health_status?: string;
}
interface Position { x: number; y: number }
interface MapOptions<NodeType, EdgeType> {
    onNodeSelect?: (node: NodeType) => void;
    onNodeFocus?: (node: NodeType) => void;
    onNodeHover?: (node: NodeType) => void;
    onEdgeSelect?: (edge: EdgeType) => void;
    onBackground?: () => void;
}
const styles = `
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
function svgElement<Tag extends keyof SVGElementTagNameMap>(tag: Tag): SVGElementTagNameMap[Tag] {
    return document.createElementNS("http://www.w3.org/2000/svg", tag);
}
function widthFor(node: TopologyMapNode): number {
    return Math.min(280, Math.max(128, (node.label || node.ip || node.id).length * 7.4 + 28, (node.stateLabel?.length || 0) * 11 + 28));
}

export class TopologyMap<NodeType extends TopologyMapNode = TopologyMapNode, EdgeType extends TopologyMapEdge = TopologyMapEdge> {
    private nodes: NodeType[] = [];
    private edges: EdgeType[] = [];
    private positions = new Map<string, Position>();
    private scale = 1;
    private tx = 0;
    private ty = 0;
    private selected: string | null = null;
    private root: SVGGElement | null = null;
    private drag: { id: string; x: number; y: number; origin: Position; moved: boolean } | null = null;
    private pan: { x: number; y: number; tx: number; ty: number } | null = null;
    private suppressClick = false;
    private readonly controller = new AbortController();

    constructor(private readonly svg: SVGSVGElement, private readonly options: MapOptions<NodeType, EdgeType> = {}) {
        if (!document.getElementById("topology-map-styles")) {
            const style = document.createElement("style"); style.id = "topology-map-styles"; style.textContent = styles; document.head.append(style);
        }
        svg.classList.add("topology-map"); svg.setAttribute("role", "group");
        const signal = this.controller.signal;
        svg.addEventListener("wheel", event => { event.preventDefault(); this.zoom(event.deltaY < 0 ? 1.1 : .9, event.clientX, event.clientY); }, { passive: false, signal });
        svg.addEventListener("pointerdown", event => {
            if (event.button !== 0 || !event.isPrimary || event.target !== svg) return;
            svg.setPointerCapture(event.pointerId);
            this.pan = { x: event.clientX, y: event.clientY, tx: this.tx, ty: this.ty };
        }, { signal });
        svg.addEventListener("click", event => {
            if (this.suppressClick) { this.suppressClick = false; return; }
            if (event.target === svg) { this.select(null); this.options.onBackground?.(); }
        }, { signal });
        window.addEventListener("pointermove", event => {
            if (!event.isPrimary) return;
            if (this.drag) {
                const deltaX = event.clientX - this.drag.x, deltaY = event.clientY - this.drag.y;
                if (Math.hypot(deltaX, deltaY) > 3) this.drag.moved = true;
                this.positions.set(this.drag.id, { x: this.drag.origin.x + deltaX / this.scale, y: this.drag.origin.y + deltaY / this.scale });
                this.draw();
            } else if (this.pan) {
                this.tx = this.pan.tx + event.clientX - this.pan.x; this.ty = this.pan.ty + event.clientY - this.pan.y; this.transform();
            }
        }, { signal });
        const release = (event: PointerEvent): void => {
            if (!event.isPrimary) return;
            const drag = this.drag;
            this.suppressClick = !!drag; this.drag = null; this.pan = null;
            if (svg.hasPointerCapture(event.pointerId)) svg.releasePointerCapture(event.pointerId);
            if (drag && !drag.moved && event.type === "pointerup") {
                const node = this.nodes.find(item => item.id === drag.id);
                if (node) {
                    this.svg.querySelectorAll<SVGGElement>("[data-node]").forEach(group => { if (group.dataset.node === node.id) group.focus(); });
                    this.select(node.id); this.options.onNodeSelect?.(node);
                }
            }
        };
        window.addEventListener("pointerup", release, { signal });
        window.addEventListener("pointercancel", release, { signal });
    }

    render(nodes: NodeType[], edges: EdgeType[], preserveLayout = true): void {
        const first = this.nodes.length === 0;
        this.nodes = nodes; this.edges = edges;
        const layout = this.layout();
        for (const node of nodes) if (!preserveLayout || !this.positions.has(node.id)) this.positions.set(node.id, layout.get(node.id)!);
        const focused = this.svg.contains(document.activeElement) ? (document.activeElement as SVGElement).dataset.node : undefined;
        this.draw();
        if (first || !preserveLayout) this.fit();
        if (focused) this.svg.querySelectorAll<SVGGElement>("[data-node]").forEach(group => { if (group.dataset.node === focused) group.focus(); });
    }

    private layout(): Map<string, Position> {
        const adjacency = new Map(this.nodes.map(node => [node.id, [] as string[]]));
        for (const edge of this.edges) { adjacency.get(edge.source)?.push(edge.target); adjacency.get(edge.target)?.push(edge.source); }
        const seed = this.nodes.find(node => node.seed) || this.nodes[0];
        const depth = new Map<string, number>();
        if (seed) {
            depth.set(seed.id, 0); const queue = [seed.id];
            for (let index = 0; index < queue.length; index++) for (const next of adjacency.get(queue[index]) || []) {
                if (!depth.has(next)) { depth.set(next, depth.get(queue[index])! + 1); queue.push(next); }
            }
        }
        const layers = new Map<number, string[]>();
        for (const node of this.nodes) { const level = depth.get(node.id) ?? 1; const layer = layers.get(level) || []; layer.push(node.id); layers.set(level, layer); }
        const positions = new Map<string, Position>();
        const gap = Math.max(200, ...this.nodes.map(node => widthFor(node) + 36));
        let nextY = 0;
        for (const [, ids] of [...layers].sort(([first], [second]) => first - second)) {
            const perRow = Math.min(ids.length, Math.max(4, Math.ceil(Math.sqrt(ids.length * 2))));
            ids.forEach((id, index) => {
                const row = Math.floor(index / perRow), columns = Math.min(perRow, ids.length - row * perRow);
                positions.set(id, { x: (index % perRow - (columns - 1) / 2) * gap, y: nextY + row * 120 });
            });
            nextY += Math.ceil(ids.length / perRow) * 120 + 30;
        }
        return positions;
    }

    private draw(): void {
        this.svg.replaceChildren(); this.root = svgElement("g"); this.svg.append(this.root);
        const edges = svgElement("g"), nodes = svgElement("g"); this.root.append(edges, nodes);
        for (const edge of this.edges) {
            const from = this.positions.get(edge.source), to = this.positions.get(edge.target);
            if (!from || !to) continue;
            const group = svgElement("g"); group.setAttribute("class", `topo-edge${edge.stale ? " is-stale" : ""}${edge.protocol?.includes("CDP") ? " cdp" : ""} ${edge.className || ""}`);
            const stroke = edge.stroke || (edge.health_status === "Critical" ? topologyRiskColor("critical") : edge.health_status === "Warning" ? topologyRiskColor("warning") : undefined);
            if (stroke) group.style.setProperty("--edge-stroke", stroke);
            const line = svgElement("line"), hit = svgElement("line"); line.setAttribute("class", "topo-edge-line"); hit.setAttribute("class", "topo-edge-hit");
            if (edge.strokeWidth !== undefined && Number.isFinite(edge.strokeWidth)) {
                group.style.setProperty("--edge-width", String(Math.min(8, Math.max(1.5, edge.strokeWidth))));
                for (const item of [line, hit]) item.setAttribute("vector-effect", "non-scaling-stroke");
            }
            for (const item of [line, hit]) { item.setAttribute("x1", String(from.x)); item.setAttribute("y1", String(from.y)); item.setAttribute("x2", String(to.x)); item.setAttribute("y2", String(to.y)); }
            const label = svgElement("text"); label.setAttribute("class", "topo-edge-label"); label.textContent = edge.label || "";
            label.setAttribute("x", String(from.x + (to.x - from.x) * .68)); label.setAttribute("y", String(from.y + (to.y - from.y) * .68 - 8));
            const title = svgElement("title"); title.textContent = edge.tooltip || edge.label || edge.protocol || "";
            group.append(title, line, hit, label); edges.append(group);
            hit.addEventListener("click", event => { event.stopPropagation(); this.select(null); this.svg.querySelectorAll(".topo-edge.selected").forEach(item => item.classList.remove("selected")); group.classList.add("selected"); this.options.onEdgeSelect?.(edge); });
        }
        for (const node of this.nodes) {
            const position = this.positions.get(node.id); if (!position) continue;
            const group = svgElement("g"); group.dataset.node = node.id;
            group.setAttribute("class", `topo-node ${node.kind === "switch" ? "switch" : "endpoint"}${node.stale || node.health?.stale ? " is-stale" : ""}${this.selected === node.id ? " selected" : ""} ${node.className || ""}`);
            group.setAttribute("transform", `translate(${position.x},${position.y})`); group.setAttribute("role", "button"); group.setAttribute("tabindex", "0");
            group.setAttribute("aria-pressed", String(this.selected === node.id)); group.setAttribute("aria-label", node.tooltip || `${node.label || node.id} ${node.stateLabel || ""}`);
            const stroke = node.stroke || (node.health ? topologyRiskColor(node.health.severity || "unknown") : node.status?.toLowerCase() === "offline" ? topologyRiskColor("critical") : undefined);
            if (stroke) group.style.setProperty("--node-stroke", stroke);
            const width = widthFor(node), height = node.badgeLabel ? 82 : node.stateLabel ? 66 : 44;
            const shape = node.kind === "switch" ? svgElement("rect") : svgElement("circle"); shape.setAttribute("class", "topo-node-shape");
            if (shape instanceof SVGRectElement) { shape.setAttribute("x", String(-width / 2)); shape.setAttribute("y", String(-height / 2)); shape.setAttribute("width", String(width)); shape.setAttribute("height", String(height)); shape.setAttribute("rx", "6"); }
            else shape.setAttribute("r", String(Math.max(height / 2, width / 3.2)));
            const title = svgElement("title"); title.textContent = node.tooltip || `${node.label || node.id} (${node.ip || ""})`;
            group.append(title);
            if (node.health?.severity === "critical" && !node.className?.split(/\s+/).includes("is-muted")) {
                const pulse = shape.cloneNode() as SVGElement; pulse.setAttribute("class", "topo-node-pulse"); group.append(pulse);
            }
            group.append(shape);
            for (const [className, content, offset] of [["topo-node-label", node.label || node.ip || node.id, -height / 2 + 18], ["topo-node-sub", node.ip || "", -height / 2 + 34], ["topo-node-state", node.stateLabel || "", -height / 2 + 50], ["topo-node-badge", node.badgeLabel || "", -height / 2 + 66]] as const) {
                if (!content) continue;
                const label = svgElement("text"); label.setAttribute("class", className); label.setAttribute("y", String(offset));
                const limit = Math.max(12, Math.floor((width - 20) / (className === "topo-node-label" ? 7.4 : 8)));
                label.textContent = content.length > limit ? `${content.slice(0, limit - 3)}...` : content; group.append(label);
            }
            nodes.append(group);
            const choose = (): void => { if (this.suppressClick) { this.suppressClick = false; return; } group.focus(); this.select(node.id); this.options.onNodeSelect?.(node); };
            group.addEventListener("pointerdown", event => { if (event.button !== 0 || !event.isPrimary) return; event.stopPropagation(); this.svg.setPointerCapture(event.pointerId); this.suppressClick = false; this.drag = { id: node.id, x: event.clientX, y: event.clientY, origin: { ...position }, moved: false }; });
            group.addEventListener("click", event => { event.stopPropagation(); choose(); });
            group.addEventListener("keydown", event => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); this.suppressClick = false; choose(); } });
            group.addEventListener("focus", () => this.options.onNodeFocus?.(node));
            group.addEventListener("pointerenter", event => { if (event.pointerType === "mouse" && !this.svg.contains(document.activeElement)) this.options.onNodeHover?.(node); });
        }
        this.transform();
    }

    select(id: string | null): void {
        this.selected = id;
        this.svg.querySelectorAll<SVGGElement>("[data-node]").forEach(group => { const selected = group.dataset.node === id; group.classList.toggle("selected", selected); group.setAttribute("aria-pressed", String(selected)); });
    }
    private transform(): void { this.root?.setAttribute("transform", `translate(${this.tx},${this.ty}) scale(${this.scale})`); }
    zoom(factor: number, clientX?: number, clientY?: number): void {
        const bounds = this.svg.getBoundingClientRect(), next = Math.min(4, Math.max(.05, this.scale * factor)), ratio = next / this.scale;
        const positionX = clientX === undefined ? bounds.width / 2 : clientX - bounds.left, positionY = clientY === undefined ? bounds.height / 2 : clientY - bounds.top;
        this.tx = positionX - (positionX - this.tx) * ratio; this.ty = positionY - (positionY - this.ty) * ratio; this.scale = next; this.transform();
    }
    fit(): void {
        const points = this.nodes.map(node => this.positions.get(node.id)).filter((position): position is Position => !!position);
        if (!points.length) return;
        const bounds = this.svg.getBoundingClientRect(), positionsX = points.map(point => point.x), positionsY = points.map(point => point.y);
        const minX = Math.min(...positionsX), maxX = Math.max(...positionsX), minY = Math.min(...positionsY), maxY = Math.max(...positionsY);
        const padding = Math.max(220, ...this.nodes.map(node => widthFor(node) + 48));
        this.scale = Math.min(2, Math.max(.05, Math.min(bounds.width / (maxX - minX + padding), bounds.height / (maxY - minY + 160))));
        this.tx = bounds.width / 2 - (minX + maxX) / 2 * this.scale; this.ty = bounds.height / 2 - (minY + maxY) / 2 * this.scale; this.transform();
    }
    relayout(): void { this.positions = this.layout(); this.draw(); this.fit(); }
    destroy(): void { this.controller.abort(); this.svg.replaceChildren(); }
}