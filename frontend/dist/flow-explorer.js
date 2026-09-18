(function () {
    'use strict';

    const state = {
        chart: null,
        axisChart: null,
        sankeyChart: null,
        filters: {
            src_ip: '',
            dst_ip: '',
            port: '',
            proto: '',
            if_index: '',
            time_start: '',
            time_end: '',
            compare_with: '',
            axis: 'ip'
        },
        timelineBuckets: []
    };
    const $ = (id) => document.getElementById(id);

    // "+150%"はベースライン比で現在値がその2.5倍(1+1.5)を超えることを意味する。
    const SPIKE_THRESHOLD_MULTIPLIER = 2.5;

    function formatBytes(bytes) {
        const value = Number(bytes) || 0;
        if (value >= 1073741824) return (value / 1073741824).toFixed(2) + ' GB';
        if (value >= 1048576) return (value / 1048576).toFixed(2) + ' MB';
        if (value >= 1024) return (value / 1024).toFixed(1) + ' KB';
        return value + ' B';
    }

    // TracePulse の observed_at 表記 ("YYYY-MM-DD HH:MM:SS") と ISO 文字列を相互変換する。
    function toIsoLocal(value) {
        if (!value) return '';
        const date = new Date(value.includes('T') ? value : value.replace(' ', 'T') + 'Z');
        return Number.isNaN(date.getTime()) ? '' : date.toISOString();
    }

    function readFiltersFromUrl() {
        const params = new URLSearchParams(window.location.search);
        ['src_ip', 'dst_ip', 'port', 'proto', 'if_index', 'time_start', 'time_end', 'compare_with', 'axis'].forEach((key) => {
            const value = params.get(key);
            if (value) state.filters[key] = value;
        });
    }

    function syncFormFromFilters() {
        if ($('fx-src-ip')) $('fx-src-ip').value = state.filters.src_ip || '';
        if ($('fx-dst-ip')) $('fx-dst-ip').value = state.filters.dst_ip || '';
        if ($('fx-port')) $('fx-port').value = state.filters.port || '';
        if ($('fx-proto')) $('fx-proto').value = state.filters.proto || '';
        if ($('fx-if-index')) $('fx-if-index').value = state.filters.if_index || '';
        if ($('fx-compare-mode')) $('fx-compare-mode').value = state.filters.compare_with || '';
        if ($('fx-axis-select')) $('fx-axis-select').value = state.filters.axis || 'ip';
    }

    function readFormIntoFilters() {
        state.filters.src_ip = ($('fx-src-ip')?.value || '').trim();
        state.filters.dst_ip = ($('fx-dst-ip')?.value || '').trim();
        state.filters.port = ($('fx-port')?.value || '').trim();
        state.filters.proto = ($('fx-proto')?.value || '').trim();
        state.filters.if_index = ($('fx-if-index')?.value || '').trim();
    }

    // 現在のフィルターを URL クエリ文字列へ反映し、直リンク共有できるようにする。
    function updateUrl() {
        const params = new URLSearchParams();
        Object.keys(state.filters).forEach((key) => {
            const value = state.filters[key];
            if (value) params.set(key, value);
        });
        const query = params.toString();
        const next = query ? `${window.location.pathname}?${query}` : window.location.pathname;
        window.history.replaceState(null, '', next);
    }

    function buildFlowsQuery(extra) {
        const params = new URLSearchParams();
        Object.keys(state.filters).forEach((key) => {
            const value = state.filters[key];
            if (value) params.set(key, value);
        });
        params.set('limit', '200');
        if (extra) {
            Object.keys(extra).forEach((key) => params.set(key, extra[key]));
        }
        return params.toString();
    }

    function renderTable(rows) {
        const tbody = $('flow-explorer-tbody');
        if (!tbody) return;
        if (!rows || !rows.length) {
            tbody.innerHTML = '<tr><td colspan="8" class="empty">No matching flows for the current filters.</td></tr>';
            return;
        }
        tbody.innerHTML = rows.map((row) => `
      <tr>
        <td>${row.observed_at}</td>
        <td>${row.source_ip}:${row.source_port}</td>
        <td>${row.destination_ip}:${row.destination_port}</td>
        <td>${row.protocol}</td>
        <td>${row.ingress_if_index != null ? row.ingress_if_index : '-'}</td>
        <td>${row.egress_if_index != null ? row.egress_if_index : '-'}</td>
        <td>${formatBytes(row.bytes)}</td>
        <td>${row.packets}</td>
      </tr>
    `).join('');
    }

    function renderTopTalkers(rows) {
        const host = $('flow-explorer-top-talkers');
        if (!host) return;
        if (!rows || !rows.length) {
            host.innerHTML = '<span style="color:#64748b;font-style:italic;font-size:.85rem;">No flows in the current selection.</span>';
            return;
        }
        const totals = new Map();
        rows.forEach((row) => {
            const key = `${row.source_ip}:${row.source_port} \u2192 ${row.destination_ip}:${row.destination_port} (${row.protocol})`;
            totals.set(key, (totals.get(key) || 0) + Number(row.bytes || 0));
        });
        const sorted = Array.from(totals.entries()).sort((a, b) => b[1] - a[1]).slice(0, 10);
        host.innerHTML = sorted.map(([label, bytes]) => `
      <div class="fx-talker-row">
        <span class="fx-talker-endpoints">${label}</span>
        <span class="fx-talker-bytes">${formatBytes(bytes)}</span>
      </div>
    `).join('');
    }

    function looksLikeIp(value) {
        return /^\d{1,3}(\.\d{1,3}){3}$/.test(value || '');
    }

    function parseIfIndexFromLabel(label) {
        const match = /idx (\d+)/.exec(label || '');
        return match ? match[1] : null;
    }

    function determineSankeyDepth(kind) {
        switch (kind) {
            case 'ingress': return 0;
            case 'source': return 1;
            case 'destination': return 2;
            case 'egress': return 3;
            default: return 0;
        }
    }

    function sankeyNodeColor(depth) {
        switch (depth) {
            case 0: return '#34d399';
            case 1: return '#38bdf8';
            case 2: return '#c084fc';
            case 3: return '#fbbf24';
            default: return '#94a3b8';
        }
    }

    function renderZoneMatrix(zones) {
        const host = $('flow-explorer-zone-matrix');
        if (!host) return;
        const total = Math.max(1, Number(zones.total_bytes) || 0);
        const cards = [
            { key: 'internal_bytes', name: 'Internal (LAN\u2194LAN)', color: '#38bdf8' },
            { key: 'outbound_bytes', name: 'Outbound (LAN\u2192WAN)', color: '#f97316', highlight: true },
            { key: 'inbound_bytes', name: 'Inbound (WAN\u2192LAN)', color: '#c084fc' },
            { key: 'wan_bytes', name: 'WAN (\u62e0\u70b9\u9593)', color: '#34d399' }
        ];
        host.innerHTML = cards.map((card) => {
            const bytes = Number(zones[card.key]) || 0;
            const pct = ((bytes / total) * 100).toFixed(1);
            return `
        <div class="fx-zone-card${card.highlight ? ' fx-zone-outbound' : ''}">
          <div class="fx-zone-name">${card.name}</div>
          <div class="fx-zone-bytes">${formatBytes(bytes)}<span class="fx-zone-pct">${pct}%</span></div>
          <div class="fx-zone-track"><div class="fx-zone-fill" style="width:${pct}%;background:${card.color};"></div></div>
        </div>
      `;
        }).join('');
    }

    // Sankeyのノード/リンク選択を Flow Detail のグローバルフィルターへ反映する（ルート単位のドリルダウン）。
    function applySankeySelection(node, keepExisting) {
        if (!node) return;
        if (!keepExisting) {
            state.filters.src_ip = '';
            state.filters.dst_ip = '';
            state.filters.if_index = '';
        }
        if (node.kind === 'source' && looksLikeIp(node.label)) {
            state.filters.src_ip = node.label;
        } else if (node.kind === 'destination' && looksLikeIp(node.label)) {
            state.filters.dst_ip = node.label;
        } else if (node.kind === 'ingress' || node.kind === 'egress') {
            const idx = parseIfIndexFromLabel(node.label);
            if (idx) state.filters.if_index = idx;
        }
        syncFormFromFilters();
        updateUrl();
        loadFlows();
    }

    function renderSankey(rawNodes, rawLinks) {
        const host = $('flow-explorer-sankey');
        if (!host || !window.echarts) return;

        const nodeMap = new Map();
        (rawNodes || []).forEach((node) => nodeMap.set(node.id, node));

        const links = (rawLinks || [])
            .filter((link) => nodeMap.has(link.source) && nodeMap.has(link.target))
            .map((link) => ({ source: link.source, target: link.target, value: Math.max(1, Number(link.value) || 1) }));
        if (!links.length) {
            host.innerHTML = '<div style="color:#64748b;font-style:italic;padding:1.5rem;text-align:center;">No flow records in the current window.</div>';
            return;
        }

        const linkedIds = new Set();
        links.forEach((link) => { linkedIds.add(link.source); linkedIds.add(link.target); });

        const nodes = Array.from(linkedIds).map((id) => {
            const node = nodeMap.get(id);
            const depth = determineSankeyDepth(node.kind);
            return {
                name: id,
                depth,
                label: { show: true, formatter: node.label, color: '#e2e8f0', fontSize: 10 },
                itemStyle: { color: sankeyNodeColor(depth), borderColor: '#0f172a', borderWidth: 1 },
                meta: node
            };
        });

        if (!state.sankeyChart) {
            state.sankeyChart = window.echarts.init(host, 'dark');
        }
        state.sankeyChart.setOption({
            backgroundColor: 'transparent',
            animationDuration: 400,
            animationEasing: 'cubicOut',
            tooltip: {
                trigger: 'item',
                formatter: function (params) {
                    if (params.dataType === 'edge') {
                        const sourceNode = nodeMap.get(params.data.source);
                        const targetNode = nodeMap.get(params.data.target);
                        return `${sourceNode ? sourceNode.label : params.data.source} &rarr; ${targetNode ? targetNode.label : params.data.target}<br/>${formatBytes(params.value)}`;
                    }
                    return params.data.meta ? params.data.meta.label : params.name;
                }
            },
            series: [{
                type: 'sankey',
                orient: 'horizontal',
                nodeAlign: 'justify',
                emphasis: { focus: 'adjacency' },
                data: nodes,
                links: links,
                left: 10,
                right: 130,
                top: 10,
                bottom: 10,
                nodeWidth: 14,
                nodeGap: 10,
                draggable: false,
                lineStyle: { color: 'gradient', curveness: 0.5, opacity: 0.45 }
            }]
        }, true);

        state.sankeyChart.off('click');
        state.sankeyChart.on('click', function (params) {
            if (params.dataType === 'node') {
                applySankeySelection(params.data.meta);
            } else if (params.dataType === 'edge') {
                applySankeySelection(nodeMap.get(params.data.source));
                applySankeySelection(nodeMap.get(params.data.target), true);
            }
        });
    }

    async function loadSankeyAndZones() {
        let data = null;
        try {
            const response = await fetch('/api/v1/sankey?window=600');
            data = await response.json();
        } catch (error) {
            console.warn('Sankey/zone load failed:', error);
            return;
        }
        if (!data || data.error) return;
        renderZoneMatrix(data.zones || {});
        renderSankey(data.nodes || [], data.links || []);
    }

    async function loadFlows() {
        const summary = $('flow-explorer-summary');
        if (summary) summary.textContent = 'Loading...';
        try {
            const query = buildFlowsQuery();
            const response = await fetch(`/api/v1/flows?${query}`);
            const data = await response.json();
            if (data && data.error) {
                renderTable([]);
                renderTopTalkers([]);
                if (summary) summary.textContent = data.error;
                return;
            }
            const rows = (data && data.rows) || [];
            renderTable(rows);
            renderTopTalkers(rows);
            if (summary) {
                const total = data && typeof data.total_matched === 'number' ? data.total_matched : rows.length;
                summary.textContent = `${rows.length.toLocaleString()} shown / ${total.toLocaleString()} matched`;
            }
        } catch (error) {
            console.warn('Flow Explorer query failed:', error);
            if (summary) summary.textContent = 'Failed to load flows.';
        }
    }

    async function loadTimeline() {
        const host = $('flow-explorer-timeline');
        if (!host || !window.echarts) return;

        const timeEnd = state.filters.time_end ? new Date(state.filters.time_end) : new Date();
        const timeStart = state.filters.time_start
            ? new Date(state.filters.time_start)
            : new Date(timeEnd.getTime() - 60 * 60 * 1000);

        const params = new URLSearchParams();
        ['src_ip', 'dst_ip', 'port', 'proto', 'if_index'].forEach((key) => {
            if (state.filters[key]) params.set(key, state.filters[key]);
        });
        params.set('time_start', timeStart.toISOString());
        params.set('time_end', timeEnd.toISOString());
        if (state.filters.compare_with) params.set('compare_with', state.filters.compare_with);

        let data = null;
        try {
            const response = await fetch(`/api/v1/flows/timeseries?${params.toString()}`);
            data = await response.json();
        } catch (error) {
            console.warn('Timeline load failed:', error);
            return;
        }
        if (!data || data.error) return;

        const current = data.current || [];
        const compare = data.compare || null;
        state.timelineBuckets = current;

        const categories = current.map((point) => point.bucket);
        const currentBps = current.map((point) => ((Number(point.bytes) || 0) * 8) / 60);
        // 比較系列はバケット位置(index)を現在系列に揃えて重畳表示する。
        const compareBps = compare
            ? categories.map((_, index) => (compare[index] ? ((Number(compare[index].bytes) || 0) * 8) / 60 : null))
            : null;

        const spikeIndices = [];
        if (compareBps) {
            currentBps.forEach((bps, index) => {
                const baseline = compareBps[index];
                if (baseline != null && baseline > 0 && bps > baseline * SPIKE_THRESHOLD_MULTIPLIER) {
                    spikeIndices.push(index);
                }
            });
        }

        if (!state.chart) {
            state.chart = window.echarts.init(host, 'dark');
        }

        const series = [{
            name: 'Current',
            type: 'line',
            data: currentBps,
            smooth: true,
            showSymbol: false,
            lineStyle: { color: '#38bdf8', width: 2 },
            markPoint: spikeIndices.length ? {
                symbol: 'pin',
                symbolSize: 40,
                itemStyle: { color: '#f87171' },
                label: { formatter: '!', color: '#0b1220', fontWeight: 700 },
                data: spikeIndices.map((index) => ({
                    name: 'Spike',
                    coord: [categories[index], currentBps[index]]
                }))
            } : undefined
        }];

        if (compareBps) {
            series.push({
                name: compareLabel(state.filters.compare_with),
                type: 'line',
                data: compareBps,
                smooth: true,
                showSymbol: false,
                lineStyle: { color: '#94a3b8', type: 'dashed', width: 1.5 },
                areaStyle: { color: 'rgba(148, 163, 184, 0.15)' }
            });
        }

        state.chart.setOption({
            backgroundColor: 'transparent',
            grid: { left: 55, right: 20, top: 30, bottom: 60 },
            tooltip: { trigger: 'axis' },
            legend: { top: 0, textStyle: { color: '#94a3b8' } },
            brush: {
                toolbox: ['lineX', 'clear'],
                xAxisIndex: 0,
                throttleType: 'debounce',
                throttleDelay: 150
            },
            toolbox: {
                show: true,
                right: 10,
                feature: { brush: { type: ['lineX', 'clear'] } }
            },
            xAxis: { type: 'category', data: categories, axisLabel: { color: '#94a3b8', rotate: 35 } },
            yAxis: { type: 'value', name: 'bps', axisLabel: { color: '#94a3b8' } },
            series: series
        }, true);

        state.chart.dispatchAction({ type: 'takeGlobalCursor', key: 'brush', brushOption: { brushType: 'lineX' } });

        state.chart.off('brushSelected');
        state.chart.on('brushSelected', (params) => {
            const area = params.batch && params.batch[0] && params.batch[0].areas && params.batch[0].areas[0];
            if (!area || !area.coordRange) return;
            const [startIdx, endIdx] = area.coordRange;
            const startBucket = state.timelineBuckets[Math.max(0, Math.round(startIdx))];
            const endBucket = state.timelineBuckets[Math.min(state.timelineBuckets.length - 1, Math.round(endIdx))];
            if (!startBucket || !endBucket) return;
            state.filters.time_start = toIsoLocal(startBucket.bucket);
            state.filters.time_end = toIsoLocal(endBucket.bucket);
            updateUrl();
            loadTimeline();
            loadFlows();
        });
    }

    function compareLabel(mode) {
        if (mode === '1d') return 'Previous Day';
        if (mode === '7d') return 'Previous Week';
        return 'Compare';
    }

    const AXIS_COLORS = ['#38bdf8', '#34d399', '#fbbf24', '#c084fc', '#f87171', '#a3e635', '#22d3ee', '#f472b6', '#facc15', '#94a3b8'];

    async function loadTopTalkersAxis() {
        const chartHost = $('flow-explorer-axis-chart');
        const tbody = $('flow-explorer-axis-tbody');
        if (!tbody) return;

        let data = null;
        try {
            const response = await fetch(`/api/v1/top-talkers?axis=${encodeURIComponent(state.filters.axis || 'ip')}&window=300`);
            data = await response.json();
        } catch (error) {
            console.warn('Top Talkers by axis load failed:', error);
        }
        const items = (data && data.items) || [];

        if (!items.length) {
            tbody.innerHTML = '<tr><td colspan="4" class="empty">No flow data for the current window.</td></tr>';
            if (chartHost && window.echarts) {
                if (!state.axisChart) state.axisChart = window.echarts.init(chartHost, 'dark');
                state.axisChart.setOption({ series: [{ type: 'pie', data: [] }] }, true);
            }
            return;
        }

        tbody.innerHTML = items.map((item, index) => `
      <tr>
        <td>${index + 1}</td>
        <td>${item.label}</td>
        <td>${formatBytes(item.bytes)}</td>
        <td>${Number(item.percentage || 0).toFixed(1)}%</td>
      </tr>
    `).join('');

        if (chartHost && window.echarts) {
            if (!state.axisChart) state.axisChart = window.echarts.init(chartHost, 'dark');
            state.axisChart.setOption({
                backgroundColor: 'transparent',
                tooltip: { trigger: 'item', formatter: '{b}: {c} ({d}%)' },
                legend: { show: false },
                series: [{
                    type: 'pie',
                    radius: ['45%', '75%'],
                    data: items.map((item, index) => ({
                        name: item.label,
                        value: item.bytes,
                        itemStyle: { color: AXIS_COLORS[index % AXIS_COLORS.length] }
                    })),
                    label: { color: '#e2e8f0', fontSize: 11 },
                    labelLine: { lineStyle: { color: '#475569' } }
                }]
            }, true);
        }
    }

    window.FlowExplorer = {
        applyFilters: function () {
            readFormIntoFilters();
            updateUrl();
            loadTimeline();
            loadFlows();
        },
        resetFilters: function () {
            state.filters = { src_ip: '', dst_ip: '', port: '', proto: '', if_index: '', time_start: '', time_end: '', compare_with: '' };
            syncFormFromFilters();
            updateUrl();
            loadTimeline();
            loadFlows();
        },
        setCompareMode: function (value) {
            state.filters.compare_with = value || '';
            updateUrl();
            loadTimeline();
        },
        setAxis: function (value) {
            state.filters.axis = value || 'ip';
            updateUrl();
            loadTopTalkersAxis();
        }
    };

    document.addEventListener('DOMContentLoaded', () => {
        readFiltersFromUrl();
        syncFormFromFilters();
        loadSankeyAndZones();
        loadTimeline();
        loadFlows();
        loadTopTalkersAxis();
    });

    window.addEventListener('resize', () => {
        if (state.chart) state.chart.resize();
        if (state.axisChart) state.axisChart.resize();
        if (state.sankeyChart) state.sankeyChart.resize();
    });
}());
