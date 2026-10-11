import { esc } from "./escape";

interface FoundDevice { ip: string; name?: string; community?: string; }
interface ScanResult { status?: string; error?: string; scanned?: number; total?: number; devices?: FoundDevice[]; }

const storageKey = "tracepulse-onboarding-v1";
const copy = {
    en: {
        guide: "Setup Guide", welcome: "Welcome to TracePulse", intro: "Start monitoring your network by discovering and registering SNMP devices.",
        discovery: "Discover and register devices", cidr: "IP range (IPv4 CIDR)", community: "SNMP v2c community", scan: "Discover", scanning: "Discovering devices...", next: "Next", back: "Back", skip: "Skip", finish: "Finish", register: "Register selected devices", empty: "No SNMP devices found. Check the range, community and connectivity, then try again.",
        selected: "Select devices to monitor", registered: "Devices registered. Monitoring starts automatically.", alerts: "Alerts (optional)", alertInfo: "TracePulse records alerts locally and displays them on the dashboard. Adjust thresholds in Settings when needed. Outbound notifications are not available in Community edition.", settings: "Alert settings", rangeError: "Enter one valid IPv4 CIDR range at /24 or narrower.", failure: "The operation failed. Please retry.", timeout: "Discovery timed out. Check connectivity and retry.", progress: "Discovery progress", step: "Step", continue: "Continue without registration", privacy: "The community is used for this discovery and registered devices only.",
    },
    ja: {
        guide: "セットアップガイド", welcome: "TracePulseへようこそ", intro: "ネットワーク内のSNMP機器を探索・登録して、監視を始めましょう。",
        discovery: "機器の探索・登録", cidr: "IP範囲（IPv4 CIDR）", community: "SNMP v2cコミュニティ", scan: "探索する", scanning: "機器を探索中...", next: "次へ", back: "戻る", skip: "スキップ", finish: "完了", register: "選択した機器を登録", empty: "SNMP機器が見つかりませんでした。IP範囲・コミュニティ・接続を確認して再探索してください。",
        selected: "監視する機器を選択", registered: "機器を登録しました。監視は自動的に開始されます。", alerts: "アラート設定（任意）", alertInfo: "TracePulseはアラートをローカルに記録し、ダッシュボードに表示します。必要に応じて設定画面でしきい値を変更できます。Community版では外部への通知送信は利用できません。", settings: "アラート設定へ", rangeError: "/24以上のプレフィックスで、有効なIPv4 CIDRを1つ入力してください。", failure: "処理に失敗しました。再試行してください。", timeout: "探索がタイムアウトしました。接続を確認して再試行してください。", progress: "探索の進捗", step: "ステップ", continue: "登録せずに次へ", privacy: "コミュニティは今回の探索と登録する機器にのみ使用します。",
    },
};

