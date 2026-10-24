import { esc } from "./escape";

if (!document.getElementById("metadata-styles")) {
    const style = document.createElement("style");
    style.id = "metadata-styles";
    style.textContent = `
        .metadata-dialog{box-sizing:border-box;width:min(640px,calc(100% - 24px));max-height:calc(100dvh - 24px);overflow:auto;margin:auto;padding:24px;border:1px solid var(--tp-border,#475569);border-radius:8px;background:var(--tp-surface,#1e293b);color:var(--tp-text,#e2e8f0)}
        .metadata-dialog::backdrop{background:rgb(0 0 0 / .6)}
        .metadata-dialog form,.metadata-dialog label{display:grid;gap:8px}
        .metadata-dialog p,.metadata-dialog li{overflow-wrap:anywhere}
        .metadata-dialog input,.metadata-dialog select,.device-toolbar input{box-sizing:border-box;width:100%;padding:8px;background:var(--tp-bg,#0f172a);color:inherit;border:1px solid #64748b;border-radius:4px;min-width:0}
        .metadata-dialog button,.device-toolbar button,#edit-device-metadata{padding:8px 12px;cursor:pointer;background:var(--tp-bg,#0f172a);color:inherit;border:1px solid #64748b;border-radius:4px}
        .metadata-dialog button:disabled,.device-toolbar button:disabled{opacity:.5;cursor:default}
        .metadata-actions,.device-toolbar,.device-tags{display:flex;gap:8px;flex-wrap:wrap;align-items:center}
        .metadata-actions{justify-content:flex-end;margin-top:12px}.device-toolbar{margin:12px 0}.device-toolbar input{flex:1 1 220px}
        .device-tag{display:inline-flex;align-items:center;gap:4px;padding:2px 7px;background:var(--tp-bg,#0f172a);border:1px solid #64748b;border-radius:12px;font-size:.8rem;overflow-wrap:anywhere}
        .device-tags{margin-top:4px}.device-tag button{padding:0 4px}.device-original{font-size:.8rem;color:var(--tp-muted,#94a3b8);overflow-wrap:anywhere}
        .metadata-dialog [role=alert],.device-toolbar [role=alert]{color:#f87171}
        :root[data-theme=light] .metadata-dialog{background:#fff;color:#1e293b}
        :root[data-theme=light] .metadata-dialog input,:root[data-theme=light] .metadata-dialog select,:root[data-theme=light] .metadata-dialog button,:root[data-theme=light] .device-toolbar input,:root[data-theme=light] .device-toolbar button,:root[data-theme=light] .device-tag,:root[data-theme=light] #edit-device-metadata{background:#f1f5f9;color:#1e293b}
    `;
    document.head.append(style);
}

export interface DeviceIdentity {
    id?: number | null;
    ip?: string;
    name?: string;
    display_name?: string | null;
    tags?: string[];
}

export function deviceLabel(device: DeviceIdentity): string {
    if (device.display_name?.trim()) return device.display_name;
    if (device.name?.trim()) return device.name;
    return device.ip || "";
}

const copy = {
    en: {
        edit: "Edit alias and tags", alias: "Display name", tags: "Tags", add: "Add tag", remove: "Remove",
        cancel: "Cancel", save: "Save", rename: "Bulk rename", prefix: "Prefix", start: "Start number",
        padding: "Padding", suffix: "Suffix", order: "Sort order", ip: "IP ascending", selection: "Selection order",
        preview: "Preview", apply: "Apply names", search: "Search alias, hostname, IP or tags", select: "Select device",
        all: "Select visible devices", selected: "selected", failed: "Request failed", missing: "Select at least one device",
        tagHint: "Enter a tag, then press Add tag or Enter.", clear: "Leave the display name empty to use sysName or IP.",
    },
    ja: {
        edit: "表示名・タグを編集", alias: "表示名", tags: "タグ", add: "タグを追加", remove: "削除",
        cancel: "キャンセル", save: "保存", rename: "一括リネーム（連番付与）", prefix: "接頭辞", start: "開始番号",
        padding: "ゼロ埋め桁数", suffix: "接尾辞", order: "並び順", ip: "IPアドレス昇順", selection: "選択順",
        preview: "プレビュー", apply: "名称を適用", search: "表示名・ホスト名・IP・タグで検索", select: "機器を選択",
        all: "表示中の機器をすべて選択", selected: "件選択", failed: "処理に失敗しました", missing: "機器を選択してください",
        tagHint: "タグを入力し、追加ボタンまたはEnterで確定します。", clear: "表示名を空にするとsysNameまたはIPを表示します。",
    },
};
export function metadataText(key: keyof typeof copy.en): string {
    return copy[document.documentElement.lang === "ja" ? "ja" : "en"][key];
}

