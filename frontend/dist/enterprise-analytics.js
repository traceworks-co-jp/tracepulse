(function () {
    'use strict';

    const state = {
        scope: { name: '', cidrs: [] },
        sankeyChart: null,
        lastSankeyGraph: null,
        lastGeoData: null,
        isInitialLoading: true
    };
    const $ = (id) => document.getElementById(id);

    function determineNodeDepth(name, kind) {
        if (kind === 'source' || name.startsWith('source:')) return 0;
        if (kind === 'ingress' || name.startsWith('ingress:')) return 1;
        if (kind === 'egress' || name.startsWith('egress:')) return 2;
        if (kind === 'destination' || name.startsWith('destination:')) return 3;
        return 0;
    }

    function determineStageName(depth) {
        switch (depth) {
            case 0: return 'Source IP (送信元)';
            case 1: return 'Ingress IF (入力ポート)';
            case 2: return 'Egress IF (出力ポート)';
            case 3: return 'Destination IP (宛先)';
            default: return 'Node';
        }
    }

    function determineNodeColor(depth) {
        switch (depth) {
            case 0: return '#38bdf8'; // Sky Blue (Source)
            case 1: return '#34d399'; // Emerald (Ingress)
            case 2: return '#fbbf24'; // Amber (Egress)
            case 3: return '#c084fc'; // Purple (Destination)
            default: return '#94a3b8';
        }
    }

    function formatInterfaceDisplayName(label, depth) {
        if (depth !== 1 && depth !== 2) return label;
        if (!label) return 'Internal/Local (idx 0)';
        if (label === 'ifIndex 0' || label === '0') return 'Internal/Local (idx 0)';
        const m = label.match(/^ifIndex\s+(\d+)$/);
        if (m) {
            const idx = parseInt(m[1], 10);
            if (idx === 0) return 'Internal/Local (idx 0)';
            if (idx <= 8) return `Gi0/${idx - 1} (idx ${idx})`;
            return `eth${idx - 1} (idx ${idx})`;
        }
        return label;
    }

    function showLoadingSpinner(hostId, text) {
        const host = $(hostId);
        if (!host) return;
        // 既にチャートやデータが描画されている場合はDOMを破壊せずそのまま維持（チラつき防止）
        if (hostId === 'enterprise-sankey' && state.sankeyChart && state.lastSankeyGraph) return;
        if (hostId === 'enterprise-geoip-list' && state.lastGeoData) return;

        host.innerHTML = `<div class="enterprise-loading-wrap"><span class="enterprise-spinner"></span><span>${text || 'Loading analytics...'}</span></div>`;
    }

    function renderSankey(graph) {
        const host = $('enterprise-sankey');
        if (!host) return;
        if (!window.echarts) {
            host.innerHTML = '<div style="color:#ef4444;padding:1rem;">ECharts is unavailable.</div>';
            return;
        }

        try {
            const rawNodes = graph && graph.nodes ? graph.nodes : [];
            const rawLinks = graph && graph.links ? graph.links : [];

            if (!rawNodes.length || !rawLinks.length) {
                if (!state.lastSankeyGraph) {
                    if (state.sankeyChart) {
                        state.sankeyChart.dispose();
                        state.sankeyChart = null;
                    }
                    host.innerHTML = '<div style="color:#64748b;font-style:italic;padding:2rem;text-align:center;">No flow records in the current window. Waiting for traffic...</div>';
                    return;
                }
            } else {
                state.lastSankeyGraph = graph;
            }

            const activeGraph = state.lastSankeyGraph || graph;
            const rawNodeMap = new Map();
            (activeGraph.nodes || []).forEach((n) => {
                const id = n.id || n.name || n.label;
                rawNodeMap.set(id, n);
            });

            // リンクの検証（厳格な階層チェック: depth(source) < depth(target) によるDAG完全保証）
            const validLinks = [];
            const linkedNodeIds = new Set();
            let totalFlowVolume = 0;

            (activeGraph.links || []).forEach((l) => {
                const s = l.source;
                const t = l.target;
                const val = Number(l.value_bytes || l.value) || 1;
                if (s && t && s !== t && rawNodeMap.has(s) && rawNodeMap.has(t) && val > 0) {
                    const sDepth = determineNodeDepth(s, rawNodeMap.get(s)?.kind);
                    const tDepth = determineNodeDepth(t, rawNodeMap.get(t)?.kind);
                    if (sDepth < tDepth) {
                        validLinks.push({
                            source: s,
                            target: t,
                            value: val
                        });
                        linkedNodeIds.add(s);
                        linkedNodeIds.add(t);
                        if (sDepth === 0) {
                            totalFlowVolume += val;
                        }
                    }
                }
            });

            if (totalFlowVolume === 0 && validLinks.length > 0) {
                totalFlowVolume = validLinks.reduce((acc, l) => acc + l.value, 0) / 3;
            }
            totalFlowVolume = Math.max(1, totalFlowVolume);

            // 各ノードの通過トラフィック量を集計
            const nodeTrafficMap = new Map();
            validLinks.forEach((l) => {
                nodeTrafficMap.set(l.source, (nodeTrafficMap.get(l.source) || 0) + l.value);
                nodeTrafficMap.set(l.target, (nodeTrafficMap.get(l.target) || 0) + l.value);
            });

            // リンクに接続されている有効なノードのみを構築
            const activeNodes = [];
            linkedNodeIds.forEach((id) => {
                const n = rawNodeMap.get(id);
                if (!n) return;
                const depth = determineNodeDepth(id, n.kind);
                const rawLabel = n.label || id.replace(/^(source|ingress|egress|destination):/, '');
                const labelText = formatInterfaceDisplayName(rawLabel, depth);

                activeNodes.push({
                    name: id,
                    depth: depth,
                    label: {
                        show: true,
                        formatter: labelText,
                        color: '#e2e8f0',
                        fontSize: 11
                    },
                    itemStyle: {
                        color: determineNodeColor(depth),
                        borderColor: '#0f172a',
                        borderWidth: 1
                    },
                    meta: {
                        depth: depth,
                        displayName: labelText,
                        trafficBytes: nodeTrafficMap.get(id) || 0
                    }
                });
            });

            if (!activeNodes.length || !validLinks.length) {
                if (state.sankeyChart) {
                    state.sankeyChart.dispose();
                    state.sankeyChart = null;
                }
                host.innerHTML = '<div style="color:#64748b;font-style:italic;padding:2rem;text-align:center;">No active flow paths in current view.</div>';
                return;
            }

            // 既存のChartインスタンスの確認または新規初期化
            let chartInstance = state.sankeyChart || echarts.getInstanceByDom(host);
            if (!chartInstance) {
                host.innerHTML = '';
                chartInstance = echarts.init(host, 'dark');
                state.sankeyChart = chartInstance;
            }

            const option = {
                backgroundColor: 'transparent',
                animation: false,
                tooltip: {
                    trigger: 'item',
                    triggerOn: 'mousemove',
                    backgroundColor: 'rgba(15, 23, 42, 0.95)',
                    borderColor: '#334155',
                    borderWidth: 1,
                    padding: [8, 12],
                    textStyle: { color: '#f8fafc' },
                    formatter: function (params) {
                        if (params.dataType === 'edge') {
                            const sNode = rawNodeMap.get(params.data.source);
                            const tNode = rawNodeMap.get(params.data.target);
                            const sDepth = determineNodeDepth(params.data.source, sNode?.kind);
                            const tDepth = determineNodeDepth(params.data.target, tNode?.kind);

                            const sRawLabel = sNode?.label || (params.data.source || '').replace(/^(source|ingress|egress|destination):/, '');
                            const tRawLabel = tNode?.label || (params.data.target || '').replace(/^(source|ingress|egress|destination):/, '');
                            const sLabel = formatInterfaceDisplayName(sRawLabel, sDepth);
                            const tLabel = formatInterfaceDisplayName(tRawLabel, tDepth);

                            const bytes = Number(params.data.value) || 0;
                            const formattedBytes = formatBytes(bytes);
                            const pct = ((bytes / totalFlowVolume) * 100).toFixed(1);
                            const mbps = (bytes * 8 / (1024 * 1024) / 60).toFixed(2);

                            const sColor = determineNodeColor(sDepth);
                            const tColor = determineNodeColor(tDepth);
                            const sStage = determineStageName(sDepth);
                            const tStage = determineStageName(tDepth);

                            return `
                <div style="font-size:12px;line-height:1.5;min-width:240px;">
                  <div style="font-size:10px;color:#94a3b8;margin-bottom:3px;letter-spacing:0.3px;">
                    ${sStage} &rarr; ${tStage}
                  </div>
                  <div style="font-size:13px;font-weight:700;margin-bottom:6px;border-bottom:1px solid #334155;padding-bottom:4px;">
                    <span style="color:${sColor};">${sLabel}</span>
                    <span style="color:#64748b;margin:0 4px;">&rarr;</span>
                    <span style="color:${tColor};">${tLabel}</span>
                  </div>
                  <div style="color:#cbd5e1;font-size:12px;">
                    <strong>流量:</strong> <span style="color:#34d399;font-weight:700;">${formattedBytes}</span>
                    <span style="color:#94a3b8;font-size:11px;">(全体の ${pct}%)</span>
                  </div>
                  <div style="color:#94a3b8;font-size:11px;margin-top:2px;">
                    <strong>帯域レート:</strong> ~${mbps} Mbps
                  </div>
                </div>
              `;
                        }

                        const meta = params.data.meta || {};
                        const depth = meta.depth != null ? meta.depth : 0;
                        const stage = determineStageName(depth);
                        const color = determineNodeColor(depth);
                        const label = meta.displayName || params.name;
                        const nodeBytes = meta.trafficBytes ? formatBytes(meta.trafficBytes) : '-';

                        return `
              <div style="font-size:12px;line-height:1.5;min-width:180px;">
                <div style="font-size:10px;color:${color};font-weight:600;margin-bottom:2px;">
                  [${stage}]
                </div>
                <div style="font-size:13px;font-weight:700;color:#f8fafc;margin-bottom:4px;border-bottom:1px solid #334155;padding-bottom:3px;">
                  ${label}
                </div>
                <div style="color:#cbd5e1;font-size:11px;">
                  <strong>通過トラフィック:</strong> <span style="color:#34d399;font-weight:600;">${nodeBytes}</span>
                </div>
              </div>
            `;
                    }
                },
                series: [{
                    type: 'sankey',
                    orient: 'horizontal',
                    nodeAlign: 'justify',
                    emphasis: { focus: 'adjacency' },
                    data: activeNodes,
                    links: validLinks,
                    left: 20,
                    right: 180,
                    top: 20,
                    bottom: 20,
                    nodeWidth: 18,
                    nodeGap: 18,
                    draggable: true,
                    lineStyle: {
                        color: 'gradient',
                        curveness: 0.5,
                        opacity: 0.45
                    }
                }]
            };

            chartInstance.setOption(option, true);
            chartInstance.resize();
        } catch (err) {
            console.error('Sankey render error:', err);
        }
    }

    function renderBadges(badges) {
        const host = $('enterprise-threat-badges');
        if (!host) return;
        host.innerHTML = (badges || []).map((badge) => `<span class="enterprise-threat-badge">[${badge.label}] ${badge.ip}</span>`).join('') || '<span style="color:#64748b;font-style:italic;font-size:12px;">No active threat badges.</span>';
    }

    function formatBytes(bytes) {
        if (!bytes || isNaN(bytes)) return '0 B';
        if (bytes >= 1073741824) return (bytes / 1073741824).toFixed(2) + ' GB';
        if (bytes >= 1048576) return (bytes / 1048576).toFixed(2) + ' MB';
        if (bytes >= 1024) return (bytes / 1024).toFixed(1) + ' KB';
        return bytes + ' B';
    }

    function renderGeoList(geoData) {
        const host = $('enterprise-geoip-list');
        if (!host) return;

        let asnList = [];
        let classification = null;

        if (geoData && geoData.asn_summary) {
            asnList = geoData.asn_summary;
            classification = geoData.classification;
            state.lastGeoData = geoData;
        } else if (Array.isArray(geoData) || (geoData && geoData.points)) {
            const points = Array.isArray(geoData) ? geoData : geoData.points;
            const agg = new Map();
            let priv = 0, inet = 0, bcast = 0;
            points.forEach((p) => {
                const asn = p.asn;
                const name = p.country || 'Unknown';
                const bytes = Number(p.traffic_bytes || p.bytes) || 0;
                const key = `${asn}_${name}`;
                if (!agg.has(key)) agg.set(key, { asn, name, traffic_bytes: 0 });
                agg.get(key).traffic_bytes += bytes;
                if (name === 'Private LAN') priv += bytes;
                else if (name === 'Broadcast' || name === 'Multicast/Broadcast') bcast += bytes;
                else inet += bytes;
            });
            const total = Math.max(1, priv + inet + bcast);
            asnList = Array.from(agg.values()).map((item) => ({
                ...item,
                percentage: (item.traffic_bytes / total) * 100
            })).sort((a, b) => b.traffic_bytes - a.traffic_bytes).slice(0, 5);
            classification = { private_bytes: priv, internet_bytes: inet, broadcast_bytes: bcast };
            state.lastGeoData = { asn_summary: asnList, classification };
        } else if (state.lastGeoData) {
            asnList = state.lastGeoData.asn_summary;
            classification = state.lastGeoData.classification;
        }

        if (!asnList.length) {
            if (!state.lastGeoData) {
                host.innerHTML = '<span class="enterprise-muted" style="font-style:italic;color:#64748b;padding:.5rem 0;">No ASN / GeoIP data in current window. Waiting for traffic...</span>';
            }
            return;
        }

        // 1. Classification Bar (Private vs Internet vs Broadcast)
        let classBarHtml = '';
        if (classification) {
            const cTotal = Math.max(1, (classification.private_bytes || 0) + (classification.internet_bytes || 0) + (classification.broadcast_bytes || 0));
            const privPct = ((classification.private_bytes || 0) / cTotal * 100).toFixed(1);
            const inetPct = ((classification.internet_bytes || 0) / cTotal * 100).toFixed(1);
            const bcastPct = ((classification.broadcast_bytes || 0) / cTotal * 100).toFixed(1);

            classBarHtml = `
        <div class="enterprise-class-wrapper">
          <div class="enterprise-class-legend">
            <span class="class-tag class-priv"><i class="class-dot"></i>Private: <b>${privPct}%</b></span>
            <span class="class-tag class-inet"><i class="class-dot"></i>Internet: <b>${inetPct}%</b></span>
            <span class="class-tag class-bcast"><i class="class-dot"></i>Bcast/Mcast: <b>${bcastPct}%</b></span>
          </div>
          <div class="enterprise-class-track">
            <span class="class-seg-priv" style="width:${privPct}%;" title="Private LAN: ${formatBytes(classification.private_bytes)}"></span>
            <span class="class-seg-inet" style="width:${inetPct}%;" title="Internet / Public: ${formatBytes(classification.internet_bytes)}"></span>
            <span class="class-seg-bcast" style="width:${bcastPct}%;" title="Broadcast: ${formatBytes(classification.broadcast_bytes)}"></span>
          </div>
        </div>
      `;
        }

        // 2. Top 5 ASN / Country Rows
        const rowsHtml = asnList.map((item) => {
            const asnBadge = item.asn ? `<span class="enterprise-asn-pill">AS${item.asn}</span>` : `<span class="enterprise-asn-pill local">Local</span>`;
            const nameStr = item.name || 'Private LAN';
            const bytesStr = formatBytes(item.traffic_bytes);
            const pctStr = `${item.percentage.toFixed(1)}%`;

            return `
        <div class="enterprise-geo-row">
          <div class="enterprise-geo-label">
            ${asnBadge}
            <span class="enterprise-geo-name" title="${nameStr}">${nameStr}</span>
          </div>
          <div class="enterprise-geo-bar">
            <i style="width:${Math.min(100, Math.max(3, item.percentage))}%;"></i>
          </div>
          <div class="enterprise-geo-meta">
            <span class="enterprise-geo-bytes">${bytesStr}</span>
            <span class="enterprise-geo-pct">(${pctStr})</span>
          </div>
        </div>
      `;
        }).join('');

        host.innerHTML = classBarHtml + rowsHtml;
    }

    async function refresh() {
        try {
            const [sankey, geoip, qos] = await Promise.all([
                fetch('/api/enterprise/sankey').then((response) => response.json()).catch(() => null),
                fetch('/api/enterprise/geoip').then((response) => response.json()).catch(() => null),
                fetch('/api/enterprise/bgp-qos').then((response) => response.json()).catch(() => null),
            ]);

            if (sankey) {
                renderSankey(sankey);
                renderBadges(sankey.threat_badges || []);
            }
            if (geoip) {
                renderGeoList(geoip);
            }
            if (qos) {
                const qosHost = $('enterprise-bgp-qos');
                if (qosHost) {
                    const dscpCount = Object.keys(qos.dscp_bytes || {}).length;
                    const bgpCount = Object.keys(qos.bgp_next_hop_bytes || {}).length;
                    qosHost.textContent = `DSCP classes: ${dscpCount} | BGP next hops: ${bgpCount}`;
                }
            }
            state.isInitialLoading = false;
        } catch (e) {
            console.warn('Analytics refresh error:', e);
            state.isInitialLoading = false;
        }
    }

    async function loadScope() {
        try {
            const scope = await fetch('/api/enterprise/scope').then((response) => response.json());
            state.scope = scope;
            const selector = $('enterprise-scope');
            if (selector) selector.value = scope.name || '';
        } catch (e) {
            // ignore
        }
        await refresh();
    }

    window.setEnterpriseScope = async function (value) {
        const cidrs = value ? value.split(',').map((item) => item.trim()).filter(Boolean) : [];
        state.scope = { name: value, cidrs };
        state.lastSankeyGraph = null;
        state.lastGeoData = null;

        showLoadingSpinner('enterprise-sankey', 'Applying scope and loading flows...');
        showLoadingSpinner('enterprise-geoip-list', 'Filtering GeoIP / ASN...');

        try {
            await fetch('/api/enterprise/scope', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(state.scope)
            });
        } catch (e) { }
        await refresh();
    };

    document.addEventListener('DOMContentLoaded', () => {
        loadScope();
        setInterval(refresh, 8000);
    });
    window.addEventListener('resize', () => {
        if (state.sankeyChart) state.sankeyChart.resize();
    });
}());
