import { showOperationProgress } from "./operation-progress";

type Json = string | number | boolean | Json[] | { [key: string]: Json };
type Document = { [key: string]: Json };
type Category = "system" | "interfaces" | "hardware" | "profile";
interface DefinitionRow { label: string; path: string; value: Json; commit: (value: Json) => void; remove?: () => void; configured?: boolean; highlightPath?: string; }
interface Common { cpu_oid: string; memory_oid: string; hardware_oids: string[]; }
interface Definitions { common: Common; profiles: Record<string, Document>; }
interface Profile { id: string; source: string; document: Document; base: Document | null; resettable: boolean; }
interface View { generation: number; applied_generation: number; definitions: Definitions; profiles: Profile[]; }
interface RegisteredDevice { ip: string; name: string; sys_object_id: string | null; }

const messages = {
    en: {
        title: "OID additions / SNMP monitoring definitions",
        help: "Templates define device-specific monitoring. Common OIDs are additional candidates for all devices: CPU/memory use the template first, then the common OID, then standard fallback. CPU/memory common values must be percentages (0–100). Hardware entries use type|numeric OID.",
        storage: "GUI changes are stored in SQLite, not written to bundled or BYO TOML files. Editing a profile creates a complete user-owned copy; Reset restores its current file-backed definition. Saves apply to new polling and diagnostic operations without restarting.",
        profile: "Profile", common: "Common additional OIDs (all devices)", save: "Save and apply", reload: "Reload saved definitions",
        reset: "Reset / remove user profile", add: "Add", remove: "Remove", import: "Import TOML", export: "Export TOML",
        newProfile: "New profile sysObjectID prefix", create: "Create profile", saved: "Saved and applied",
        savedToast: "Saved and applied (Generation {generation})", closeToast: "Dismiss notification",
        dirty: "Unsaved changes", source: "Definition source", version: "Saved / applied generation", property: "Optional section",
        preview: "Preview profile selection", sysObject: "sysObjectID", noSelector: "Enter a numeric sysObjectID prefix.",
        discard: "Discard all unsaved changes?", resetConfirm: "Reset this profile? A file-backed profile will be restored; a GUI-only profile will be removed. Save to apply.",
        disclaimer: "When configuring proprietary vendor OIDs or importing third-party templates, ensure compliance with your hardware vendor's EULA and support agreements. TraceWorks assumes no liability for unauthorized access to proprietary vendor metrics.",
        systemTab: "CPU / memory", interfacesTab: "Interfaces / optical power", hardwareTab: "Hardware sensors", profileTab: "Profile information",
        item: "Monitoring item", oids: "Configured OIDs", configuration: "Configuration", configured: "Configured", unconfigured: "Not configured",
        edit: "Edit", addItem: "+ Add item", itemType: "Item type", advanced: "Advanced settings", applyItem: "Apply to draft", cancel: "Cancel",
        empty: "No items configured", cpuCandidate: "CPU utilization OID", memorySource: "Memory source", customMetric: "Additional metric", domMetric: "Optical power metric",
        interfaceSettings: "Interface settings", cpuSettings: "CPU settings", domSettings: "Optical table settings", newProfileTitle: "Create profile",
        notes: "Definition storage and priority", invalidNumber: "Enter a valid number.", confirmRemove: "Remove this item from the draft?",
        loading: "Loading monitoring definitions", saving: "Saving and applying monitoring definitions", importing: "Importing TOML", exporting: "Exporting TOML", previewing: "Previewing profile selection",
        profileGuide: "Create monitoring rules that apply automatically to a specific vendor or model. The profile with the longest matching sysObjectID prefix is selected.",
        profileName: "Profile name", profileNameExample: "Cisco Catalyst 9200 / Yamaha RTX1220",
        registeredDevice: "Select a registered device", chooseDevice: "Select a device to use its sysObjectID",
        unknownOid: "sysObjectID not collected yet", probeIp: "Device IP address", community: "Community (optional; blank uses device or system credentials)",
        probe: "Fetch", probing: "Fetching sysObjectID…", vendorScope: "Apply to the entire vendor", modelScope: "Apply to this model (full OID)",
        scopePreview: "Automatically applies to devices whose sysObjectID starts with", longestHint: "A more specific matching profile takes priority.",
        baseTemplate: "Initial settings (base template)", emptyTemplate: "Start empty", duplicateProfile: "A profile with this prefix already exists. Select it from the profile list.",
        deviceProfiles: "Device profiles (automatic sysObjectID selection)", commonProfiles: "Shared settings for all devices",
        condition: "Applies when sysObjectID starts with", fallbackCondition: "Fallback when no device-specific profile matches",
        matchingDevices: "Matching registered devices", commonCondition: "All registered devices (additional OID candidates)",
        scalarHint: "Single-value (scalar) OIDs must end in .0.",
        scaleHint: "Use 0.1 to convert integers in tenths (for example, 455 = 45.5°C).",
    },
    ja: {
        title: "OID追加・SNMP監視定義",
        help: "テンプレートで機器別の監視を定義します。共通OIDは全デバイスへの追加候補です。CPU・メモリはテンプレート→共通OID→標準フォールバックの順で採用します。共通CPU・メモリは割合値（0〜100）専用です。ハードウェア項目は 種別|数値OID の形式です。",
        storage: "GUIの変更はSQLiteに保存し、同梱・BYOのTOMLは書き換えません。編集したプロファイルは全項目を含むユーザー定義となり、リセットで現在のファイル定義に戻ります。保存後、新しいポーリング・診断から再起動なしで反映します。",
        profile: "プロファイル", common: "共通追加OID（全デバイス）", save: "保存して適用", reload: "保存済み定義を再読込",
        reset: "リセット・ユーザープロファイル削除", add: "追加", remove: "削除", import: "TOMLをインポート", export: "TOMLをエクスポート",
        newProfile: "新規プロファイルのsysObjectIDプレフィックス", create: "プロファイル作成", saved: "保存・適用しました",
        savedToast: "保存・適用しました（世代 {generation}）— 次回のポーリング・診断から反映されます", closeToast: "通知を閉じる",
        dirty: "未保存の変更があります", source: "定義元", version: "保存済み／適用済み世代", property: "追加可能なセクション",
        preview: "プロファイル選択をプレビュー", sysObject: "sysObjectID", noSelector: "数値のsysObjectIDプレフィックスを入力してください。",
        discard: "未保存の変更をすべて破棄しますか？", resetConfirm: "リセットしますか？ファイル由来の定義は元に戻し、GUIのみの定義は削除します。保存するまで監視には反映されません。",
        disclaimer: "ベンダー独自OIDや第三者テンプレートを利用する際は、機器ベンダーのEULAとサポート契約への準拠を確認してください。TraceWorksはベンダー独自メトリクスへの無許可アクセスについて責任を負いません。",
        systemTab: "CPU・メモリ", interfacesTab: "インターフェース・光レベル", hardwareTab: "ハードウェアセンサー", profileTab: "プロファイル情報",
        item: "監視項目", oids: "設定OID", configuration: "設定状態", configured: "設定済み", unconfigured: "未設定",
        edit: "編集", addItem: "＋項目追加", itemType: "項目の種類", advanced: "詳細設定", applyItem: "編集内容を反映", cancel: "キャンセル",
        empty: "監視項目は登録されていません", cpuCandidate: "CPU使用率OID", memorySource: "メモリ取得元", customMetric: "追加監視項目", domMetric: "光レベル監視項目",
        interfaceSettings: "インターフェースの詳細設定", cpuSettings: "CPUの詳細設定", domSettings: "光レベルテーブルの詳細設定", newProfileTitle: "プロファイル作成",
        notes: "定義の保存先・適用順", invalidNumber: "有効な数値を入力してください。", confirmRemove: "この項目を下書きから削除しますか？",
        loading: "監視定義を読み込んでいます", saving: "監視定義を保存・適用しています", importing: "TOMLをインポートしています", exporting: "TOMLをエクスポートしています", previewing: "プロファイル選択をプレビューしています",
        profileGuide: "特定のベンダーや機種に自動適用する監視ルールを作成します。機器の sysObjectID の先頭一致（最長一致）で自動的に選ばれます。",
        profileName: "プロファイル名", profileNameExample: "Cisco Catalyst 9200 / Yamaha RTX1220",
        registeredDevice: "登録済みデバイスから選択", chooseDevice: "sysObjectIDを取得するデバイスを選択",
        unknownOid: "sysObjectID未取得", probeIp: "機器のIPアドレス", community: "コミュニティ（任意・空欄は個別／システム共通設定を使用）",
        probe: "取得", probing: "sysObjectIDを取得しています…", vendorScope: "ベンダー全体に適用", modelScope: "この機種専用に適用（フルOID）",
        scopePreview: "で始まるすべての機器に自動適用されます", longestHint: "より具体的な条件のプロファイルがある場合は、そちらが優先されます。",
        baseTemplate: "初期設定（ベーステンプレート）", emptyTemplate: "空から作成", duplicateProfile: "このプレフィックスのプロファイルは既にあります。一覧から選択してください。",
        deviceProfiles: "機器別プロファイル（sysObjectID 自動選択）", commonProfiles: "全デバイス共通設定",
        condition: "適用条件（sysObjectIDプレフィックス）", fallbackCondition: "機器別プロファイルに一致しない機器に適用",
        matchingDevices: "一致する登録機器", commonCondition: "すべての登録機器（追加OID候補）",
        scalarHint: "※単一値（スカラー）のOIDは末尾に .0 を付けてください",
        scaleHint: "※0.1単位の整数（例: 455 = 45.5℃）を換算する場合は 0.1 を指定",
    },
};
type Key = keyof typeof messages.en;
const text = (key: Key): string => messages[localStorage.getItem("tracepulse-lang") === "ja" ? "ja" : "en"][key];
const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;