async function post<T>(url: string, body: unknown): Promise<T> {
    const response = await fetch(url, {
        method: "POST", headers: { "Content-Type": "application/json", "X-CSRF-Token": window.tracepulseCsrf || "" },
        body: JSON.stringify(body),
    });
    const data: T & { error?: string; ok?: boolean } = await response.json();
    if (!response.ok || data.error || data.ok !== true) throw new Error(data.error || metadataText("failed"));
    return data;
}

function dialog(id: string, title: string): HTMLDialogElement {
    const existing = document.getElementById(id);
    if (existing instanceof HTMLDialogElement) existing.remove();
    const element = document.createElement("dialog");
    element.id = id;
    element.className = "metadata-dialog";
    element.setAttribute("aria-labelledby", `${id}-title`);
    element.innerHTML = `<h2 id="${id}-title">${esc(title)}</h2>`;
    document.body.append(element);
    return element;
}

function field(id: string, label: string, value: string, type = "text", limits = ""): string {
    return `<label for="${id}">${esc(label)}<input id="${id}" type="${type}" value="${esc(value)}" ${limits}></label>`;
}

export function editDeviceMetadata(device: DeviceIdentity, saved: () => void): void {
    if (!device.ip) return;
    const element = dialog("device-metadata-dialog", metadataText("edit"));
    let tags = [...(device.tags || [])];
    element.insertAdjacentHTML("beforeend", `<form>
        <p>${esc(device.name || "")} (${esc(device.ip)})</p>
        ${field("device-alias", metadataText("alias"), device.display_name || "", "text", "maxlength=512")}
        <p>${esc(metadataText("clear"))}</p>
        <label for="device-tag-input">${esc(metadataText("tags"))}</label>
        <div id="device-tag-chips" class="device-tags"></div>
        <input id="device-tag-input" maxlength="64" aria-describedby="device-tag-hint">
        <p id="device-tag-hint">${esc(metadataText("tagHint"))}</p>
        <button type="button" id="device-add-tag">${esc(metadataText("add"))}</button>
        <p role="alert"></p><div class="metadata-actions">
        <button type="button" data-cancel>${esc(metadataText("cancel"))}</button>
        <button type="submit">${esc(metadataText("save"))}</button></div></form>`);
    const input = element.querySelector<HTMLInputElement>("#device-tag-input")!;
    const error = element.querySelector<HTMLElement>("[role=alert]")!;
    const renderTags = (): void => {
        const chips = element.querySelector<HTMLElement>("#device-tag-chips")!;
        chips.replaceChildren();
        tags.forEach((tag) => {
            const chip = document.createElement("span");
            chip.className = "device-tag";
            chip.append(document.createTextNode(tag));
            const remove = document.createElement("button");
            remove.type = "button";
            remove.textContent = "×";
            remove.setAttribute("aria-label", `${metadataText("remove")} ${tag}`);
            remove.onclick = () => { tags = tags.filter((value) => value !== tag); renderTags(); };
            chip.append(remove);
            chips.append(chip);
        });
    };
    const addTag = (): void => {
        const tag = input.value.trim();
        if (tag && !tags.includes(tag)) tags.push(tag);
        input.value = "";
        renderTags();
    };
    element.querySelector<HTMLButtonElement>("#device-add-tag")!.onclick = addTag;
    input.addEventListener("keydown", (event) => {
        if (event.key === "Enter" && !event.isComposing) { event.preventDefault(); addTag(); }
    });
    element.querySelector<HTMLButtonElement>("[data-cancel]")!.onclick = () => element.close();
    element.querySelector<HTMLFormElement>("form")!.onsubmit = async (event) => {
        event.preventDefault();
        addTag();
        const submit = element.querySelector<HTMLButtonElement>("[type=submit]")!;
        submit.disabled = true;
        error.textContent = "";
        try {
            await post(`/api/device/${encodeURIComponent(device.ip!)}/metadata`, {
                display_name: element.querySelector<HTMLInputElement>("#device-alias")!.value || null, tags,
            });
            element.close();
            saved();
        } catch (failure) {
            error.textContent = String(failure);
        } finally { submit.disabled = false; }
    };
    renderTags();
    element.showModal();
}

