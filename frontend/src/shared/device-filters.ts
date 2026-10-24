import { esc } from "./escape";

interface FilterDevice {
    ip: string;
    display_name?: string | null;
    name?: string;
    tags?: string[];
    status?: string;
    health_score?: number | null;
}

const copy = {
    en: {
        title: "Column filters (AND)", add: "Add condition", clear: "Clear conditions",
        field: "Column", operator: "Condition", value: "Value", remove: "Remove condition",
        ip: "IP address", display_name: "Display name", name: "Hostname (sysName)", tags: "Tags",
        status: "Status", health_score: "Health score",
        contains: "Contains", equals: "Equals", not_equals: "Does not equal", starts_with: "Starts with",
        gte: "At least (≥)", lte: "At most (≤)", gt: "Greater than (>)", lt: "Less than (<)",
        hint: "All conditions and the search must match. Empty values are ignored. Text matching is case-insensitive; tags match individually.",
        invalid: "Health score must be a number from 0 to 100.",
    },
    ja: {
        title: "カラム別フィルター（AND）", add: "条件を追加", clear: "条件をクリア",
        field: "対象カラム", operator: "条件", value: "値", remove: "条件を削除",
        ip: "IPアドレス", display_name: "表示名", name: "ホスト名（sysName）", tags: "タグ",
        status: "状態", health_score: "ヘルススコア",
        contains: "含む", equals: "一致する", not_equals: "一致しない", starts_with: "で始まる",
        gte: "以上（≥）", lte: "以下（≤）", gt: "より大きい（>）", lt: "より小さい（<）",
        hint: "すべての条件と検索に一致する機器を表示します。空欄は無視します。英字の大文字・小文字は区別せず、タグは個別に照合します。",
        invalid: "ヘルススコアは0から100の数値を入力してください。",
    },
};
type Field = keyof FilterDevice;
type Operator = "contains" | "equals" | "not_equals" | "starts_with" | "gte" | "lte" | "gt" | "lt";
interface Condition { field: Field; operator: Operator; value: string; }
const fields: Field[] = ["ip", "display_name", "name", "tags", "status", "health_score"];
const statuses = ["online", "warning", "critical", "offline", "unknown"];
const text = (key: keyof typeof copy.en): string => copy[document.documentElement.lang === "ja" ? "ja" : "en"][key];
const operators = (field: Field): Operator[] => field === "health_score"
    ? ["gte", "lte", "equals", "not_equals", "gt", "lt"]
    : field === "status" ? ["equals", "not_equals"] : ["contains", "equals", "not_equals", "starts_with"];