function validOid(value: string): boolean {
    const parts = value.replace(/^\./, "").split(".");
    return parts.length >= 2 && parts.length <= 128
        && parts.every(part => /^\d+$/.test(part) && Number(part) <= 4294967295)
        && Number(parts[0]) <= 2 && (Number(parts[0]) === 2 || Number(parts[1]) <= 39);
}

function selector(doc: Document): string {
    return typeof doc.sysObjectID_prefix === "string" ? doc.sysObjectID_prefix.replace(/^\.+|\.+$/g, "")
        : Number(doc.enterprise_id) ? `1.3.6.1.4.1.${doc.enterprise_id}` : "";
}

function selectedProfile(oid: string | null, profiles: Map<string, Profile>): string {
    let selected = "generic", segments = 0;
    if (!oid || !validOid(oid)) return selected;
    oid = oid.replace(/^\.+/, "");
    for (const profile of profiles.values()) {
        const prefix = selector(profile.document), length = prefix.split(".").length;
        if (prefix && (oid === prefix || oid.startsWith(`${prefix}.`)) && length >= segments) {
            selected = profile.id; segments = length;
        }
    }
    return selected;
}

async function registeredDevices(): Promise<RegisteredDevice[]> {
    const response = await fetch("/api/devices");
    const result: RegisteredDevice[] | { error?: string } = await response.json();
    if (!response.ok || !Array.isArray(result)) {
        throw new Error(!Array.isArray(result) && result.error || `HTTP ${response.status}`);
    }
    return result;
}

const fieldLabels: Record<string, [string, string]> = {
    profile: ["Monitoring profile", "監視プロファイル"], common: ["Common additional OIDs", "共通追加OID"],
    name: ["Profile name", "プロファイル名"], description: ["Description", "説明"],
    sysObjectID_prefix: ["Device identification (sysObjectID prefix)", "機器識別条件（sysObjectIDプレフィックス）"],
    enterprise_id: ["Vendor enterprise number", "ベンダー企業番号"],
    cpu: ["CPU monitoring", "CPU監視"], candidates: ["CPU utilization OIDs (priority order)", "CPU使用率OID（優先順）"],
    cpu_oid: ["Additional CPU utilization OID", "追加CPU使用率OID"],
    memory_oid: ["Additional memory utilization OID", "追加メモリ使用率OID"],
    hardware_oids: ["Additional hardware sensor OIDs", "追加ハードウェアセンサーOID"],
    memory: ["Memory monitoring", "メモリ監視"], mode: ["Memory calculation method", "メモリ使用率の計算方法"],
    aggregation: ["Table value selection", "テーブル値の採用方法"],
    used_prefix: ["Used memory OID", "使用メモリOID"], free_prefix: ["Free memory OID", "空きメモリOID"],
    total_prefix: ["Total memory OID", "総メモリOID"], used_is_bytes: ["Used memory is in bytes", "使用メモリの単位はバイト"],
    fallbacks: ["Alternative memory sources (priority order)", "代替メモリ取得元（優先順）"],
    tables: ["Interface and optical monitoring", "インターフェース・光レベル監視"],
    interfaces: ["Interface traffic (IF-MIB)", "インターフェーストラフィック（IF-MIB）"],
    base_oid: ["Table base OID", "テーブルの基点OID"],
    index: ["Interface index (ifIndex)", "インターフェース番号（ifIndex）"],
    in_octets: ["Received traffic (ifInOctets)", "受信トラフィック（ifInOctets）"],
    out_octets: ["Transmitted traffic (ifOutOctets)", "送信トラフィック（ifOutOctets）"],
    dom: ["Optical power (DOM / DDM)", "光レベル（DOM / DDM）"],
    index_oid: ["Interface mapping OID (ifIndex)", "インターフェース対応付けOID（ifIndex）"],
    entity: ["ENTITY sensor mapping", "ENTITYセンサー対応付け"],
    metrics: ["Monitoring metrics", "監視項目"], key: ["Metric identifier", "監視項目ID"],
    oid: ["Numeric OID", "数値OID"], unit: ["Unit", "単位"], label: ["Display label", "表示名"],
    data_type: ["Value type", "値の種類"], scale: ["Multiplier", "倍率"], offset: ["Value offset", "値の補正量"],
    invalid_values: ["Unmeasured sentinel values (before scaling)", "未計測として扱う値（倍率適用前）"],
    match_names: ["Sensor name filters", "センサー名の絞り込み"],
    sensors: ["Hardware sensors", "ハードウェアセンサー"], temperature: ["Temperature sensors", "温度センサー"],
    power: ["Power sensors", "電源センサー"], fan: ["Fan sensors", "ファンセンサー"],
    descr_prefix: ["Sensor name OID", "センサー名OID"], value_prefix: ["Sensor reading OID", "センサー測定値OID"],
    state_prefix: ["Sensor status OID", "センサー状態OID"], fallback_builtin_power: ["Synthetic power fallback (unsupported)", "仮想電源値の補完（非対応）"],
    alarm_states: ["Alarm status values", "異常とする状態値"], normal_states: ["Normal status values", "正常とする状態値"],
    state_labels: ["Status display names", "状態値の表示名"],
    name_oid: ["Sensor name OID", "センサー名OID"], contained_in_oid: ["Parent entity OID", "親エンティティOID"],
    alias_oid: ["Interface alias mapping OID", "インターフェース別名対応OID"], type_oid: ["Sensor type OID", "センサー種別OID"],
    scale_oid: ["Sensor scale OID", "センサー倍率OID"], precision_oid: ["Sensor precision OID", "センサー精度OID"],
    status_oid: ["Sensor availability OID", "センサー稼働状態OID"],
    category: ["Metric category", "監視項目の分類"], rule: ["Trend evaluation rule", "傾向評価ルール"],
    instance: ["Scalar or table", "単一値・テーブル"], match_name: ["Sensor name match", "対象センサー名"],
    unit_oid: ["Sensor unit OID", "センサー単位OID"], match_unit: ["Required sensor unit", "対象センサー単位"],
    warning_value: ["Warning threshold", "警告しきい値"], min_slope_per_day: ["Minimum daily change", "1日あたりの最小変化量"],
    forecast_days: ["Forecast horizon (days)", "予測期間（日）"], min_samples: ["Minimum sample count", "最小サンプル数"],
    min_period_days: ["Minimum observation period (days)", "最小観測期間（日）"],
};

function fieldLabel(path: string): string {
    const parts = path.split(".");
    const key = parts[parts.length - 1];
    const language = localStorage.getItem("tracepulse-lang") === "ja" ? 1 : 0;
    if (/^\d+$/.test(key)) {
        const parent = parts[parts.length - 2];
        if (parent === "state_labels") return `${language ? "状態値" : "Status value"} ${key}`;
        const label = parent === "candidates" ? ["CPU utilization OID", "CPU使用率OID"]
            : fieldLabels[parent] || ["Entry", "項目"];
        return `${label[language]} ${Number(key) + 1}`;
    }
    if (path === "profile.tables.interfaces.name") return language ? "インターフェース名（ifDescr）" : "Interface name (ifDescr)";
    if (key === "name" && parts.includes("metrics")) return language ? "監視項目名" : "Metric name";
    if (key === "oid" && parts.includes("memory")) return language ? "メモリ使用率OID" : "Memory utilization OID";
    return fieldLabels[key]?.[language] || (language ? "追加項目" : "Additional field");
}