interface RenameItem { id: number; ip: string; display_name: string; }
export function bulkRename(ids: number[], saved: () => void): void {
    const element = dialog("bulk-rename-dialog", metadataText("rename"));
    element.insertAdjacentHTML("beforeend", `<form>
        ${field("rename-prefix", metadataText("prefix"), "", "text", "maxlength=512")}
        ${field("rename-start", metadataText("start"), "1", "number", "min=0 max=4294967295 required")}
        ${field("rename-padding", metadataText("padding"), "2", "number", "min=0 max=10 required")}
        ${field("rename-suffix", metadataText("suffix"), "", "text", "maxlength=512")}
        <label for="rename-sort">${esc(metadataText("order"))}<select id="rename-sort">
        <option value="ip_asc">${esc(metadataText("ip"))}</option>
        <option value="selection_order">${esc(metadataText("selection"))}</option></select></label>
        <p role="alert"></p><ol id="rename-preview" aria-live="polite"></ol>
        <div class="metadata-actions"><button type="button" data-cancel>${esc(metadataText("cancel"))}</button>
        <button type="button" id="rename-preview-button">${esc(metadataText("preview"))}</button>
        <button type="submit" disabled>${esc(metadataText("apply"))}</button></div></form>`);
    const form = element.querySelector<HTMLFormElement>("form")!;
    const submit = element.querySelector<HTMLButtonElement>("[type=submit]")!;
    const previewButton = element.querySelector<HTMLButtonElement>("#rename-preview-button")!;
    const error = element.querySelector<HTMLElement>("[role=alert]")!;
    const preview = element.querySelector<HTMLOListElement>("#rename-preview")!;
    let approved: { device_ids: number[]; prefix: string; start_number: number; padding: number; suffix: string; sort_by: string } | null = null;
    let revision = 0;
    let busy = false;
    const invalidate = (): void => { revision++; approved = null; submit.disabled = true; preview.replaceChildren(); };
    form.addEventListener("input", invalidate);
    form.addEventListener("change", invalidate);
    element.querySelector<HTMLButtonElement>("[data-cancel]")!.onclick = () => { revision++; element.close(); };
    element.addEventListener("cancel", () => { revision++; });
    previewButton.onclick = async () => {
        if (busy || !form.reportValidity()) return;
        invalidate();
        const version = revision;
        const payload = {
            device_ids: ids,
            prefix: element.querySelector<HTMLInputElement>("#rename-prefix")!.value,
            start_number: Number(element.querySelector<HTMLInputElement>("#rename-start")!.value),
            padding: Number(element.querySelector<HTMLInputElement>("#rename-padding")!.value),
            suffix: element.querySelector<HTMLInputElement>("#rename-suffix")!.value,
            sort_by: element.querySelector<HTMLSelectElement>("#rename-sort")!.value,
        };
        busy = true; previewButton.disabled = true; error.textContent = "";
        try {
            const data = await post<{ devices: RenameItem[] }>("/api/devices/bulk-rename/preview", payload);
            if (version !== revision || !element.open) return;
            data.devices.forEach((device) => {
                const row = document.createElement("li");
                row.textContent = `${device.ip} → ${device.display_name}`;
                preview.append(row);
            });
            approved = payload;
            submit.disabled = false;
        } catch (failure) { error.textContent = String(failure); }
        finally { busy = false; previewButton.disabled = false; }
    };
    form.onsubmit = async (event) => {
        event.preventDefault();
        if (!approved || busy) return;
        busy = true; submit.disabled = true; previewButton.disabled = true; error.textContent = "";
        const inputs = form.querySelectorAll<HTMLInputElement | HTMLSelectElement>("input,select");
        inputs.forEach((input) => { input.disabled = true; });
        try {
            await post("/api/devices/bulk-rename", approved);
            element.close();
            saved();
        } catch (failure) { error.textContent = String(failure); invalidate(); }
        finally {
            busy = false; previewButton.disabled = false;
            inputs.forEach((input) => { input.disabled = false; });
        }
    };
    element.showModal();
}