export function initializeDeviceFilters(before: HTMLElement, changed: () => void): (device: FilterDevice) => boolean {
    const conditions: Condition[] = [];
    const section = document.createElement("section");
    section.id = "device-filters";
    section.setAttribute("aria-labelledby", "device-filters-title");
    const style = document.createElement("style");
    style.textContent = `
        #device-filters{margin:12px 0}
        .device-filter-row{display:flex;flex-wrap:wrap;gap:8px;align-items:end;margin:8px 0}
        .device-filter-row label{display:grid;gap:4px;flex:1 1 150px;min-width:0}
        .device-filter-row input,.device-filter-row select,#device-filters button{box-sizing:border-box;max-width:100%;padding:8px;border:1px solid #64748b;border-radius:4px;background:var(--tp-bg,#0f172a);color:inherit}
        .device-filter-row input,.device-filter-row select{width:100%;min-width:0}
        #device-filters button{cursor:pointer}#device-filters p{font-size:.85rem}
        #device-filters [role=alert]{color:#f87171}
        :root[data-theme=light] #device-filters input,:root[data-theme=light] #device-filters select,:root[data-theme=light] #device-filters button{background:#f1f5f9;color:#1e293b}
    `;
    document.head.append(style);
    before.before(section);
    const invalid = (condition: Condition): boolean => condition.field === "health_score" && condition.value.trim() !== "" &&
        (!Number.isFinite(Number(condition.value)) || Number(condition.value) < 0 || Number(condition.value) > 100);
    const update = (): void => {
        section.querySelector<HTMLElement>("[role=alert]")!.textContent = conditions.some(invalid) ? text("invalid") : "";
        changed();
    };
    const render = (): void => {
        section.innerHTML = `<h3 id="device-filters-title">${esc(text("title"))}</h3><p>${esc(text("hint"))}</p><div id="device-filter-rows"></div><button type="button" id="device-filter-add">${esc(text("add"))}</button> <button type="button" id="device-filter-clear">${esc(text("clear"))}</button><p role="alert"></p>`;
        const rows = section.querySelector<HTMLElement>("#device-filter-rows")!;
        conditions.forEach((condition, index) => {
            const row = document.createElement("div");
            row.className = "device-filter-row";
            const options = (values: string[], selected: string, labels: boolean): string => values.map((value) =>
                `<option value="${esc(value)}" ${value === selected ? "selected" : ""}>${esc(labels ? text(value as keyof typeof copy.en) : value)}</option>`).join("");
            row.innerHTML = `<label>${esc(text("field"))}<select data-filter-field>${options(fields, condition.field, true)}</select></label>
                <label>${esc(text("operator"))}<select data-filter-operator>${options(operators(condition.field), condition.operator, true)}</select></label>
                <label>${esc(text("value"))}${condition.field === "status"
                    ? `<select data-filter-value><option value="">-</option>${options(statuses, condition.value, false)}</select>`
                    : `<input data-filter-value type="text" ${condition.field === "health_score" ? 'inputmode="decimal"' : ""} value="${esc(condition.value)}">`}</label>
                <button type="button" data-filter-remove>${esc(text("remove"))}</button>`;
            row.querySelector<HTMLSelectElement>("[data-filter-field]")!.onchange = (event) => {
                condition.field = (event.currentTarget as HTMLSelectElement).value as Field;
                condition.operator = operators(condition.field)[0];
                condition.value = "";
                render(); update();
                section.querySelectorAll<HTMLElement>("[data-filter-field]")[index]?.focus();
            };
            row.querySelector<HTMLSelectElement>("[data-filter-operator]")!.onchange = (event) => {
                condition.operator = (event.currentTarget as HTMLSelectElement).value as Operator;
                update();
            };
            row.querySelector<HTMLInputElement | HTMLSelectElement>("[data-filter-value]")!.oninput = (event) => {
                condition.value = (event.currentTarget as HTMLInputElement | HTMLSelectElement).value;
                update();
            };
            row.querySelector<HTMLButtonElement>("[data-filter-remove]")!.onclick = () => {
                conditions.splice(index, 1); render(); update();
                section.querySelector<HTMLButtonElement>("#device-filter-add")!.focus();
            };
            rows.append(row);
        });
        section.querySelector<HTMLButtonElement>("#device-filter-add")!.onclick = () => {
            conditions.push({ field: "ip", operator: "contains", value: "" });
            render(); update();
            section.querySelectorAll<HTMLElement>("[data-filter-field]")[conditions.length - 1]?.focus();
        };
        section.querySelector<HTMLButtonElement>("#device-filter-clear")!.onclick = () => {
            conditions.length = 0; render(); update();
            section.querySelector<HTMLButtonElement>("#device-filter-add")!.focus();
        };
        section.querySelector<HTMLElement>("[role=alert]")!.textContent = conditions.some(invalid) ? text("invalid") : "";
    };
    render();
    window.addEventListener("tracepulse-language-change", render);
    return (device) => conditions.every((condition) => {
        const value = condition.value.trim().toLocaleLowerCase();
        if (!value) return true;
        if (invalid(condition)) return false;
        if (condition.field === "health_score") {
            const score = device.health_score;
            if (typeof score !== "number" || !Number.isFinite(score)) return false;
            const target = Number(value);
            switch (condition.operator) {
                case "gte": return score >= target;
                case "lte": return score <= target;
                case "gt": return score > target;
                case "lt": return score < target;
                case "not_equals": return score !== target;
                default: return score === target;
            }
        }
        const raw = device[condition.field];
        const values = (Array.isArray(raw) ? raw : [raw || ""]).map((item) => item.toLocaleLowerCase());
        if (condition.operator === "not_equals") return values.every((item) => item !== value);
        return values.some((item) => condition.operator === "contains" ? item.includes(value)
            : condition.operator === "starts_with" ? item.startsWith(value) : item === value);
    });
}
