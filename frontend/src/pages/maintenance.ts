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
        hour: "1 hour", day: "24 hours", indefinite: "Until manually unmuted", mute: "Mute",
        unmute: "Unmute", active: "Muted: collection and recording continue.", inactive: "Not muted",
        help: "After unmuting, continuing risks are summarized following a fresh observation." },
    ja: { title: "アラート静観", target: "対象", global: "全機器", reason: "理由",
        hour: "1時間", day: "24時間", indefinite: "手動解除まで", mute: "静観する",
        unmute: "解除する", active: "静観中：収集・記録は継続します。", inactive: "静観なし",
        help: "解除後の新しい観測で継続中のリスクをまとめて通知します。" },
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
        root.replaceChildren(element("h2", text("title")), element("p", text("help")));
        const label = element("label", text("target")), target = element("select");
        const all = element("option", text("global")); all.value = ""; target.append(all);
        for (const device of devices) {
            const option = element("option", `${device.name} (${device.ip})`);
            option.value = String(device.id); target.append(option);
        }
        target.value = selected;
        target.onchange = () => { selected = target.value; render(); };
        label.append(target); root.append(label);
        const own = selected ? status?.devices[selected] : status?.global;
        const active = !!own?.enabled && (own.muted_until === null || own.muted_until > (status?.server_time || 0));
        root.append(element("p", text(active ? "active" : "inactive")));
        const reasonLabel = element("label", text("reason")), reason = element("input");
        reason.maxLength = 500; reason.value = own?.reason || ""; reasonLabel.append(reason); root.append(reasonLabel);
        const duration = element("select");
        for (const [value, key] of [["3600", "hour"], ["86400", "day"], ["", "indefinite"]] as const) {
            const option = element("option", text(key)); option.value = value; duration.append(option);
        }
        root.append(duration);
        for (const enabled of [true, false]) {
            const button = element("button", text(enabled ? "mute" : "unmute"));
            button.type = "button"; button.className = "btn";
            button.disabled = busy || !status || (!enabled && !active);
            button.onclick = async () => {
                busy = true; error.textContent = ""; render();
                try {
                    status = await request("POST", { device_id: selected ? Number(selected) : null,
                        enabled, duration: enabled && duration.value ? Number(duration.value) : null, reason: reason.value });
                } catch (failure) { error.textContent = String(failure); }
                finally { busy = false; render(); }
            };
            root.append(button);
        }
        root.append(error);
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
