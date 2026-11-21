"use strict";
(() => {
  // frontend/src/shared/escape.ts
  function esc(value) {
    return String(value === null || value === void 0 ? "" : value).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/\x22/g, "&quot;").replace(/\x27/g, "&#39;");
  }

  // frontend/src/shared/format.ts
  function parseTracePulseTime(iso) {
    if (typeof iso !== "string" || iso.length === 0) return /* @__PURE__ */ new Date(NaN);
    if (/Z$|[+-]\d\d:\d\d$/.test(iso)) return new Date(iso);
    return /* @__PURE__ */ new Date(`${iso.replace(" ", "T")}Z`);
  }
  function tracePulseTimeZone() {
    try {
      return localStorage.getItem("tracepulse-timezone") === "jst" ? "Asia/Tokyo" : "UTC";
    } catch {
      return "UTC";
    }
  }
  function formatTracePulseTime(value) {
    const parts = new Intl.DateTimeFormat("en-US", {
      timeZone: tracePulseTimeZone(),
      month: "numeric",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false
    }).formatToParts(value);
    const map = {};
    parts.forEach((part) => {
      if (part.type !== "literal") map[part.type] = part.value;
    });
    return `${map.month}/${map.day} ${map.hour}:${map.minute}:${map.second}`;
  }
  function fmtTime(iso) {
    if (!iso) return "-";
    const date = parseTracePulseTime(iso);
    return Number.isNaN(date.getTime()) ? String(iso) : formatTracePulseTime(date);
  }

  // frontend/src/pages/alerts.ts
  var root = document.getElementById("alerts-page");
  if (root) {
    let csvCell = function(value) {
      const safeValue = value !== "-" && /^[\s]*[=+\-@]/.test(value) ? `'${value}` : value;
      return `"${safeValue.replace(/"/g, '""')}"`;
    };
    csvCell2 = csvCell;
    let filters = { q: "", severity: "", status: "all", days: "" };
    let limit = 20;
    let offset = 0;
    let result;
    let loading = false;
    let busy = false;
    let error = "";
    let message = "";
    let requestSequence = 0;
    const t = (key) => typeof window.t === "function" ? window.t(key) : key;
    const tx = (key) => esc(t(key));
    const filterParams = (pageOffset = offset, pageLimit = limit) => {
      const params = new URLSearchParams({
        limit: String(pageLimit),
        offset: String(pageOffset),
        status: filters.status
      });
      if (filters.q) params.set("q", filters.q);
      if (filters.severity) params.set("severity", filters.severity);
      if (filters.days) params.set("days", filters.days);
      return params;
    };
    const request = async (url, init) => {
      const headers = new Headers(init?.headers);
      headers.set("Accept", "application/json");
      if (init?.body !== void 0) headers.set("Content-Type", "application/json");
      Object.entries(window.tracepulseRequestHeaders?.() || {}).forEach(([name, value]) => headers.set(name, value));
      const response = await fetch(url, { ...init, headers });
      const body = await response.json();
      if (!response.ok || body.error) throw new Error(body.error || `HTTP ${response.status}`);
      return body;
    };
    const fetchPage = (pageOffset = offset, pageLimit = limit) => request(`/api/alerts?${filterParams(pageOffset, pageLimit)}`);
    const renderRows = () => {
      const items = result?.items || [];
      if (loading && !result) return `<tr><td class="empty" colspan="8">${tx("alerts_loading")}</td></tr>`;
      if (items.length === 0) return `<tr><td class="empty" colspan="8">${tx("alerts_no_results")}</td></tr>`;
      return items.map((item) => {
        const severity = ["critical", "warning", "info"].includes(item.severity) ? item.severity : "info";
        const status = item.acknowledged_at ? `<span class="alert-acknowledged">${tx("alerts_acknowledged")}</span>` : `<button class="btn alerts-ack-button" type="button" data-alert-ack="${item.id}" ${busy || loading ? "disabled" : ""}><span class="alerts-ack-icon" aria-hidden="true">\u2713</span>${tx("alerts_acknowledge")}</button>`;
        const acknowledgedAt = item.acknowledged_at ? `<time class="alerts-acknowledged-time" datetime="${esc(item.acknowledged_at)}">${esc(fmtTime(item.acknowledged_at))}</time>` : "-";
        return `<tr class="${item.acknowledged_at ? "alert-acknowledged-row" : "alert-unacknowledged"}">
                <td class="alerts-time-cell"><time datetime="${esc(item.created_at)}">${esc(fmtTime(item.created_at))}</time></td>
                <td class="alerts-device-cell"><a href="/device/${esc(encodeURIComponent(item.device_ip))}">${esc(item.device_name)}</a></td>
                <td class="alerts-ip-cell">${esc(item.device_ip)}</td>
                <td class="alerts-type-cell">${esc(item.alert_type)}</td>
                <td><span class="alert-severity sev-${severity}">${tx(`alerts_severity_${severity}`)}</span></td>
                <td class="alerts-details-cell">${esc(item.details)}</td>
                <td class="alerts-status-cell">${status}</td>
                <td class="alerts-acknowledged-at-cell">${acknowledgedAt}</td>
            </tr>`;
      }).join("");
    };
    const render = () => {
      if (!root) return;
      const total = result?.total || 0;
      const unacknowledged = result?.unacknowledged_count || 0;
      const first = total === 0 ? 0 : offset + 1;
      const last = Math.min(offset + limit, total);
      const showing = t("alerts_showing").replace("{start}", String(first)).replace("{end}", String(last)).replace("{total}", String(total));
      root.innerHTML = `
            <div class="alerts-overview">
                <div class="alerts-stats">
                    <div class="alerts-stat"><span>${tx("alerts_total")}</span><strong id="alerts-total">${total}</strong></div>
                    <div class="alerts-stat"><span>${tx("alerts_unacknowledged_count")}</span><strong id="alerts-unacknowledged">${unacknowledged}</strong></div>
                </div>
                <div class="alerts-toolbar">
                    <button class="btn btn-primary" id="alerts-ack-all" type="button" ${busy || loading || unacknowledged === 0 || filters.status === "acknowledged" ? "disabled" : ""}>${tx("alerts_acknowledge_all")}</button>
                    <button class="btn btn-secondary" id="alerts-export" type="button" ${busy || loading ? "disabled" : ""}>${tx("alerts_export_csv")}</button>
                    <button class="btn btn-secondary" id="alerts-clear" type="button" ${busy || loading || total === 0 ? "disabled" : ""}>${tx("alerts_clear_history")}</button>
                </div>
            </div>
            <form class="alerts-filter-form" id="alerts-filter-form">
                <div class="alerts-filter-field"><label for="alerts-query">${tx("alerts_search")}</label><input id="alerts-query" name="q" type="search" value="${esc(filters.q)}" placeholder="${tx("alerts_search_placeholder")}"></div>
                <div class="alerts-filter-field"><label for="alerts-severity">${tx("alerts_severity")}</label><select id="alerts-severity" name="severity">
                    <option value="" ${filters.severity === "" ? "selected" : ""}>${tx("alerts_severity_all")}</option>
                    <option value="critical" ${filters.severity === "critical" ? "selected" : ""}>${tx("alerts_severity_critical")}</option>
                    <option value="warning" ${filters.severity === "warning" ? "selected" : ""}>${tx("alerts_severity_warning")}</option>
                    <option value="info" ${filters.severity === "info" ? "selected" : ""}>${tx("alerts_severity_info")}</option>
                </select></div>
                <div class="alerts-filter-field"><label for="alerts-status">${tx("alerts_status")}</label><select id="alerts-status" name="status">
                    <option value="all" ${filters.status === "all" ? "selected" : ""}>${tx("alerts_status_all")}</option>
                    <option value="unacknowledged" ${filters.status === "unacknowledged" ? "selected" : ""}>${tx("alerts_status_unacknowledged")}</option>
                    <option value="acknowledged" ${filters.status === "acknowledged" ? "selected" : ""}>${tx("alerts_status_acknowledged")}</option>
                </select></div>
                <div class="alerts-filter-field"><label for="alerts-days">${tx("alerts_period")}</label><select id="alerts-days" name="days">
                    <option value="" ${filters.days === "" ? "selected" : ""}>${tx("alerts_period_all")}</option>
                    <option value="1" ${filters.days === "1" ? "selected" : ""}>${tx("alerts_period_24h")}</option>
                    <option value="7" ${filters.days === "7" ? "selected" : ""}>${tx("alerts_period_7d")}</option>
                    <option value="30" ${filters.days === "30" ? "selected" : ""}>${tx("alerts_period_30d")}</option>
                </select></div>
                <button class="btn btn-primary" type="submit">${tx("alerts_apply_filters")}</button>
            </form>
            ${message ? `<p class="alerts-message" role="status">${esc(message)}</p>` : ""}
            ${error ? `<p class="alerts-error" role="alert">${esc(error)}</p>` : ""}
            <div class="alerts-table-scroll" role="region" aria-label="${tx("alerts_title")}" tabindex="0">
                <table class="alerts-table"><thead><tr>
                    <th>${tx("alerts_time")}</th><th>${tx("alerts_device")}</th><th>${tx("alerts_ip")}</th>
                    <th>${tx("alerts_type")}</th><th>${tx("alerts_severity")}</th><th>${tx("alerts_details")}</th>
                    <th>${tx("alerts_status")}</th><th>${tx("alerts_acknowledged_at")}</th>
                </tr></thead><tbody>${renderRows()}</tbody></table>
            </div>
            <div class="alerts-pagination">
                <span id="alerts-range">${esc(showing)}</span>
                <div class="alerts-page-controls">
                    <label for="alerts-page-size">${tx("alerts_page_size")}
                        <select id="alerts-page-size" ${busy || loading ? "disabled" : ""}>
                            <option value="20" ${limit === 20 ? "selected" : ""}>20</option>
                            <option value="50" ${limit === 50 ? "selected" : ""}>50</option>
                            <option value="100" ${limit === 100 ? "selected" : ""}>100</option>
                        </select>
                    </label>
                    <button class="btn btn-secondary" id="alerts-previous" type="button" ${busy || loading || offset === 0 ? "disabled" : ""}>${tx("alerts_previous")}</button>
                    <button class="btn btn-secondary" id="alerts-next" type="button" ${busy || loading || offset + limit >= total ? "disabled" : ""}>${tx("alerts_next")}</button>
                </div>
            </div>
            <dialog class="alerts-dialog" id="alerts-clear-dialog">
                <h2>${tx("alerts_clear_dialog_title")}</h2>
                <p>${tx("alerts_clear_dialog_body")}</p>
                <div class="alerts-dialog-actions">
                    <button class="btn btn-secondary" type="button" data-clear-mode="acknowledged">${tx("alerts_clear_acknowledged")}</button>
                    <button class="btn btn-row-danger" type="button" data-clear-mode="all">${tx("alerts_clear_all")}</button>
                    <button class="btn btn-secondary" type="button" id="alerts-clear-cancel">${tx("alerts_cancel")}</button>
                </div>
            </dialog>`;
      root.querySelector("#alerts-filter-form")?.addEventListener("submit", (event) => {
        event.preventDefault();
        const form = new FormData(event.currentTarget);
        filters = {
          q: String(form.get("q") || "").trim(),
          severity: String(form.get("severity") || ""),
          status: String(form.get("status") || "all"),
          days: String(form.get("days") || "")
        };
        offset = 0;
        message = "";
        void load();
      });
      root.querySelector("#alerts-status")?.addEventListener("change", () => root.querySelector("#alerts-filter-form")?.requestSubmit());
      root.querySelector("#alerts-severity")?.addEventListener("change", () => root.querySelector("#alerts-filter-form")?.requestSubmit());
      root.querySelector("#alerts-days")?.addEventListener("change", () => root.querySelector("#alerts-filter-form")?.requestSubmit());
      root.querySelector("#alerts-page-size")?.addEventListener("change", (event) => {
        limit = Number(event.currentTarget.value);
        offset = 0;
        void load();
      });
      root.querySelector("#alerts-previous")?.addEventListener("click", () => {
        offset = Math.max(0, offset - limit);
        void load();
      });
      root.querySelector("#alerts-next")?.addEventListener("click", () => {
        offset += limit;
        void load();
      });
      root.querySelector("#alerts-ack-all")?.addEventListener("click", () => {
        void mutate("/api/alerts/ack-all", {
          severity: filters.severity || void 0,
          q: filters.q || void 0,
          days: filters.days ? Number(filters.days) : void 0
        }, "acknowledge-all");
      });
      root.querySelector("#alerts-export")?.addEventListener("click", () => {
        void exportCsv();
      });
      root.querySelector("#alerts-clear")?.addEventListener("click", () => {
        root.querySelector("#alerts-clear-dialog")?.showModal();
      });
      root.querySelector("#alerts-clear-cancel")?.addEventListener("click", () => {
        root.querySelector("#alerts-clear-dialog")?.close();
      });
      root.querySelectorAll("[data-clear-mode]").forEach((button) => {
        button.addEventListener("click", () => {
          root.querySelector("#alerts-clear-dialog")?.close();
          void mutate("/api/alerts/clear", { mode: button.dataset.clearMode }, button.dataset.clearMode === "all" ? "clear-all" : "clear-acknowledged");
        });
      });
      root.querySelectorAll("[data-alert-ack]").forEach((button) => {
        button.addEventListener("click", () => {
          void acknowledge(Number(button.dataset.alertAck));
        });
      });
    };
    async function load() {
      const sequence = ++requestSequence;
      loading = true;
      error = "";
      render();
      try {
        let next = await fetchPage();
        if (sequence !== requestSequence) return;
        if (offset > 0 && offset >= next.total && next.total > 0) {
          offset = Math.floor((next.total - 1) / limit) * limit;
          next = await fetchPage();
        } else if (next.total === 0) {
          offset = 0;
        }
        if (sequence === requestSequence) result = next;
      } catch (failure) {
        if (sequence === requestSequence) error = String(failure);
      } finally {
        if (sequence === requestSequence) {
          loading = false;
          render();
        }
      }
    }
    async function mutate(url, body, action) {
      if (busy) return;
      busy = true;
      error = "";
      message = "";
      render();
      try {
        const response = await request(url, { method: "POST", body: JSON.stringify(body) });
        if (action === "acknowledge-all") {
          message = t("alerts_acknowledged_many").replace("{count}", String(response.updated || 0));
        } else if (action === "clear-acknowledged") {
          message = t("alerts_acknowledged_cleared");
        } else if (action === "clear-all") {
          message = t("alerts_history_cleared");
        }
        result = await fetchPage();
        if (result.total === 0) offset = 0;
        else if (offset >= result.total) {
          offset = Math.floor((result.total - 1) / limit) * limit;
          result = await fetchPage();
        }
      } catch (failure) {
        error = String(failure);
      } finally {
        busy = false;
        render();
      }
    }
    async function acknowledge(id) {
      if (!Number.isInteger(id) || id <= 0) return;
      if (busy) return;
      busy = true;
      error = "";
      message = "";
      render();
      try {
        await request(`/api/alerts/${id}/ack`, { method: "POST", body: "{}" });
        message = t("alerts_acknowledged_one");
        result = await fetchPage();
      } catch (failure) {
        error = String(failure);
      } finally {
        busy = false;
        render();
      }
    }
    async function exportCsv() {
      if (busy) return;
      busy = true;
      error = "";
      message = "";
      render();
      try {
        const items = [];
        const total = result?.total || 0;
        for (let pageOffset = 0; pageOffset < total; pageOffset += 100) {
          const page = await fetchPage(pageOffset, 100);
          items.push(...page.items);
          if (page.items.length === 0) break;
        }
        const rows = [
          ["alerts_time", "alerts_device", "alerts_ip", "alerts_type", "alerts_severity", "alerts_details", "alerts_status", "alerts_acknowledged_at"].map((key) => csvCell(t(key))).join(","),
          ...items.map((item) => [
            item.created_at,
            item.device_name,
            item.device_ip,
            item.alert_type,
            item.severity,
            item.details,
            t(item.acknowledged_at ? "alerts_acknowledged" : "alerts_status_unacknowledged"),
            item.acknowledged_at || "-"
          ].map(csvCell).join(","))
        ];
        const blob = new Blob([`\uFEFF${rows.join("\r\n")}`], { type: "text/csv;charset=utf-8" });
        const url = URL.createObjectURL(blob);
        const anchor = document.createElement("a");
        anchor.href = url;
        anchor.download = `tracepulse-alerts-${(/* @__PURE__ */ new Date()).toISOString().slice(0, 10)}.csv`;
        document.body.append(anchor);
        anchor.click();
        anchor.remove();
        URL.revokeObjectURL(url);
      } catch (failure) {
        error = String(failure);
      } finally {
        busy = false;
        render();
      }
    }
    window.addEventListener("tracepulse-language-change", render);
    render();
    void load();
  }
  var csvCell2;
})();