async function api<T>(path: string, body?: unknown): Promise<T> {
    const response = await fetch(path, body === undefined ? {} : {
        method: "POST", headers: { "Content-Type": "application/json", ...window.tracepulseRequestHeaders?.() },
        body: JSON.stringify(body),
    });
    const result: { ok?: boolean; error?: string; data?: T } = await response.json();
    if (!response.ok || !result.ok || result.data === undefined) throw new Error(result.error || `HTTP ${response.status}`);
    return result.data;
}

function element<K extends keyof HTMLElementTagNameMap>(tag: K, content?: string): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    if (content !== undefined) node.textContent = content;
    return node;
}

function object(value: Json | undefined): Document {
    return value && typeof value === "object" && !Array.isArray(value) ? value : {};
}

function array(value: Json | undefined): Json[] {
    return Array.isArray(value) ? value : [];
}

function configuredOids(value: Json): string[] {
    if (typeof value === "string") return value.split("|").filter(part => /^\.?\d+(\.\d+)+$/.test(part));
    if (typeof value !== "object") return [];
    return [...new Set(Object.values(value).flatMap(configuredOids))];
}

function button(label: string, action: () => void): HTMLButtonElement {
    const node = element("button", label);
    node.type = "button";
    node.addEventListener("click", action);
    return node;
}

function optionalSections(path: string, value: Document): Record<string, Json> {
    if (path === "profile") return {
        cpu: { candidates: [], aggregation: "first" },
        memory: { mode: "direct", oid: "", fallbacks: [] },
        sensors: { temperature: [], power: [], fan: [] },
        metrics: [],
        tables: {},
    };
    if (path.endsWith(".tables")) return {
        interfaces: { base_oid: "", index: "", name: "", in_octets: "", out_octets: "" },
        dom: { base_oid: "", metrics: [] },
    };
    if (path.endsWith(".sensors")) return { temperature: [], power: [], fan: [] };
    if (path.endsWith(".memory")) return { mode: "direct", oid: "", used_prefix: "", free_prefix: "", total_prefix: "", aggregation: "first", used_is_bytes: false, fallbacks: [] };
    if (path.endsWith(".dom")) return { index_oid: "", entity: { name_oid: "", contained_in_oid: "", alias_oid: "", type_oid: "", scale_oid: "", precision_oid: "", status_oid: "" } };
    if (/\.sensors\.(temperature|power|fan)\.\d+$/.test(path)) return { alarm_states: [2, 3, 4, 6] };
    if (/\.metrics\.\d+$/.test(path) && !("key" in value)) return {
        name_oid: "", match_name: "", unit_oid: "", match_unit: "",
        scale_oid: "", precision_oid: "", warning_value: 0,
    };
    return {};
}

function arraySeed(path: string, current: Json[]): Json {
    if (path.endsWith(".metrics")) {
        if (path.includes(".dom.")) return { key: "rx_power", oid: "", unit: "dBm", scale: 0.01, label: "Rx Power", invalid_values: [], match_names: [] };
        return { key: "custom_metric", oid: "", unit: "count", label: "Custom metric", data_type: "gauge" };
    }
    if (path.endsWith(".fallbacks")) return { mode: "direct", oid: "", used_prefix: "", free_prefix: "", total_prefix: "", aggregation: "first", used_is_bytes: false };
    if (/\.(temperature|power|fan)$/.test(path)) return {
        descr_prefix: "", value_prefix: "", state_prefix: "", fallback_builtin_power: false,
        normal_states: [], state_labels: {}, match_names: [],
    };
    if (path.endsWith(".invalid_values") || path.endsWith(".normal_states") || path.endsWith(".alarm_states")) return 0;
    if (current.length && typeof current[0] !== "object") return typeof current[0] === "number" ? 0 : "";
    return "";
}

