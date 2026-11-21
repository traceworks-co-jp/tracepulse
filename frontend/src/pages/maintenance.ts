interface MuteState {
    enabled: boolean;
    muted_until: number | null;
    reason: string;
}
interface MaintenanceStatus {
    server_time: number;
    global: MuteState | null;
    devices: Record<string, MuteState>;
}
const words = {
    en: { title: "Alert maintenance", target: "Scope", global: "All devices", reason: "Reason",
        duration: "Duration", reasonPlaceholder: "e.g., Planned switch firmware upgrade", muted: "Muted", until: "Until",
        hour: "1 hour", day: "24 hours", indefinite: "Until manually unmuted", mute: "Mute",
        unmute: "Unmute", active: "Muted: collection and recording continue.", inactive: "Not muted",
        help: "Mute and Unmute apply immediately; you do not need to click Save Settings. After unmuting, continuing risks are summarized following a fresh observation." },
    ja: { title: "アラート静観", target: "対象", global: "全機器", reason: "理由",
        duration: "期間", reasonPlaceholder: "例：スイッチのファームウェア更新", muted: "静観中", until: "解除予定",
        hour: "1時間", day: "24時間", indefinite: "手動解除まで", mute: "静観する",
        unmute: "解除する", active: "静観中：収集・記録は継続します。", inactive: "静観なし",
        help: "静観と解除は即時に適用されるため、「設定を保存」は不要です。解除後の新しい観測で継続中のリスクをまとめて通知します。" },
};
const root = document.getElementById("core-maintenance");
if (root) {
    let status: MaintenanceStatus | undefined;
    let devices: { id: number; name: string; ip: string }[] = [];
    let selected = "";
    let busy = false;
    const text = <K extends keyof typeof words.en>(key: K): string =>
        words[document.documentElement.lang === "ja" ? "ja" : "en"][key];
    const element = <K extends keyof HTMLElementTagNameMap>(tag: K, value = ""): HTMLElementTagNameMap[K] => {
        const node = document.createElement(tag); node.textContent = value; return node;
    };
    const error = element("p"); error.setAttribute("role", "alert");
    const request = async (method: string, body?: object): Promise<MaintenanceStatus> => {
        const response = await fetch("/api/maintenance", {
            method, headers: { "Content-Type": "application/json", ...window.tracepulseRequestHeaders?.() },
            body: body === undefined ? undefined : JSON.stringify(body),
        });
        const value = await response.json() as MaintenanceStatus & { error?: string };
        if (!response.ok || value.error) throw new Error(value.error || `HTTP ${response.status}`);
        return value;
    };
    const render = (): void => {
        root.replaceChildren();
        const own = selected ? status?.devices[selected] : status?.global;
        const active = !!own?.enabled && (own.muted_until === null || own.muted_until > (status?.server_time || 0));
        const header = element("div");
        header.className = "maintenance-header";
        header.append(element("h2", text("title")));
        const state = element("div");
        state.className = "maintenance-state";
        const badge = element("span", text(active ? "muted" : "inactive"));
        badge.className = `maintenance-status-badge ${active ? "is-muted" : "is-clear"}`;
        badge.setAttribute("role", "status");
        state.append(badge);
        if (active && own) {
            const details = element("div");
            details.className = "maintenance-status-details";
            details.append(element("span", text("active")));
            const until = own.muted_until === null
                ? text("indefinite")
                : new Intl.DateTimeFormat(document.documentElement.lang === "ja" ? "ja-JP" : "en-US",
                    { dateStyle: "medium", timeStyle: "short" }).format(new Date(own.muted_until * 1000));
            details.append(element("span", `${text("until")}: ${until}`));
            if (own.reason) details.append(element("span", `${text("reason")}: ${own.reason}`));
            state.append(details);
        }
        header.append(state);
        root.append(header, element("p", text("help")));

        const form = element("div");
        form.className = "maintenance-form-grid";
        const scopeField = element("div");
        scopeField.className = "maintenance-field";
        const scopeLabel = element("label", text("target"));
        const target = element("select");
        target.id = "maintenance-scope";
        scopeLabel.htmlFor = target.id;
        const all = element("option", text("global")); all.value = ""; target.append(all);
        for (const device of devices) {
            const option = element("option", `${device.name} (${device.ip})`);
            option.value = String(device.id); target.append(option);
        }
        target.value = selected;
        target.onchange = () => { selected = target.value; render(); };
        scopeField.append(scopeLabel, target);

        const durationField = element("div");
        durationField.className = "maintenance-field";
        const durationLabel = element("label", text("duration"));
        const duration = element("select");
        duration.id = "maintenance-duration";
        durationLabel.htmlFor = duration.id;
        for (const [value, key] of [["3600", "hour"], ["86400", "day"], ["", "indefinite"]] as const) {
            const option = element("option", text(key)); option.value = value; duration.append(option);
        }
        durationField.append(durationLabel, duration);

        const reasonField = element("div");
        reasonField.className = "maintenance-field maintenance-field-full";
        const reasonLabel = element("label", text("reason"));
        const reason = element("input");
        reason.id = "maintenance-reason";
        reasonLabel.htmlFor = reason.id;
        reason.maxLength = 500;
        reason.value = own?.reason || "";
        reason.placeholder = text("reasonPlaceholder");
        reasonField.append(reasonLabel, reason);
        form.append(scopeField, durationField, reasonField);
        root.append(form);

        const actions = element("div");
        actions.className = "maintenance-actions";
        for (const enabled of [true, false]) {
            const button = element("button", text(enabled ? "mute" : "unmute"));
            button.type = "button";
            button.className = enabled ? "btn btn-primary" : "btn btn-secondary";
            button.disabled = busy || !status || (!enabled && !active);
            button.onclick = async () => {
                const submittedDuration = duration.value;
                const submittedReason = reason.value;
                busy = true; error.textContent = ""; render();
                try {
                    status = await request("POST", { device_id: selected ? Number(selected) : null,
                        enabled, duration: enabled && submittedDuration ? Number(submittedDuration) : null,
                        reason: submittedReason });
                } catch (failure) { error.textContent = String(failure); }
                finally { busy = false; render(); }
            };
            actions.append(button);
        }
        root.append(actions, error);
    };
    const load = async (): Promise<void> => {
        try {
            const [snapshot, response] = await Promise.all([request("GET"), fetch("/api/devices")]);
            if (!response.ok) throw new Error(`HTTP ${response.status}`);
            devices = await response.json() as typeof devices;
            status = snapshot;
        } catch (failure) { error.textContent = String(failure); }
        render();
    };
    window.addEventListener("tracepulse-language-change", render);
    void load();
}
export {};
