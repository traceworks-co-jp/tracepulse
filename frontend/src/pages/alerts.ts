import { esc as escapeHtml } from "../shared/escape";
import { fmtTime } from "../shared/format";

interface AlertHistoryItem {
    id: number;
    device_id: number;
    device_name: string;
    device_ip: string;
    alert_type: string;
    severity: string;
    details: string;
    created_at: string;
    acknowledged_at: string | null;
}

interface AlertHistoryPage {
    total: number;
    unacknowledged_count: number;
    items: AlertHistoryItem[];
}

interface AlertFilters {
    q: string;
    severity: string;
    status: string;
    days: string;
}

interface ApiResponse {
    ok?: boolean;
    error?: string;
    updated?: number;
    deleted?: number;
    acknowledged_at?: string;
}

const root = document.getElementById("alerts-page");
if (root) {
    let filters: AlertFilters = { q: "", severity: "", status: "all", days: "" };
    let limit = 20;
    let offset = 0;
    let result: AlertHistoryPage | undefined;
    let loading = false;
    let busy = false;
    let error = "";
    let message = "";
    let requestSequence = 0;

    const t = (key: string): string => typeof window.t === "function" ? window.t(key) : key;
    const tx = (key: string): string => escapeHtml(t(key));
    const filterParams = (pageOffset = offset, pageLimit = limit): URLSearchParams => {
        const params = new URLSearchParams({
            limit: String(pageLimit),
            offset: String(pageOffset),
            status: filters.status,
        });
        if (filters.q) params.set("q", filters.q);
        if (filters.severity) params.set("severity", filters.severity);
        if (filters.days) params.set("days", filters.days);
        return params;
    };
    const request = async <T>(url: string, init?: RequestInit): Promise<T> => {
        const headers = new Headers(init?.headers);
        headers.set("Accept", "application/json");
        if (init?.body !== undefined) headers.set("Content-Type", "application/json");
        Object.entries(window.tracepulseRequestHeaders?.() || {}).forEach(([name, value]) => headers.set(name, value));
        const response = await fetch(url, { ...init, headers });
        const body = await response.json() as T & ApiResponse;
        if (!response.ok || body.error) throw new Error(body.error || `HTTP ${response.status}`);
        return body;
    };
    const fetchPage = (pageOffset = offset, pageLimit = limit): Promise<AlertHistoryPage> =>
        request<AlertHistoryPage>(`/api/alerts?${filterParams(pageOffset, pageLimit)}`);

    const renderRows = (): string => {
        const items = result?.items || [];
        if (loading && !result) return `<tr><td class="empty" colspan="8">${tx("alerts_loading")}</td></tr>`;
        if (items.length === 0) return `<tr><td class="empty" colspan="8">${tx("alerts_no_results")}</td></tr>`;
        return items.map((item) => {
            const severity = ["critical", "warning", "info"].includes(item.severity) ? item.severity : "info";
            const status = item.acknowledged_at
                ? `<span class="alert-acknowledged">${tx("alerts_acknowledged")}</span>`
                : document.body.dataset.role === "viewer"
                    ? `<span>${tx("alerts_status_unacknowledged")}</span>`
                    : `<button class="btn alerts-ack-button" type="button" data-alert-ack="${item.id}" ${busy || loading ? "disabled" : ""}><span class="alerts-ack-icon" aria-hidden="true">✓</span>${tx("alerts_acknowledge")}</button>`;
            const acknowledgedAt = item.acknowledged_at
                ? `<time class="alerts-acknowledged-time" datetime="${escapeHtml(item.acknowledged_at)}">${escapeHtml(fmtTime(item.acknowledged_at))}</time>`
                : "-";
            return `<tr class="${item.acknowledged_at ? "alert-acknowledged-row" : "alert-unacknowledged"}">
                <td class="alerts-time-cell"><time datetime="${escapeHtml(item.created_at)}">${escapeHtml(fmtTime(item.created_at))}</time></td>
                <td class="alerts-device-cell"><a href="/device/${escapeHtml(encodeURIComponent(item.device_ip))}">${escapeHtml(item.device_name)}</a></td>
                <td class="alerts-ip-cell">${escapeHtml(item.device_ip)}</td>
                <td class="alerts-type-cell">${escapeHtml(item.alert_type)}</td>
                <td><span class="alert-severity sev-${severity}">${tx(`alerts_severity_${severity}`)}</span></td>
                <td class="alerts-details-cell">${escapeHtml(item.details)}</td>
                <td class="alerts-status-cell">${status}</td>
                <td class="alerts-acknowledged-at-cell">${acknowledgedAt}</td>
            </tr>`;
        }).join("");
    };

    const render = (): void => {
        if (!root) return;
        const total = result?.total || 0;
        const unacknowledged = result?.unacknowledged_count || 0;
        const first = total === 0 ? 0 : offset + 1;
        const last = Math.min(offset + limit, total);
        const showing = t("alerts_showing")
            .replace("{start}", String(first))
            .replace("{end}", String(last))
            .replace("{total}", String(total));
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
                <div class="alerts-filter-field"><label for="alerts-query">${tx("alerts_search")}</label><input id="alerts-query" name="q" type="search" value="${escapeHtml(filters.q)}" placeholder="${tx("alerts_search_placeholder")}"></div>
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
            ${message ? `<p class="alerts-message" role="status">${escapeHtml(message)}</p>` : ""}
            ${error ? `<p class="alerts-error" role="alert">${escapeHtml(error)}</p>` : ""}
            <div class="alerts-table-scroll" role="region" aria-label="${tx("alerts_title")}" tabindex="0">
                <table class="alerts-table"><thead><tr>
                    <th>${tx("alerts_time")}</th><th>${tx("alerts_device")}</th><th>${tx("alerts_ip")}</th>
                    <th>${tx("alerts_type")}</th><th>${tx("alerts_severity")}</th><th>${tx("alerts_details")}</th>
                    <th>${tx("alerts_status")}</th><th>${tx("alerts_acknowledged_at")}</th>
                </tr></thead><tbody>${renderRows()}</tbody></table>
            </div>
            <div class="alerts-pagination">
                <span id="alerts-range">${escapeHtml(showing)}</span>
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

        root.querySelector<HTMLFormElement>("#alerts-filter-form")?.addEventListener("submit", (event) => {
            event.preventDefault();
            const form = new FormData(event.currentTarget as HTMLFormElement);
            filters = {
                q: String(form.get("q") || "").trim(),
                severity: String(form.get("severity") || ""),
                status: String(form.get("status") || "all"),
                days: String(form.get("days") || ""),
            };
            offset = 0;
            message = "";
            void load();
        });
        root.querySelector<HTMLSelectElement>("#alerts-status")?.addEventListener("change", () => root.querySelector<HTMLFormElement>("#alerts-filter-form")?.requestSubmit());
        root.querySelector<HTMLSelectElement>("#alerts-severity")?.addEventListener("change", () => root.querySelector<HTMLFormElement>("#alerts-filter-form")?.requestSubmit());
        root.querySelector<HTMLSelectElement>("#alerts-days")?.addEventListener("change", () => root.querySelector<HTMLFormElement>("#alerts-filter-form")?.requestSubmit());
        root.querySelector<HTMLSelectElement>("#alerts-page-size")?.addEventListener("change", (event) => {
            limit = Number((event.currentTarget as HTMLSelectElement).value);
            offset = 0;
            void load();
        });
        root.querySelector<HTMLButtonElement>("#alerts-previous")?.addEventListener("click", () => {
            offset = Math.max(0, offset - limit);
            void load();
        });
        root.querySelector<HTMLButtonElement>("#alerts-next")?.addEventListener("click", () => {
            offset += limit;
            void load();
        });
        root.querySelector<HTMLButtonElement>("#alerts-ack-all")?.addEventListener("click", () => {
            void mutate("/api/alerts/ack-all", {
                severity: filters.severity || undefined,
                q: filters.q || undefined,
                days: filters.days ? Number(filters.days) : undefined,
            }, "acknowledge-all");
        });
        root.querySelector<HTMLButtonElement>("#alerts-export")?.addEventListener("click", () => { void exportCsv(); });
        root.querySelector<HTMLButtonElement>("#alerts-clear")?.addEventListener("click", () => {
            root.querySelector<HTMLDialogElement>("#alerts-clear-dialog")?.showModal();
        });
        root.querySelector<HTMLButtonElement>("#alerts-clear-cancel")?.addEventListener("click", () => {
            root.querySelector<HTMLDialogElement>("#alerts-clear-dialog")?.close();
        });
        root.querySelectorAll<HTMLButtonElement>("[data-clear-mode]").forEach((button) => {
            button.addEventListener("click", () => {
                root.querySelector<HTMLDialogElement>("#alerts-clear-dialog")?.close();
                void mutate("/api/alerts/clear", { mode: button.dataset.clearMode }, button.dataset.clearMode === "all" ? "clear-all" : "clear-acknowledged");
            });
        });
        root.querySelectorAll<HTMLButtonElement>("[data-alert-ack]").forEach((button) => {
            button.addEventListener("click", () => { void acknowledge(Number(button.dataset.alertAck)); });
        });
    };

    async function load(): Promise<void> {
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

    async function mutate(url: string, body: object, action: string): Promise<void> {
        if (busy) return;
        busy = true;
        error = "";
        message = "";
        render();
        try {
            const response = await request<ApiResponse>(url, { method: "POST", body: JSON.stringify(body) });
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

    async function acknowledge(id: number): Promise<void> {
        if (!Number.isInteger(id) || id <= 0) return;
        if (busy) return;
        busy = true;
        error = "";
        message = "";
        render();
        try {
            await request<ApiResponse>(`/api/alerts/${id}/ack`, { method: "POST", body: "{}" });
            message = t("alerts_acknowledged_one");
            result = await fetchPage();
        } catch (failure) {
            error = String(failure);
        } finally {
            busy = false;
            render();
        }
    }

    function csvCell(value: string): string {
        const safeValue = value !== "-" && /^[\s]*[=+\-@]/.test(value) ? `'${value}` : value;
        return `"${safeValue.replace(/"/g, '""')}"`;
    }

    async function exportCsv(): Promise<void> {
        if (busy) return;
        busy = true;
        error = "";
        message = "";
        render();
        try {
            const items: AlertHistoryItem[] = [];
            const total = result?.total || 0;
            for (let pageOffset = 0; pageOffset < total; pageOffset += 100) {
                const page = await fetchPage(pageOffset, 100);
                items.push(...page.items);
                if (page.items.length === 0) break;
            }
            const rows = [
                ["alerts_time", "alerts_device", "alerts_ip", "alerts_type", "alerts_severity", "alerts_details", "alerts_status", "alerts_acknowledged_at"]
                    .map((key) => csvCell(t(key))).join(","),
                ...items.map((item) => [
                    item.created_at,
                    item.device_name,
                    item.device_ip,
                    item.alert_type,
                    item.severity,
                    item.details,
                    t(item.acknowledged_at ? "alerts_acknowledged" : "alerts_status_unacknowledged"),
                    item.acknowledged_at || "-",
                ].map(csvCell).join(",")),
            ];
            const blob = new Blob([`\uFEFF${rows.join("\r\n")}`], { type: "text/csv;charset=utf-8" });
            const url = URL.createObjectURL(blob);
            const anchor = document.createElement("a");
            anchor.href = url;
            anchor.download = `tracepulse-alerts-${new Date().toISOString().slice(0, 10)}.csv`;
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
    window.addEventListener("tracepulse-role-change", render);
    render();
    void load();
}