export function initializeTemplateManager(): void {
    const root = document.getElementById("snmp-definitions");
    if (!root) return;
    const style = element("style");
    style.textContent = `
        #snmp-definitions .template-controls{display:flex;gap:8px;flex-wrap:wrap;align-items:center;margin:12px 0}
        #snmp-definitions .template-source{display:grid;grid-template-columns:max-content minmax(0,1fr);gap:4px 12px;margin:12px 0;padding:10px 12px;border-left:3px solid var(--tp-accent,#38c5b2);background:var(--tp-bg,#0b1524);font-size:.88rem}
        #snmp-definitions .template-source dt{color:var(--tp-muted,#94a3b8)}
        #snmp-definitions .template-source dd{min-width:0;margin:0;font-weight:600;color:var(--tp-text,#eef5fb);overflow-wrap:anywhere}
        html[data-theme=light] #snmp-definitions .template-source{background:#f1f5f9}
        html[data-theme=light] #snmp-definitions .template-source dt{color:#64748b}
        html[data-theme=light] #snmp-definitions .template-source dd{color:#0f172a}
        #snmp-definitions fieldset{min-width:0;margin:10px 0;padding:12px;border:1px solid var(--tp-border,#475569);border-radius:4px}
        #snmp-definitions legend{padding:0 5px;overflow-wrap:anywhere}
        #snmp-definitions label{display:grid;gap:5px;margin:8px 0;overflow-wrap:anywhere}
        #snmp-definitions input,#snmp-definitions select{max-width:100%;min-width:0;padding:8px;background:var(--tp-surface,#152337);color:var(--tp-text,#eef5fb);border:1px solid #64748b;border-radius:4px}
        #snmp-definitions button{max-width:100%;overflow-wrap:anywhere;padding:7px 10px;background:var(--tp-raised,#1a2b40);color:var(--tp-text,#eef5fb);border:1px solid #64748b;border-radius:4px;cursor:pointer}
        #snmp-definitions button:disabled{opacity:.5;cursor:default}
        #snmp-definitions .template-scalar{display:flex;gap:8px;align-items:end}#snmp-definitions .template-scalar label{flex:1}
        #snmp-definitions [role=alert]{color:#f87171}html[data-theme=light] #snmp-definitions [role=alert]{color:#b91c1c}
        #snmp-definitions p{margin:10px 0;overflow-wrap:anywhere}
        #snmp-definitions .template-toolbar{position:sticky;top:0;z-index:5;display:flex;flex-wrap:wrap;gap:8px;align-items:flex-end;padding:12px 0;background:var(--tp-surface,#172638);border-bottom:1px solid var(--tp-border,#334155)}
        #snmp-definitions .template-toolbar>label{flex:1 1 260px;min-width:0;margin:0}
        #snmp-definitions .template-toolbar select,#snmp-definitions .template-toolbar button{box-sizing:border-box;height:40px;font-size:.9rem;line-height:1.25}
        @media(max-width:600px){#snmp-definitions .template-toolbar>label{flex-basis:100%}#snmp-definitions .template-toolbar button{height:auto;min-height:40px}}
        #snmp-definitions .template-toolbar .template-save{background:var(--tp-accent,#38c5b2);color:#082524;border-color:transparent;font-weight:600}
        #snmp-definitions .template-toolbar[data-state=dirty] .template-save{outline:2px solid #f59e0b;outline-offset:2px;box-shadow:0 0 12px rgba(245,158,11,.5)}
        #snmp-definitions .template-status-bar{display:flex;align-items:center;gap:8px;flex-wrap:wrap;margin:10px 0}
        #snmp-definitions .template-status-bar p{margin:0}
        #snmp-definitions [data-template-status]:empty{display:none}
        #snmp-definitions [data-template-status][data-state]{display:inline-flex;align-items:center;gap:.45rem;max-width:100%;box-sizing:border-box;padding:.45rem .7rem;border-radius:6px;font-weight:600}
        #snmp-definitions [data-template-status][data-state=saved]{background:rgba(20,83,45,.55);border:1px solid #22c55e;color:#86efac;animation:template-status-appear .3s ease-out}
        #snmp-definitions [data-template-status][data-state=saved]::before{content:"✓";font-weight:700}
        #snmp-definitions [data-template-status][data-state=dirty]{background:rgba(120,53,15,.45);border:1px solid #f59e0b;color:#fde68a}
        #snmp-definitions [data-template-status][data-state=dirty]::before{content:"●";font-size:.75em}
        html[data-theme=light] #snmp-definitions [data-template-status][data-state=saved]{background:#dcfce7;border-color:#16a34a;color:#166534}
        html[data-theme=light] #snmp-definitions [data-template-status][data-state=dirty]{background:#fef3c7;border-color:#d97706;color:#92400e}
        #snmp-definitions .template-generation{display:inline-flex;align-items:center;max-width:100%;box-sizing:border-box;padding:.4rem .7rem;border:1px solid var(--tp-border,#475569);border-radius:999px;background:var(--tp-bg,#0b1524);color:var(--tp-text,#eef5fb);font-size:.85rem}
        html[data-theme=light] #snmp-definitions .template-generation{background:#f1f5f9;color:#0f172a}
        #snmp-definitions .template-generation.template-generation-flash{animation:template-generation-flash 1.5s ease-out}
        #snmp-definitions .template-items tr.template-row-highlight td{animation:template-row-highlight 2.5s ease-out}
        .template-saved-toast{position:fixed;z-index:1200;top:1rem;right:1rem;display:flex;align-items:flex-start;gap:.6rem;box-sizing:border-box;width:max-content;max-width:calc(100vw - 2rem);padding:.75rem 1rem;border:1px solid #22c55e;border-radius:8px;background:#14532d;color:#dcfce7;box-shadow:0 8px 24px #0006;transition:opacity .25s ease,transform .25s ease;animation:template-status-appear .3s ease-out}
        .template-saved-toast::before{content:"✓";font-weight:700}
        .template-saved-toast span{overflow-wrap:anywhere}
        .template-saved-toast button{flex:none;padding:0 .2rem;border:0;background:transparent;color:inherit;font-size:1.2rem;line-height:1;cursor:pointer}
        .template-saved-toast.is-closing{opacity:0;transform:translateY(-8px);pointer-events:none}
        html[data-theme=light] .template-saved-toast{background:#dcfce7;border-color:#16a34a;color:#166534;box-shadow:0 8px 24px #0003}
        @keyframes template-status-appear{from{opacity:0;transform:translateY(-4px)}to{opacity:1;transform:translateY(0)}}
        @keyframes template-generation-flash{0%,30%{background:#166534;color:#dcfce7;box-shadow:0 0 12px #22c55e}100%{box-shadow:0 0 0 transparent}}
        @keyframes template-row-highlight{0%,20%{background:rgba(34,197,94,.35)}100%{background:transparent}}
        @media(prefers-reduced-motion:reduce){#snmp-definitions [data-template-status][data-state=saved],#snmp-definitions .template-generation.template-generation-flash,#snmp-definitions .template-items tr.template-row-highlight td,.template-saved-toast{animation:none}.template-saved-toast{transition:none}}
        #snmp-definitions .template-tabs{display:flex;gap:4px;overflow:auto;border-bottom:1px solid var(--tp-border,#334155);margin:14px 0}
        #snmp-definitions .template-tabs button{flex:0 0 auto;border:0;border-bottom:3px solid transparent;border-radius:0;background:transparent;padding:10px 12px}
        #snmp-definitions .template-tabs [aria-selected=true]{border-bottom-color:var(--tp-accent,#38c5b2);color:var(--tp-accent,#38c5b2);font-weight:600}
        #snmp-definitions .template-list-actions{display:flex;flex-wrap:wrap;justify-content:flex-end;gap:8px;margin:12px 0}
        #snmp-definitions .template-table-scroll{overflow:auto;max-width:100%}
        #snmp-definitions .template-items{table-layout:fixed;min-width:760px;margin:0}
        #snmp-definitions .template-items th:nth-child(1){width:22%}#snmp-definitions .template-items th:nth-child(2){width:42%}
        #snmp-definitions .template-items th:nth-child(3){width:18%}#snmp-definitions .template-items th:nth-child(4){width:18%}
        #snmp-definitions .template-items th:nth-child(3),#snmp-definitions .template-items td:nth-child(3){white-space:nowrap}
        #snmp-definitions .template-items th,#snmp-definitions .template-items td{vertical-align:top;overflow-wrap:anywhere}
        #snmp-definitions .template-oid{display:block;font-family:monospace;font-size:.82rem;margin-bottom:4px;overflow-wrap:anywhere}
        #snmp-definitions .template-row-actions{display:flex;gap:6px;flex-wrap:wrap}
        .template-dialog{box-sizing:border-box;width:min(760px,calc(100% - 24px));max-height:calc(100dvh - 32px);overflow:auto;margin:auto;padding:20px;border:1px solid var(--tp-border,#475569);border-radius:8px;background:var(--tp-surface,#172638);color:var(--tp-text,#eef5fb)}
        .template-dialog::backdrop{background:rgb(0 0 0 / .6)}
        .template-dialog h3{margin:0 0 14px;font-size:1.05rem;overflow-wrap:anywhere}
        .template-dialog fieldset{min-width:0;border:0;border-top:1px solid var(--tp-border,#475569);padding:12px 0;margin:12px 0}
        .template-dialog legend{padding-right:8px;overflow-wrap:anywhere}
        .template-dialog label{display:grid;gap:6px;margin:10px 0;min-width:0;overflow-wrap:anywhere}
        .template-dialog input,.template-dialog select{box-sizing:border-box;width:100%;min-width:0;padding:8px;background:var(--tp-bg,#0b1524);color:inherit;border:1px solid #64748b;border-radius:4px}
        .template-dialog input[type=checkbox]{width:auto;justify-self:start}
        .template-dialog button{padding:8px 12px;border:1px solid #64748b;border-radius:4px;background:var(--tp-raised,#1a2b40);color:inherit;cursor:pointer}
        .template-dialog .template-scalar{display:flex;gap:8px;align-items:end}.template-dialog .template-scalar label{flex:1}
        .template-dialog .template-dialog-actions{display:flex;gap:8px;justify-content:flex-end;position:sticky;bottom:-20px;padding:12px 0;background:var(--tp-surface,#172638)}
        .template-dialog .template-advanced{border-top:1px solid var(--tp-border,#475569);padding:10px 0;margin-top:12px}
        .template-dialog summary,#snmp-definitions summary{cursor:pointer;padding:8px 0}
        .template-dialog [role=alert]{color:#f87171}
        .template-dialog .template-hint{font-size:.85rem;font-weight:normal;color:var(--tp-muted,#94a3b8)}
        .template-dialog .template-scope-actions{display:flex;flex-wrap:wrap;gap:8px;margin:10px 0}
        .template-dialog button:disabled{opacity:.5;cursor:default}
        html[data-theme=light] .template-dialog,html[data-theme=light] .template-dialog .template-dialog-actions,html[data-theme=light] #snmp-definitions .template-toolbar{background:#fff;color:#0f172a}
        html[data-theme=light] .template-dialog input,html[data-theme=light] .template-dialog select,html[data-theme=light] .template-dialog button{background:#f1f5f9;color:#0f172a}
        html[data-theme=light] .template-dialog [role=alert]{color:#b91c1c}
    `;
    document.head.append(style);
    let view: View | undefined;
    let devices: RegisteredDevice[] = [];
    let definitions: Definitions | undefined;
    let selected = "generic";
    let dirty = false;
    let busy = false;
    let message = "";
    let savedMessage = false;
    let error = "";
    let flashGeneration = false;
    let toast: HTMLElement | undefined;
    let toastTimer: number | undefined;
    let toastRemovalTimer: number | undefined;
    let capture = (): void => { };
    let category: Category = "system";
    let activeDialog: HTMLDialogElement | undefined;
    let modalDirty = false;
    let operation: Key = "loading";
    let operationStarted = 0;
    let stopOperationProgress = (): void => { };
    const showSavedToast = (generation: number): void => {
        if (toastTimer !== undefined) window.clearTimeout(toastTimer);
        if (toastRemovalTimer !== undefined) window.clearTimeout(toastRemovalTimer);
        toast?.remove();
        const node = element("div"); node.className = "template-saved-toast"; node.setAttribute("role", "status");
        const content = element("span", text("savedToast").replace("{generation}", String(generation)));
        const close = button("×", () => dismiss()); close.setAttribute("aria-label", text("closeToast"));
        const dismiss = (): void => {
            if (toastTimer !== undefined) window.clearTimeout(toastTimer);
            node.classList.add("is-closing");
            toastRemovalTimer = window.setTimeout(() => { node.remove(); if (toast === node) toast = undefined; }, 250);
        };
        node.append(content, close); document.body.append(node); toast = node;
        toastTimer = window.setTimeout(dismiss, 3500);
    };
    const run = async (action: () => Promise<void>, kind: Key = "loading"): Promise<void> => {
        if (busy) return;
        operation = kind; operationStarted = performance.now();
        busy = true; error = ""; render();
        if (view) root.querySelector<HTMLElement>(".operation-progress")?.scrollIntoView({ block: "nearest" });
        try { await action(); } catch (reason) { error = String(reason); }
        finally { busy = false; render(); }
    };
    const changed = (): void => {
        capture();
        dirty = true; message = ""; savedMessage = false;
        const status = root.querySelector<HTMLElement>("[data-template-status]");
        if (status) { status.textContent = text("dirty"); status.dataset.state = "dirty"; }
        const toolbar = root.querySelector<HTMLElement>(".template-toolbar");
        if (toolbar) toolbar.dataset.state = "dirty";
    };
    const load = async (): Promise<void> => {
        const [loadedView, loadedDevices] = await Promise.all([api<View>("/api/snmp/definitions"), registeredDevices()]);
        view = loadedView; devices = loadedDevices;
        definitions = clone(view.definitions);
        dirty = false; message = ""; savedMessage = false;
    };
    const editTree = (value: Json, path: string, update: (value: Json) => void, remove?: () => void, notify = changed, refresh = render): HTMLElement => {
        if (typeof value !== "object") {
            const row = element("div"); row.className = "template-scalar";
            const label = element("label", fieldLabel(path));
            const input = element("input");
            input.dataset.templatePath = path;
            input.type = typeof value === "boolean" ? "checkbox" : typeof value === "number" ? "number" : "text";
            if (input.type === "checkbox") input.checked = Boolean(value); else input.value = String(value);
            if (input.type === "number") input.step = "any";
            input.addEventListener("input", () => {
                update(input.type === "checkbox" ? input.checked : input.type === "number" ? input.valueAsNumber : input.value);
                notify();
            });
            label.append(input);
            const field = path.split(".").pop();
            const oidField = field !== "sysObjectID_prefix" && (/(^oid$|_oid$|_prefix$)/.test(field || "")
                || /^profile\.tables\.interfaces\.(index|name|in_octets|out_octets)$/.test(path)
                || /^profile\.cpu\.candidates\.\d+$/.test(path) || /^common\.hardware_oids\.\d+$/.test(path));
            if (field === "scale" || oidField) {
                const hint = element("small", text(field === "scale" ? "scaleHint" : "scalarHint"));
                hint.className = "template-hint"; hint.id = `template-hint-${path.replace(/\./g, "-")}`;
                input.setAttribute("aria-label", fieldLabel(path)); input.setAttribute("aria-describedby", hint.id); label.append(hint);
            }
            row.append(label);
            if (remove) row.append(button(text("remove"), () => { remove(); notify(); refresh(); }));
            return row;
        }
        const group = element("fieldset");
        group.dataset.templateGroup = path;
        group.append(element("legend", fieldLabel(path)));
        if (remove) group.append(button(text("remove"), () => { remove(); notify(); refresh(); }));
        if (Array.isArray(value)) {
            value.forEach((item, index) => group.append(editTree(item, `${path}.${index}`, (next) => { value[index] = next; }, () => { value.splice(index, 1); }, notify, refresh)));
            group.append(button(text("add"), () => { value.push(arraySeed(path, value)); notify(); refresh(); }));
        } else {
            const advanced = element("details"); advanced.className = "template-advanced"; advanced.dataset.templateAdvanced = path;
            advanced.append(element("summary", text("advanced")));
            const advancedKeys = new Set(["aggregation", "mode", "base_oid", "index_oid", "entity", "used_is_bytes", "fallbacks", "state_labels", "normal_states", "alarm_states", "invalid_values", "match_names", "offset", "scale", "data_type", "enterprise_id", "sysObjectID_prefix"]);
            for (const [key, item] of Object.entries(value)) {
                const locked = path === "profile" && ["enterprise_id", "sysObjectID_prefix"].includes(key);
                const field = editTree(item, `${path}.${key}`, (next) => { value[key] = next; },
                    ["cpu", "memory", "sensors", "metrics", "tables"].includes(key) && path === "profile"
                        || path.endsWith(".tables") || path.endsWith(".sensors") || path.endsWith(".state_labels")
                        || path.endsWith(".dom") && ["entity", "index_oid"].includes(key)
                        || /(?:\.sensors\.(?:temperature|power|fan)|\.metrics)\.\d+$/.test(path) && ["alarm_states", "name_oid", "match_name", "unit_oid", "match_unit", "scale_oid", "precision_oid", "warning_value"].includes(key)
                        ? () => { delete value[key]; } : undefined, notify, refresh);
                if (locked) field.querySelectorAll("input").forEach((input) => { input.disabled = true; });
                (advancedKeys.has(key) ? advanced : group).append(field);
            }
            if (path.endsWith(".state_labels")) {
                const key = element("input"); key.type = "number"; key.step = "1"; key.setAttribute("aria-label", fieldLabel(`${path}.0`));
                const label = element("input"); label.setAttribute("aria-label", fieldLabel("label"));
                group.append(key, label, button(text("add"), () => {
                    if (!Number.isSafeInteger(key.valueAsNumber) || !label.value.trim()) {
                        key.reportValidity(); label.focus(); return;
                    }
                    value[String(key.valueAsNumber)] = label.value; notify(); refresh();
                }));
            }
            const options = Object.entries(optionalSections(path, value)).filter(([key]) => path !== "profile" && !(key in value));
            if (options.length) {
                const label = element("label", text("property"));
                const select = element("select");
                select.setAttribute("aria-label", text("property"));
                for (const [key] of options) select.append(new Option(fieldLabel(`${path}.${key}`), key));
                label.append(select); advanced.append(label);
                advanced.append(button(text("add"), () => {
                    value[select.value] = clone(Object.fromEntries(options)[select.value]); notify(); refresh();
                }));
            }
            if (advanced.children.length > 1) group.append(advanced);
        }
        return group;
    };
    const dialog = (title: string): HTMLDialogElement => {
        activeDialog?.close();
        const previous = document.activeElement;
        const node = element("dialog"); node.className = "template-dialog"; node.id = "template-edit-dialog";
        const heading = element("h3", title); heading.id = "template-dialog-title";
        node.setAttribute("aria-labelledby", heading.id); node.append(heading);
        node.addEventListener("close", () => {
            node.remove();
            if (activeDialog === node) {
                activeDialog = undefined; modalDirty = false;
                if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
                else root.querySelector<HTMLSelectElement>("#template-profile")?.focus();
            }
        });
        activeDialog = node; document.body.append(node);
        return node;
    };
    const editRow = (row: DefinitionRow): void => {
        const node = dialog(`${text("edit")}: ${row.label}`);
        const form = element("form"), body = element("div"), alert = element("p"); alert.setAttribute("role", "alert");
        const actions = element("div"); actions.className = "template-dialog-actions";
        const apply = button(text("applyItem"), () => { }); apply.type = "submit";
        actions.append(button(text("cancel"), () => node.close()), apply);
        form.append(body, alert, actions); node.append(form);
        let local = clone(row.value);
        const commitDraft = capture;
        const refresh = (): void => {
            const open = Array.from(body.querySelectorAll<HTMLDetailsElement>("details[open]")).map(details => details.dataset.templateAdvanced);
            body.replaceChildren(editTree(local, row.path, next => { local = next; }, undefined, () => { modalDirty = true; }, refresh));
            body.querySelectorAll<HTMLDetailsElement>("details").forEach(details => { details.open = open.includes(details.dataset.templateAdvanced); });
        };
        refresh();
        form.onsubmit = event => {
            event.preventDefault();
            const invalid = Array.from(form.querySelectorAll<HTMLInputElement>("input[type=number][data-template-path]:not(:disabled)")).find(input => !input.checkValidity() || !Number.isFinite(input.valueAsNumber));
            if (invalid) {
                const details = invalid.closest("details"); if (details) details.open = true;
                alert.textContent = text("invalidNumber"); invalid.focus(); return;
            }
            row.commit(local); commitDraft(); dirty = true; modalDirty = false; message = ""; savedMessage = false; error = "";
            node.close(); render();
            const path = row.highlightPath || row.path;
            const updated = Array.from(root.querySelectorAll<HTMLElement>("[data-template-row]")).find(entry => entry.dataset.templateRow === path);
            if (updated) {
                updated.classList.add("template-row-highlight");
                window.setTimeout(() => updated.classList.remove("template-row-highlight"), 2500);
            }
        };
        node.showModal();
    };
    const rowsFor = (doc: Document, common: boolean): DefinitionRow[] => {
        const rows: DefinitionRow[] = [];
        const list = (values: Json[], path: string, title: string, commit: (values: Json[]) => void): void => {
            values.forEach((value, index) => rows.push({
                label: String(object(value).label || object(value).name || object(value).key || `${title} ${index + 1}`), path: `${path}.${index}`, value,
                commit: next => { values[index] = next; commit(values); }, remove: () => { values.splice(index, 1); commit(values); },
            }));
        };
        if (common) {
            if (category === "system") for (const key of ["cpu_oid", "memory_oid"]) {
                if (doc[key]) rows.push({ label: fieldLabel(`common.${key}`), path: `common.${key}`, value: doc[key], commit: next => { doc[key] = next; }, remove: () => { doc[key] = ""; } });
            }
            if (category === "hardware") list(array(doc.hardware_oids), "common.hardware_oids", fieldLabel("common.hardware_oids"), values => { doc.hardware_oids = values; });
            return rows;
        }
        if (category === "system") {
            list(array(object(doc.cpu).candidates), "profile.cpu.candidates", text("cpuCandidate"), values => { doc.cpu = { ...object(doc.cpu), candidates: values }; });
            if ("memory" in doc) {
                const memory = object(doc.memory);
                rows.push({ label: text("memorySource"), path: "profile.memory", value: Object.fromEntries(Object.entries(memory).filter(([key]) => key !== "fallbacks")), commit: next => { doc.memory = { ...object(next), ...(!("fallbacks" in object(next)) && memory.fallbacks ? { fallbacks: memory.fallbacks } : {}) }; }, remove: () => { delete doc.memory; } });
                list(array(memory.fallbacks), "profile.memory.fallbacks", fieldLabel("profile.memory.fallbacks"), values => { object(doc.memory).fallbacks = values; });
            }
            list(array(doc.metrics), "profile.metrics", text("customMetric"), values => { doc.metrics = values; });
        }
        if (category === "interfaces") {
            const tables = object(doc.tables), interfaces = object(tables.interfaces);
            for (const key of ["index", "name", "in_octets", "out_octets"]) if (key in interfaces) rows.push({
                label: fieldLabel(`profile.tables.interfaces.${key}`), path: `profile.tables.interfaces.${key}`, value: interfaces[key],
                commit: next => { interfaces[key] = next; }, remove: () => { delete interfaces[key]; },
            });
            list(array(object(tables.dom).metrics), "profile.tables.dom.metrics", text("domMetric"), values => { object(tables.dom).metrics = values; });
        }
        if (category === "hardware") for (const kind of ["temperature", "power", "fan"]) {
            list(array(object(doc.sensors)[kind]), `profile.sensors.${kind}`, fieldLabel(`profile.sensors.${kind}`), values => { doc.sensors = { ...object(doc.sensors), [kind]: values }; });
        }
        if (category === "profile") rows.push({ label: String(doc.name || text("profileTab")), path: "profile", value: Object.fromEntries(Object.entries(doc).filter(([key]) => ["name", "description", "sysObjectID_prefix", "enterprise_id"].includes(key))), configured: true, commit: next => { Object.assign(doc, object(next)); } });
        return rows;
    };
    const additionsFor = (doc: Document, common: boolean): DefinitionRow[] => {
        if (common) {
            if (category === "system") return ["cpu_oid", "memory_oid"].filter(key => !doc[key]).map(key => ({ label: fieldLabel(`common.${key}`), path: `common.${key}`, value: "", commit: next => { doc[key] = next; } }));
            if (category === "hardware") return [{ label: fieldLabel("common.hardware_oids"), path: `common.hardware_oids.${array(doc.hardware_oids).length}`, value: "temperature|", commit: next => { doc.hardware_oids = [...array(doc.hardware_oids), next]; } }];
            return [];
        }
        if (category === "system") return [
            { label: text("cpuCandidate"), path: `profile.cpu.candidates.${array(object(doc.cpu).candidates).length}`, value: "", commit: next => { doc.cpu = { ...object(doc.cpu), candidates: [...array(object(doc.cpu).candidates), next] }; } },
            {
                label: text("memorySource"), path: "profile.memory", highlightPath: "memory" in doc ? `profile.memory.fallbacks.${array(object(doc.memory).fallbacks).length}` : "profile.memory", value: { mode: "direct", oid: "", aggregation: "first", used_is_bytes: false }, commit: next => {
                    if ("memory" in doc) object(doc.memory).fallbacks = [...array(object(doc.memory).fallbacks), next]; else doc.memory = next;
                }
            },
            { label: text("customMetric"), path: `profile.metrics.${array(doc.metrics).length}`, value: arraySeed("profile.metrics", []), commit: next => { doc.metrics = [...array(doc.metrics), next]; } },
        ];
        if (category === "interfaces") return [
            ...["index", "name", "in_octets", "out_octets"].filter(key => !(key in object(object(doc.tables).interfaces))).map(key => ({
                label: fieldLabel(`profile.tables.interfaces.${key}`), path: `profile.tables.interfaces.${key}`, value: "", commit: (next: Json) => { doc.tables = { ...object(doc.tables), interfaces: { base_oid: "1.3.6.1.2.1.2.2.1", ...object(object(doc.tables).interfaces), [key]: next } }; },
            })),
            { label: text("domMetric"), path: `profile.tables.dom.metrics.${array(object(object(doc.tables).dom).metrics).length}`, value: arraySeed("profile.tables.dom.metrics", []), commit: next => { doc.tables = { ...object(doc.tables), dom: { base_oid: "", ...object(object(doc.tables).dom), metrics: [...array(object(object(doc.tables).dom).metrics), next] } }; } },
        ];
        if (category === "hardware") return ["temperature", "power", "fan"].map(kind => ({ label: fieldLabel(`profile.sensors.${kind}`), path: `profile.sensors.${kind}.${array(object(doc.sensors)[kind]).length}`, value: arraySeed(`profile.sensors.${kind}`, []), commit: next => { doc.sensors = { ...object(doc.sensors), [kind]: [...array(object(doc.sensors)[kind]), next] }; } }));
        return [];
    };
    const render = (): void => {
        capture = () => { };
        stopOperationProgress();
        root.replaceChildren(element("h2", text("title")));
        if (busy) {
            stopOperationProgress = showOperationProgress(root, text(operation), () => {
                const seconds = Math.floor((performance.now() - operationStarted) / 1000);
                return `${text(operation)}... ${localStorage.getItem("tracepulse-lang") === "ja" ? `（経過 ${seconds}秒）` : `(${seconds}s elapsed)`}`;
            });
        }
        const alert = element("p", error); alert.setAttribute("role", "alert"); root.append(alert);
        const reload = button(text("reload"), () => { if (!dirty || window.confirm(text("discard"))) void run(load); });
        if (!view || !definitions) { reload.disabled = busy; root.append(reload); return; }
        const currentView = view, draft = definitions;
        const choices = new Map(view.profiles.map(profile => [profile.id, profile]));
        for (const [id, document] of Object.entries(draft.profiles)) choices.set(id, { id, document, source: "database", base: choices.get(id)?.base || null, resettable: true });
        if (selected !== "@common" && !choices.has(selected)) selected = "generic";
        const common = selected === "@common", profile = choices.get(selected);
        const doc: Document = common ? { cpu_oid: draft.common.cpu_oid, memory_oid: draft.common.memory_oid, hardware_oids: draft.common.hardware_oids } : draft.profiles[selected] || clone(profile!.document);
        const profileId = selected;
        capture = () => {
            if (common) draft.common = { cpu_oid: String(doc.cpu_oid), memory_oid: String(doc.memory_oid), hardware_oids: array(doc.hardware_oids).map(String) };
            else draft.profiles[profileId] = doc;
        };
        const toolbar = element("div"); toolbar.className = "template-toolbar";
        if (dirty) toolbar.dataset.state = "dirty";
        const label = element("label", text("profile")), select = element("select"); select.id = "template-profile";
        const deviceGroup = element("optgroup"), commonGroup = element("optgroup");
        deviceGroup.label = text("deviceProfiles"); commonGroup.label = text("commonProfiles");
        commonGroup.append(new Option(text("common"), "@common"));
        for (const item of choices.values()) (selector(item.document) ? deviceGroup : commonGroup).append(new Option(`${item.document.name} (${item.id})`, item.id));
        select.append(deviceGroup, commonGroup);
        select.value = selected; select.onchange = () => { selected = select.value; render(); };
        label.append(select); toolbar.append(label);
        const save = button(text("save"), () => void run(async () => {
            const next = await api<View>("/api/snmp/definitions", { generation: currentView.generation, definitions: draft });
            view = next; definitions = clone(next.definitions); dirty = false; message = text("saved"); savedMessage = true;
            flashGeneration = true; showSavedToast(next.applied_generation);
        }, "saving"));
        save.className = "template-save"; toolbar.append(save);
        if (profile) {
            toolbar.append(button(text("reset"), () => {
                if (!window.confirm(text("resetConfirm"))) return;
                delete draft.profiles[selected];
                const original = view!.profiles.find(item => item.id === selected);
                if (profile.base && original) { original.document = clone(profile.base); original.source = "file"; }
                else { view!.profiles = view!.profiles.filter(item => item.id !== selected); selected = "generic"; }
                capture = () => { }; changed(); render();
            }));
            toolbar.append(button(text("export"), () => void run(async () => {
                const data = await api<{ toml: string }>("/api/snmp/definitions/convert", { document: doc });
                const url = URL.createObjectURL(new Blob([data.toml], { type: "application/toml" }));
                const link = element("a"); link.href = url; link.download = `${selected.split(".").join("-")}.toml`; link.click();
                setTimeout(() => URL.revokeObjectURL(url), 1000);
            }, "exporting")));
        }
        const file = element("input"); file.type = "file"; file.accept = ".toml"; file.id = "template-import"; file.hidden = true;
        file.onchange = () => {
            const selectedFile = file.files?.[0]; if (!selectedFile) return;
            void run(async () => {
                const result = await api<{ document: Document }>("/api/snmp/definitions/convert", { toml: await selectedFile.text() });
                const imported = result.document;
                const id = typeof imported.sysObjectID_prefix === "string" ? imported.sysObjectID_prefix.replace(/^\.|\.$/g, "") : Number(imported.enterprise_id) ? `1.3.6.1.4.1.${imported.enterprise_id}` : "generic";
                if (choices.has(id) && !window.confirm(`${text("import")}: ${id}?`)) return;
                draft.profiles[id] = imported; selected = id; capture = () => { }; changed();
            }, "importing");
        };
        toolbar.append(file, button(text("import"), () => file.click()), reload);
        toolbar.append(button(text("newProfileTitle"), () => {
            const node = dialog(text("newProfileTitle")), form = element("form");
            form.append(element("p", text("profileGuide")));
            const nameLabel = element("label", text("profileName")), name = element("input");
            name.id = "template-new-name"; name.required = true; name.placeholder = text("profileNameExample"); nameLabel.append(name);
            const deviceLabel = element("label", text("registeredDevice")), deviceSelect = element("select");
            deviceSelect.id = "template-new-device"; deviceSelect.append(new Option(text("chooseDevice"), ""));
            for (const device of devices) {
                deviceSelect.append(new Option(`${device.name || device.ip} (${device.ip}) - ${device.sys_object_id || text("unknownOid")}`, device.ip));
            }
            deviceLabel.append(deviceSelect);
            const ipLabel = element("label", text("probeIp")), ip = element("input");
            ip.id = "template-probe-ip"; ip.placeholder = "192.0.2.10"; ipLabel.append(ip);
            const communityLabel = element("label", text("community")), community = element("input");
            community.type = "password"; community.autocomplete = "off"; communityLabel.append(community);
            const newLabel = element("label", text("newProfile")), prefix = element("input"); prefix.id = "template-new-prefix"; prefix.placeholder = "1.3.6.1.4.1.12345"; prefix.required = true;
            newLabel.append(prefix); const failure = element("p"); failure.setAttribute("role", "alert");
            let fullOid = "";
            const scope = element("p"), probeStatus = element("p"); probeStatus.setAttribute("role", "status");
            scope.setAttribute("aria-live", "polite"); scope.id = "template-scope-preview"; prefix.setAttribute("aria-describedby", scope.id);
            const vendorPrefix = (): string => /^1\.3\.6\.1\.4\.1\.\d+(?=\.|$)/.exec(fullOid)?.[0] || "";
            const updateScope = (): void => {
                scope.textContent = validOid(prefix.value)
                    ? `${localStorage.getItem("tracepulse-lang") === "ja" ? `"${prefix.value}" ${text("scopePreview")}` : `${text("scopePreview")} "${prefix.value}".`} ${text("longestHint")}`
                    : text("noSelector");
                vendor.disabled = !validOid(fullOid) || !vendorPrefix(); model.disabled = !validOid(fullOid);
            };
            const useOid = (oid: string): void => { prefix.value = oid.replace(/^\./, ""); fullOid = prefix.value; modalDirty = true; updateScope(); };
            const vendor = button(text("vendorScope"), () => { prefix.value = vendorPrefix(); modalDirty = true; updateScope(); });
            const model = button(text("modelScope"), () => { prefix.value = fullOid; modalDirty = true; updateScope(); });
            const scopeActions = element("div"); scopeActions.className = "template-scope-actions"; scopeActions.append(vendor, model);
            prefix.oninput = () => { fullOid = prefix.value.replace(/^\./, ""); modalDirty = true; updateScope(); };
            deviceSelect.onchange = () => {
                const device = devices.find(device => device.ip === deviceSelect.value);
                if (!device) return;
                ip.value = device.ip; modalDirty = true;
                useOid(device.sys_object_id || "");
            };
            let probing = false;
            const probe = button(text("probe"), () => {
                if (probing) return;
                probing = true; probe.disabled = true; create.disabled = true; failure.textContent = ""; probeStatus.textContent = text("probing");
                const targetIp = ip.value.trim();
                void api<{ sys_object_id: string; sys_name: string | null }>("/api/snmp/probe-sysobjectid", { ip: targetIp, community: community.value })
                    .then(result => {
                        if (!node.open) return;
                        useOid(result.sys_object_id);
                        const device = devices.find(device => device.ip === targetIp);
                        if (device) device.sys_object_id = result.sys_object_id;
                        probeStatus.textContent = [result.sys_name, result.sys_object_id].filter(Boolean).join(" — ");
                    }).catch(reason => { if (node.open) failure.textContent = String(reason); })
                    .finally(() => { probing = false; probe.disabled = false; create.disabled = false; if (node.open && failure.textContent) probeStatus.textContent = ""; });
            });
            const baseLabel = element("label", text("baseTemplate")), base = element("select"); base.id = "template-new-base";
            for (const item of choices.values()) base.append(new Option(String(item.document.name), item.id));
            base.append(new Option(text("emptyTemplate"), "@empty")); base.value = "generic"; baseLabel.append(base);
            const create = button(text("create"), () => { }); create.type = "submit";
            const actions = element("div"); actions.className = "template-dialog-actions"; actions.append(button(text("cancel"), () => node.close()), create);
            form.append(nameLabel, deviceLabel, ipLabel, communityLabel, probe, probeStatus, newLabel, scope, scopeActions, baseLabel, failure, actions); node.append(form);
            form.addEventListener("input", () => { modalDirty = true; });
            form.onsubmit = event => {
                event.preventDefault();
                if (probing) return;
                const id = prefix.value.trim().replace(/^\./, "");
                if (!validOid(id)) { failure.textContent = text("noSelector"); return; }
                if (!name.value.trim()) { name.focus(); return; }
                if (choices.has(id)) { failure.textContent = text("duplicateProfile"); return; }
                const document = base.value === "@empty" ? { description: "", cpu: { candidates: [] }, memory: {}, sensors: {}, metrics: [], tables: {} } : clone(choices.get(base.value)!.document);
                const enterprise = /^1\.3\.6\.1\.4\.1\.(\d+)(?=\.|$)/.exec(id);
                draft.profiles[id] = { ...document, name: name.value.trim(), sysObjectID_prefix: id, enterprise_id: enterprise ? Number(enterprise[1]) : 0 };
                dirty = true; message = ""; savedMessage = false; capture = () => { };
                selected = id; node.close(); render();
            };
            updateScope();
            node.showModal();
        }));
        root.append(toolbar);
        const statusBar = element("div"); statusBar.className = "template-status-bar";
        const status = element("p", [dirty ? text("dirty") : "", savedMessage ? text("saved") : message].filter(Boolean).join(" — "));
        status.dataset.templateStatus = ""; status.setAttribute("role", "status");
        if (dirty) status.dataset.state = "dirty"; else if (savedMessage) status.dataset.state = "saved";
        const generation = element("p", `${text("version")}: ${view.generation} / ${view.applied_generation}`);
        generation.className = "template-generation";
        if (flashGeneration && !busy) {
            generation.classList.add("template-generation-flash"); flashGeneration = false;
            window.setTimeout(() => generation.classList.remove("template-generation-flash"), 1500);
        }
        statusBar.append(status, generation); root.append(statusBar);
        {
            const matches = common ? devices : devices.filter(device => selectedProfile(device.sys_object_id, choices) === selected);
            const count = element("dd", String(matches.length)); count.dataset.templateMatches = "";
            count.title = matches.map(device => `${device.name || device.ip} (${device.ip})`).join("\n");
            const condition = common ? text("commonCondition") : selector(doc) || text("fallbackCondition");
            const sourceInfo = element("dl"); sourceInfo.className = "template-source";
            sourceInfo.append(element("dt", text("condition")), element("dd", condition), element("dt", text("matchingDevices")), count);
            if (profile) {
                const source = draft.profiles[selected] ? "database" : profile.source;
                const sources: Record<string, [string, string]> = { database: ["User definition (SQLite)", "ユーザー定義（SQLite）"], bundled: ["Bundled template", "同梱テンプレート"], byo: ["External TOML template", "外部TOMLテンプレート"], file: ["Original template", "元のテンプレート"] };
                sourceInfo.dataset.templateSource = source;
                sourceInfo.prepend(element("dt", text("source")), element("dd", sources[source]?.[localStorage.getItem("tracepulse-lang") === "ja" ? 1 : 0] || source));
            }
            root.append(sourceInfo);
        }
        const categories: [Category, Key][] = common ? [["system", "systemTab"], ["hardware", "hardwareTab"]] : [["system", "systemTab"], ["interfaces", "interfacesTab"], ["hardware", "hardwareTab"], ["profile", "profileTab"]];
        if (!categories.some(([key]) => key === category)) category = "system";
        const tabs = element("div"); tabs.className = "template-tabs"; tabs.setAttribute("role", "tablist"); tabs.setAttribute("aria-label", text("title"));
        categories.forEach(([key, title], index) => {
            const tab = button(text(title), () => { category = key; render(); root.querySelector<HTMLElement>(`#template-tab-${key}`)?.focus(); });
            tab.id = `template-tab-${key}`; tab.setAttribute("role", "tab"); tab.setAttribute("aria-selected", String(category === key)); tab.setAttribute("aria-controls", "template-panel"); tab.tabIndex = category === key ? 0 : -1;
            tab.onkeydown = event => {
                const next = event.key === "ArrowRight" ? (index + 1) % categories.length : event.key === "ArrowLeft" ? (index + categories.length - 1) % categories.length : event.key === "Home" ? 0 : event.key === "End" ? categories.length - 1 : -1;
                if (next >= 0) { event.preventDefault(); category = categories[next][0]; render(); root.querySelector<HTMLElement>(`#template-tab-${category}`)?.focus(); }
            };
            tabs.append(tab);
        });
        root.append(tabs);
        const panel = element("div"); panel.id = "template-panel"; panel.dataset.templateCategory = category; panel.setAttribute("role", "tabpanel"); panel.setAttribute("aria-labelledby", `template-tab-${category}`);
        const listActions = element("div"); listActions.className = "template-list-actions";
        const settings = (title: Key, path: string, section: Document, excluded: string[], commit: (next: Document) => void): void => {
            listActions.append(button(text(title), () => editRow({ label: text(title), path, value: Object.fromEntries(Object.entries(section).filter(([key]) => !excluded.includes(key))), commit: next => commit(object(next)) })));
        };
        if (!common && category === "system") settings("cpuSettings", "profile.cpu", { aggregation: "first", ...object(doc.cpu) }, ["candidates"], next => { doc.cpu = { ...next, candidates: array(object(doc.cpu).candidates) }; });
        if (!common && category === "interfaces") {
            settings("interfaceSettings", "profile.tables.interfaces", { base_oid: "1.3.6.1.2.1.2.2.1", ...object(object(doc.tables).interfaces) }, ["index", "name", "in_octets", "out_octets"], next => { doc.tables = { ...object(doc.tables), interfaces: { ...object(object(doc.tables).interfaces), ...next } }; });
            settings("domSettings", "profile.tables.dom", { base_oid: "", ...object(object(doc.tables).dom) }, ["metrics"], next => { doc.tables = { ...object(doc.tables), dom: { ...next, metrics: array(object(object(doc.tables).dom).metrics) } }; });
        }
        const additions = additionsFor(doc, common);
        if (category !== "profile") {
            const add = button(text("addItem"), () => {
                const node = dialog(text("addItem")), label = element("label", text("itemType")), kind = element("select");
                kind.setAttribute("aria-label", text("itemType"));
                additions.forEach((row, index) => kind.append(new Option(row.label, String(index)))); label.append(kind);
                node.append(label, button(text("cancel"), () => node.close()), button(text("add"), () => editRow(additions[Number(kind.value)])));
                node.showModal();
            });
            add.disabled = !additions.length; listActions.append(add);
        }
        panel.append(listActions);
        const scroll = element("div"); scroll.className = "template-table-scroll"; scroll.tabIndex = 0; scroll.setAttribute("role", "region"); scroll.setAttribute("aria-label", text(categories.find(([key]) => key === category)![1]));
        const table = element("table"); table.className = "template-items"; table.id = "template-items";
        const head = element("thead"), header = element("tr");
        for (const key of ["item", "oids", "configuration", "edit"] as Key[]) { const cell = element("th", text(key)); cell.scope = "col"; header.append(cell); }
        head.append(header); table.append(head); const body = element("tbody");
        const rows = rowsFor(doc, common);
        for (const row of rows) {
            const entry = element("tr"); entry.dataset.templateRow = row.path;
            entry.append(element("td", row.label)); const oidCell = element("td"), oids = configuredOids(row.value);
            if (oids.length) for (const oid of oids) { const code = element("code", oid); code.className = "template-oid"; oidCell.append(code); }
            else oidCell.textContent = row.configured ? String(object(row.value).description || object(row.value).name || "—") : "—";
            entry.append(oidCell, element("td", text(row.configured || oids.length ? "configured" : "unconfigured")));
            const controls = element("td"), actions = element("div"); actions.className = "template-row-actions";
            const edit = button(text("edit"), () => editRow(row)); edit.setAttribute("aria-label", `${text("edit")}: ${row.label}`); actions.append(edit);
            if (row.remove) actions.append(button(text("remove"), () => { if (window.confirm(text("confirmRemove"))) { row.remove!(); changed(); render(); } }));
            controls.append(actions); entry.append(controls); body.append(entry);
        }
        if (!rows.length) { const empty = element("tr"), cell = element("td", text("empty")); cell.colSpan = 4; empty.append(cell); body.append(empty); }
        table.append(body); scroll.append(table); panel.append(scroll); root.append(panel);
        const preview = element("details"); preview.append(element("summary", text("preview")));
        const previewLabel = element("label", text("sysObject")), sysObject = element("input"); sysObject.id = "template-preview-oid"; previewLabel.append(sysObject);
        preview.append(previewLabel, button(text("preview"), () => {
            const oid = sysObject.value;
            void run(async () => { const result = await api<{ id: string; document: Document }>("/api/snmp/definitions/preview", { definitions: draft, sys_object_id: oid }); message = `${text("preview")}: ${result.document.name} (${result.id})`; savedMessage = false; }, "previewing");
        }));
        const notes = element("details"); notes.append(element("summary", text("notes")), element("p", text("help")), element("p", text("storage")), element("p", text("disclaimer")));
        root.append(preview, notes);
        if (busy) root.querySelectorAll<HTMLInputElement | HTMLButtonElement | HTMLSelectElement>("input,button,select").forEach(node => { node.disabled = true; });
    };
    window.addEventListener("tracepulse-language-change", () => { if (!activeDialog?.open) render(); });
    window.addEventListener("beforeunload", event => { if (dirty || modalDirty) { event.preventDefault(); event.returnValue = ""; } });
    void run(load);
}
