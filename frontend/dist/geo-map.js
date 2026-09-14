(function () {
  'use strict';

  const root = document.getElementById('geo-map');
  const panel = document.getElementById('geo-panel');
  if (!root) return;

  let chart = null;

  function initChart() {
    if (!window.echarts) {
      root.innerHTML = '<div style="color:#ef4444;padding:2rem;">ECharts library failed to load.</div>';
      return;
    }
    if (!chart) {
      chart = echarts.init(root, 'dark');
    }
  }

  function formatBytes(bytes) {
    if (!bytes || isNaN(bytes)) return '0 B';
    if (bytes >= 1073741824) return (bytes / 1073741824).toFixed(2) + ' GB';
    if (bytes >= 1048576) return (bytes / 1048576).toFixed(2) + ' MB';
    if (bytes >= 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return bytes.toLocaleString() + ' B';
  }

  // 同一または極近傍の座標（例: Private LANの複数端末など）を集約・クラスタリングする
  function clusterGeoPoints(points) {
    const clusterMap = new Map();

    points.forEach((p) => {
      if (p.longitude == null || p.latitude == null) return;
      // 小数第4位（約11m精度）で同一座標を判定
      const key = `${Number(p.latitude).toFixed(4)},${Number(p.longitude).toFixed(4)}`;
      const bytes = Number(p.traffic_bytes || p.bytes) || 0;

      if (!clusterMap.has(key)) {
        clusterMap.set(key, {
          key: key,
          longitude: Number(p.longitude),
          latitude: Number(p.latitude),
          country: p.country || 'Unknown',
          city: p.city || '',
          asn: p.asn,
          total_bytes: 0,
          hosts: []
        });
      }

      const cluster = clusterMap.get(key);
      cluster.total_bytes += bytes;
      cluster.hosts.push({
        ip: p.ip,
        country: p.country,
        city: p.city,
        asn: p.asn,
        traffic_bytes: bytes
      });
    });

    return Array.from(clusterMap.values()).map((c) => {
      c.hostCount = c.hosts.length;
      // トラフィック降順でホストをソート
      c.hosts.sort((a, b) => b.traffic_bytes - a.traffic_bytes);

      let displayName = c.country || c.city || (c.hosts[0] && c.hosts[0].ip) || 'Location';
      if (c.hostCount > 1) {
        displayName = `${displayName} (${c.hostCount} Hosts)`;
      }

      return {
        name: displayName,
        value: [c.longitude, c.latitude, Math.max(c.total_bytes, 1024)],
        meta: c
      };
    });
  }

  async function renderData() {
    if (!chart) return;
    try {
      const [geoResponse, sankeyResponse] = await Promise.all([
        fetch('/api/enterprise/geoip').then((r) => r.json()).catch(() => []),
        fetch('/api/enterprise/sankey').then((r) => r.json()).catch(() => ({ links: [] }))
      ]);

      const points = Array.isArray(geoResponse) ? geoResponse : (geoResponse.points || []);
      const scatterData = clusterGeoPoints(points);

      const linesData = [];
      for (let i = 1; i < scatterData.length && linesData.length < 50; i++) {
        const prev = scatterData[i - 1];
        const curr = scatterData[i];
        // 異なる座標間のみラインを結ぶ
        if (prev.value[0] !== curr.value[0] || prev.value[1] !== curr.value[1]) {
          linesData.push({
            fromName: prev.name,
            toName: curr.name,
            coords: [prev.value.slice(0, 2), curr.value.slice(0, 2)],
            value: curr.value[2]
          });
        }
      }

      chart.setOption({
        backgroundColor: '#0b1220',
        title: {
          text: '',
          left: 'center',
          textStyle: { color: '#fff' }
        },
        tooltip: {
          trigger: 'item',
          backgroundColor: 'rgba(15, 23, 42, 0.95)',
          borderColor: '#334155',
          borderWidth: 1,
          padding: 0,
          textStyle: { color: '#f8fafc' },
          formatter: function (params) {
            if (params.seriesType === 'effectScatter') {
              const meta = params.data.meta || {};
              const hosts = meta.hosts || [];
              const hostCount = meta.hostCount || hosts.length || 1;
              const totalMb = formatBytes(meta.total_bytes || params.data.value[2]);

              if (hostCount > 1) {
                const hostRows = hosts.slice(0, 8).map((h) => {
                  const hBytes = formatBytes(h.traffic_bytes);
                  return `<tr>
                    <td style="padding:3px 6px 3px 0;color:#38bdf8;font-family:monospace;font-size:11px;">${h.ip}</td>
                    <td style="text-align:right;padding:3px 0;color:#f8fafc;font-weight:600;font-size:11px;">${hBytes}</td>
                  </tr>`;
                }).join('');

                const overflowNote = hosts.length > 8
                  ? `<tr><td colspan="2" style="text-align:center;color:#64748b;font-size:10px;padding-top:4px;">+${hosts.length - 8} more hosts</td></tr>`
                  : '';

                return `<div style="font-size:12px;padding:8px 10px;min-width:220px;max-width:320px;line-height:1.4;">
                  <div style="font-weight:700;color:#38bdf8;font-size:13px;margin-bottom:4px;border-bottom:1px solid #334155;padding-bottom:4px;display:flex;justify-content:space-between;align-items:center;">
                    <span>${meta.country || 'Cluster'}</span>
                    <span style="font-size:10px;background:#0284c7;color:#fff;padding:1px 6px;border-radius:10px;font-weight:500;">${hostCount} Hosts</span>
                  </div>
                  <div style="color:#94a3b8;font-size:11px;margin-bottom:6px;">
                    Total: <strong style="color:#34d399;">${totalMb}</strong> | ASN: ${meta.asn ? 'AS' + meta.asn : 'Private/Local'}
                  </div>
                  <table style="width:100%;border-collapse:collapse;margin-top:4px;">
                    <thead>
                      <tr style="color:#64748b;border-bottom:1px solid #1e293b;font-size:10px;">
                        <th style="text-align:left;padding:2px 0;">Host IP</th>
                        <th style="text-align:right;padding:2px 0;">Traffic</th>
                      </tr>
                    </thead>
                    <tbody>
                      ${hostRows}
                      ${overflowNote}
                    </tbody>
                  </table>
                  <div style="margin-top:6px;font-size:10px;color:#64748b;text-align:right;">Click node for full details</div>
                </div>`;
              }

              const singleHost = hosts[0] || meta;
              return `<div style="font-size:12px;padding:8px 10px;min-width:180px;line-height:1.5;">
                <div style="font-weight:700;color:#38bdf8;font-size:13px;margin-bottom:4px;border-bottom:1px solid #334155;padding-bottom:3px;">
                  ${meta.country || singleHost.ip}
                </div>
                <div style="color:#cbd5e1;font-size:11px;"><strong>IP:</strong> <span style="font-family:monospace;color:#38bdf8;">${singleHost.ip}</span></div>
                <div style="color:#cbd5e1;font-size:11px;"><strong>ASN:</strong> ${meta.asn ? 'AS' + meta.asn : 'Private/Local'}</div>
                <div style="color:#cbd5e1;font-size:11px;"><strong>City:</strong> ${meta.city || '-'}</div>
                <div style="color:#cbd5e1;font-size:11px;margin-top:2px;"><strong>Traffic:</strong> <span style="color:#34d399;font-weight:700;">${totalMb}</span></div>
              </div>`;
            }
            if (params.seriesType === 'lines') {
              return `<div style="padding:4px 8px;font-size:12px;">${params.data.fromName} &rarr; ${params.data.toName}</div>`;
            }
            return params.name;
          }
        },
        geo: {
          map: 'world',
          roam: true,
          zoom: 1.2,
          aspectScale: 0.85,
          layoutCenter: ['50%', '50%'],
          layoutSize: '100%',
          itemStyle: {
            areaColor: '#17243b',
            borderColor: '#3b82f6',
            borderWidth: 0.6
          },
          emphasis: {
            itemStyle: {
              areaColor: '#1d4ed8'
            },
            label: {
              show: false
            }
          }
        },
        series: [
          {
            name: 'GeoNodes',
            type: 'effectScatter',
            coordinateSystem: 'geo',
            data: scatterData,
            symbolSize: function (val) {
              const b = val[2] || 0;
              return Math.max(10, Math.min(34, Math.sqrt(b) / 12));
            },
            showEffectOn: 'render',
            rippleEffect: {
              brushType: 'stroke',
              scale: 3.5,
              period: 4
            },
            label: {
              formatter: '{b}',
              position: 'right',
              show: true,
              color: '#f8fafc',
              fontSize: 11,
              fontWeight: 600,
              backgroundColor: 'rgba(11, 18, 32, 0.85)',
              borderColor: 'rgba(56, 189, 248, 0.45)',
              borderWidth: 1,
              borderRadius: 4,
              padding: [3, 7],
              distance: 8
            },
            itemStyle: {
              color: '#38bdf8',
              shadowBlur: 10,
              shadowColor: '#38bdf8'
            },
            zlevel: 2
          },
          {
            name: 'FlowLines',
            type: 'lines',
            coordinateSystem: 'geo',
            data: linesData,
            large: true,
            effect: {
              show: true,
              period: 4,
              trailLength: 0.4,
              symbol: 'arrow',
              symbolSize: 6
            },
            lineStyle: {
              color: '#f97316',
              width: 1.5,
              opacity: 0.5,
              curveness: 0.25
            },
            zlevel: 1
          }
        ]
      }, true);

      chart.off('click');
      chart.on('click', function (params) {
        if (!panel) return;
        const meta = params.data && params.data.meta;
        if (meta) {
          const hosts = meta.hosts || [];
          const hostCount = meta.hostCount || hosts.length || 1;
          const totalFormatted = formatBytes(meta.total_bytes);

          let hostListHtml = '';
          if (hostCount > 1) {
            hostListHtml = `
              <div style="margin-top:16px;border-top:1px solid #334155;padding-top:12px;">
                <h3 style="font-size:13px;color:#94a3b8;margin:0 0 8px;text-transform:uppercase;letter-spacing:0.5px;">Grouped Hosts (${hostCount})</h3>
                <div style="display:flex;flex-direction:column;gap:8px;max-height:260px;overflow-y:auto;">
                  ${hosts.map((h) => {
                    const pct = meta.total_bytes > 0 ? ((h.traffic_bytes / meta.total_bytes) * 100).toFixed(1) : '0.0';
                    return `
                      <div style="background:#0f172a;border:1px solid #1e293b;border-radius:4px;padding:8px 10px;">
                        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:4px;">
                          <span style="font-family:monospace;color:#38bdf8;font-weight:600;font-size:12px;">${h.ip}</span>
                          <span style="font-size:11px;color:#e2e8f0;font-weight:600;">${formatBytes(h.traffic_bytes)} (${pct}%)</span>
                        </div>
                        <div style="width:100%;height:4px;background:#1e293b;border-radius:2px;overflow:hidden;">
                          <div style="width:${pct}%;height:100%;background:#38bdf8;border-radius:2px;"></div>
                        </div>
                      </div>
                    `;
                  }).join('')}
                </div>
              </div>
            `;
          }

          panel.classList.add('open');
          panel.innerHTML = `
            <button class="geo-panel-close" onclick="document.getElementById('geo-panel').classList.remove('open')">&times; Close</button>
            <h2 style="margin:0 0 12px;color:#38bdf8;font-size:1.2rem;">${params.data.name}</h2>
            <div style="font-size:13px;line-height:1.7;color:#cbd5e1;">
              ${hostCount === 1 ? `<div><strong>IP Address:</strong> <span style="font-family:monospace;color:#38bdf8;">${hosts[0]?.ip || meta.ip}</span></div>` : `<div><strong>Total Hosts:</strong> ${hostCount}</div>`}
              <div><strong>Location:</strong> ${meta.city || '-'}, ${meta.country || '-'}</div>
              <div><strong>Autonomous System:</strong> ${meta.asn ? 'AS' + meta.asn : 'Private/Local'}</div>
              <div><strong>Total Traffic:</strong> <strong style="color:#34d399;">${totalFormatted}</strong></div>
            </div>
            ${hostListHtml}
            <div style="margin-top:20px;border-top:1px solid #334155;padding-top:14px;">
              <h3 style="font-size:13px;color:#94a3b8;margin:0 0 8px;">Active Flows</h3>
              <p style="font-size:12px;color:#64748b;margin:0;">Routing active on monitored interface.</p>
            </div>
          `;
        }
      });
    } catch (e) {
      console.error('Geo map render failed:', e);
    }
  }

  async function boot() {
    initChart();
    try {
      const worldGeoJson = await fetch('/static/world.json').then((r) => r.json());
      echarts.registerMap('world', worldGeoJson);
      await renderData();
      setInterval(renderData, 10000);
    } catch (e) {
      console.error('Failed to load world map:', e);
      if (root) root.innerHTML = '<div style="color:#ef4444;padding:2rem;">Failed to load map data.</div>';
    }
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot);
  } else {
    boot();
  }

  window.addEventListener('resize', function () {
    if (chart) chart.resize();
  });
}());