export function initializeOnboarding(auto = false): void {
    const text = (key: keyof typeof copy.en): string => copy[document.documentElement.lang === "ja" ? "ja" : "en"][key];
    const style = document.createElement("style");
    style.textContent = `
        .setup-dialog{box-sizing:border-box;inset:0;margin:auto;width:min(600px,calc(100% - 24px));max-height:calc(100dvh - 24px);padding:24px;border:1px solid var(--border,#475569);border-radius:8px;background:var(--panel-bg,#1e293b);color:var(--text,#e2e8f0);overflow:auto}
        .setup-dialog::backdrop{background:rgb(0 0 0 / .6)}
        .setup-dialog h2{font-size:1.25rem;margin:12px 0}.setup-dialog p{line-height:1.6;overflow-wrap:anywhere}
        .setup-dialog form{display:grid;gap:12px}.setup-dialog label{display:grid;gap:6px}
        .setup-dialog input:not([type=checkbox]),.setup-dialog select{box-sizing:border-box;width:100%;min-width:0;padding:10px;border-radius:4px;border:1px solid var(--border,#64748b);background:var(--input-bg,#0f172a);color:inherit}
        .setup-actions{display:flex;gap:8px;flex-wrap:wrap;justify-content:flex-end;margin-top:20px}
        .setup-dialog button,.setup-launch{padding:8px 14px;cursor:pointer;border-radius:4px;border:1px solid var(--border,#64748b);background:var(--input-bg,#0f172a);color:inherit;white-space:normal}
        .setup-dialog button:disabled{opacity:.5;cursor:default}.setup-dialog .setup-primary{background:#0369a1;color:white;border-color:#0369a1}
        .setup-dialog .setup-results{display:grid;gap:8px;max-height:220px;overflow:auto;padding:8px 0}
        .setup-dialog .setup-device{display:flex;align-items:center;gap:10px;overflow-wrap:anywhere}.setup-dialog progress{width:100%}
        .setup-dialog [role=alert]{color:#f87171}.setup-step{font-size:.85rem}.setup-launch{margin-bottom:16px}
        :root[data-theme=light] .setup-dialog{background:#fff;color:#1e293b}:root[data-theme=light] .setup-dialog input,:root[data-theme=light] .setup-dialog select,:root[data-theme=light] .setup-dialog button:not(.setup-primary),:root[data-theme=light] .setup-launch{background:#f1f5f9;color:#1e293b}
        @media(max-width:480px){.setup-dialog{padding:16px}.setup-actions button{flex:1 1 auto}}
    `;
    document.head.append(style);
    const dialog = document.createElement("dialog");
    dialog.className = "setup-dialog";
    dialog.id = "setup-guide";
    dialog.setAttribute("aria-labelledby", "setup-title");
    document.body.append(dialog);
    let step = 0;
    let choosingLanguage = false;
    let languageChoiceEnabled = false;
    let busy = false;
    let devices: FoundDevice[] = [];
    let registered = false;
    let controller = new AbortController();
    let timer: number | undefined;
    let deadline: number | undefined;
    let previousFocus: HTMLElement | null = null;
    let dismissed = false;
    try { dismissed = localStorage.getItem(storageKey) === "done"; } catch { /* ignore */ }

    const stop = (): void => {
        controller.abort();
        window.clearTimeout(timer);
        window.clearTimeout(deadline);
        controller = new AbortController();
        busy = false;
    };
    const remember = (): void => {
        dismissed = true;
        if (auto) launch.hidden = true;
        try { localStorage.setItem(storageKey, "done"); } catch { /* ignore */ }
    };
    const close = (): void => {
        stop();
        remember();
        dialog.close();
        dialog.replaceChildren();
        devices = [];
        if (previousFocus?.hidden) {
            const heading = document.querySelector<HTMLElement>("main h1");
            heading?.setAttribute("tabindex", "-1");
            heading?.focus();
        } else previousFocus?.focus();
        window.refreshDashboard?.();
    };
    const status = (message: string, error = false): void => {
        const node = dialog.querySelector<HTMLElement>("#setup-status");
        if (node) { node.textContent = message; node.setAttribute("role", error ? "alert" : "status"); }
    };
    const setBusy = (value: boolean): void => {
        busy = value;
        dialog.querySelectorAll<HTMLButtonElement>("button:not([data-action=skip])").forEach((button) => { button.disabled = value; });
        dialog.querySelectorAll<HTMLInputElement>("input").forEach((input) => { input.disabled = value; });
    };
    const request = async <Result>(url: string, signal: AbortSignal, body?: URLSearchParams | FoundDevice[]): Promise<Result> => {
        const response = await fetch(url, {
            signal,
            ...(body ? { method: "POST", headers: { "Content-Type": body instanceof URLSearchParams ? "application/x-www-form-urlencoded" : "application/json", "X-CSRF-Token": window.tracepulseCsrf || "" }, body: body instanceof URLSearchParams ? body.toString() : JSON.stringify(body) } : {}),
        });
        const data = await response.json();
        if (!response.ok || data.error) throw new Error(data.error || `HTTP ${response.status}`);
        return data as Result;
    };
    const render = (): void => {
        if (choosingLanguage) {
            dialog.innerHTML = `<h2 id="setup-title" tabindex="-1">Choose Language</h2><form id="setup-language-form"><label for="setup-language">Language</label><select id="setup-language" required><option value="" disabled selected>Select a language</option><option value="ja">Japanese</option><option value="en">English</option></select><div class="setup-actions"><button type="submit" class="setup-primary">Continue</button></div></form>`;
            dialog.querySelector<HTMLSelectElement>("#setup-language")?.focus();
            dialog.querySelector("form")?.addEventListener("submit", (event) => {
                event.preventDefault();
                const language = dialog.querySelector<HTMLSelectElement>("#setup-language")!.value;
                if (language !== "ja" && language !== "en") return;
                choosingLanguage = false;
                window.setLanguage?.(language);
                window.refreshDashboard?.();
                render();
            });
            return;
        }
        const titles = ["welcome", "discovery", "alerts"] as const;
        dialog.innerHTML = `<div class="setup-step">${text("step")} ${step + 1} / 3</div><h2 id="setup-title" tabindex="-1">${text(titles[step])}</h2>` +
            (step === 0 ? `<p>${text("intro")}</p>` : step === 1 ?
                `<form id="setup-form"><label>${text("cidr")}<input id="setup-cidr" required placeholder="192.168.1.0/24" autocomplete="off"></label><label>${text("community")}<input id="setup-community" type="password" required value="public" autocomplete="off"></label><p>${text("privacy")}</p><button class="setup-primary" type="submit">${text("scan")}</button></form><progress id="setup-progress" hidden aria-label="${text("progress")}"></progress><div id="setup-results" class="setup-results"></div><button type="button" data-action="register" hidden>${text("register")}</button><p id="setup-status" role="status" aria-live="polite"></p>` :
                `<p>${text("alertInfo")}</p><a href="/settings" id="setup-settings">${text("settings")}</a>`) +
            `<div class="setup-actions"><button type="button" data-action="skip">${text("skip")}</button>${step > 0 || (step === 0 && languageChoiceEnabled) ? `<button type="button" data-action="${step === 0 ? "language-back" : "back"}">${text("back")}</button>` : ""}<button type="button" class="setup-primary" data-action="next">${text(step === 2 ? "finish" : step === 1 && !registered ? "continue" : "next")}</button></div>`;
        dialog.querySelector<HTMLElement>("#setup-title")?.focus();
        dialog.querySelector("#setup-settings")?.addEventListener("click", close);
        dialog.querySelector("form")?.addEventListener("submit", (event) => { event.preventDefault(); void scan(); });
    };
    const scan = async (): Promise<void> => {
        if (busy) return;
        const cidr = dialog.querySelector<HTMLInputElement>("#setup-cidr")!.value.trim();
        const community = dialog.querySelector<HTMLInputElement>("#setup-community")!.value;
        const match = cidr.match(/^(\d{1,3}(?:\.\d{1,3}){3})\/(\d{1,2})$/);
        if (!match || match[1].split(".").some((part) => Number(part) > 255) || Number(match[2]) < 24 || Number(match[2]) > 32) { status(text("rangeError"), true); return; }
        stop();
        devices = [];
        registered = false;
        dialog.querySelector("#setup-results")!.replaceChildren();
        dialog.querySelector<HTMLButtonElement>("[data-action=register]")!.hidden = true;
        const signal = controller.signal;
        setBusy(true);
        status(text("scanning"));
        const progress = dialog.querySelector<HTMLProgressElement>("#setup-progress")!;
        progress.hidden = false;
        progress.removeAttribute("value");
        const fail = (error: unknown): void => {
            if (signal.aborted) return;
            stop(); setBusy(false); progress.hidden = true;
            status(error instanceof Error ? error.message : text("failure"), true);
        };
        deadline = window.setTimeout(() => fail(new Error(text("timeout"))), 300_000);
        try {
            const prefix = Number(match[2]);
            const hosts = prefix >= 31 ? 2 ** (32 - prefix) : 2 ** (32 - prefix) - 2;
            const job = await request<{ job_id: string }>("/api/discovery/scan", signal, new URLSearchParams({ cidr, community, max_hosts: String(hosts) }));
            if (signal.aborted) return;
            if (!job.job_id) throw new Error(text("failure"));
            const poll = async (): Promise<void> => {
                try {
                    const result = await request<ScanResult>(`/api/discovery/scan/${encodeURIComponent(job.job_id)}`, signal);
                    if (signal.aborted) return;
                    progress.max = result.total || hosts;
                    progress.value = result.scanned || 0;
                    status(`${text("scanning")} ${result.scanned || 0} / ${result.total || hosts}`);
                    if (result.status === "error") throw new Error(text("failure"));
                    if (result.status !== "done") { timer = window.setTimeout(() => void poll(), 1500); return; }
                    const existing = await request<FoundDevice[]>("/api/devices", signal);
                    if (signal.aborted) return;
                    window.clearTimeout(deadline);
                    const known = new Set(existing.map((device) => device.ip));
                    devices = (result.devices || []).filter((device) => !known.has(device.ip)).map((device) => ({ ...device, community: device.community || community }));
                    setBusy(false);
                    progress.hidden = true;
                    status(devices.length ? text("selected") : text("empty"));
                    dialog.querySelector("#setup-results")!.innerHTML = devices.map((device, index) => `<label class="setup-device"><input type="checkbox" value="${index}" checked><span>${esc(device.ip)} ${esc(device.name || "")}</span></label>`).join("");
                    dialog.querySelector<HTMLButtonElement>("[data-action=register]")!.hidden = devices.length === 0;
                } catch (error) { fail(error); }
            };
            await poll();
        } catch (error) { fail(error); }
    };
    const register = async (): Promise<void> => {
        if (busy) return;
        const selected = Array.from(dialog.querySelectorAll<HTMLInputElement>("#setup-results input:checked")).map((input) => devices[Number(input.value)]);
        if (!selected.length) { status(text("selected"), true); return; }
        const signal = controller.signal;
        setBusy(true);
        try {
            const result = await request<{ registered: string[]; skipped: string[]; errors: string[] }>("/api/discovery/register", signal, selected);
            if (signal.aborted) return;
            const accepted = new Set([...result.registered, ...result.skipped]);
            dialog.querySelectorAll<HTMLInputElement>("#setup-results input").forEach((input) => { if (accepted.has(devices[Number(input.value)].ip)) input.closest("label")?.remove(); });
            registered = result.registered.length > 0 || result.skipped.length > 0;
            status(result.errors.length ? result.errors.join("\n") : registered ? text("registered") : text("failure"), result.errors.length > 0 || !registered);
            window.refreshDashboard?.();
            dialog.querySelector<HTMLButtonElement>("[data-action=next]")!.textContent = text(registered ? "next" : "continue");
            dialog.querySelector<HTMLButtonElement>("[data-action=register]")!.hidden = dialog.querySelectorAll("#setup-results input").length === 0;
        } catch (error) { if (!signal.aborted) status(error instanceof Error ? error.message : text("failure"), true); }
        finally { if (!signal.aborted) setBusy(false); }
    };
    dialog.addEventListener("click", (event) => {
        const action = (event.target as HTMLElement).closest<HTMLButtonElement>("[data-action]")?.dataset.action;
        if (action === "skip") close();
        else if (action === "language-back" && languageChoiceEnabled) { choosingLanguage = true; render(); }
        else if (!busy && action === "register") void register();
        else if (!busy && (action === "back" || action === "next")) {
            if (action === "next" && step === 2) { close(); return; }
            stop(); devices = []; step += action === "back" ? -1 : 1; render();
        }
    });
    dialog.addEventListener("cancel", (event) => { event.preventDefault(); close(); });
    const open = (selectLanguage = false): void => {
        if (dialog.open) return;
        previousFocus = document.activeElement as HTMLElement | null;
        stop(); step = 0; registered = false;
        languageChoiceEnabled = selectLanguage;
        choosingLanguage = selectLanguage;
        dialog.showModal(); render();
    };
    const launch = document.createElement("button");
    launch.className = "setup-launch";
    launch.type = "button";
    launch.textContent = text("guide");
    launch.hidden = auto && dismissed;
    launch.addEventListener("click", () => open());
    document.querySelector("main h1")?.after(launch);
    window.addEventListener("tracepulse-language-change", () => { launch.textContent = text("guide"); if (dialog.open && step !== 1) render(); });
    if (auto) {
        if (!dismissed) void request<FoundDevice[]>("/api/devices", controller.signal).then((existing) => { if (existing.length === 0) open(true); }).catch(() => { /* keep normal dashboard available */ });
    }
}