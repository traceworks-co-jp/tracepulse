import { expect, test, type Page } from "@playwright/test";

test.beforeEach(async ({ page, context }) => {
    await page.addInitScript(() => {
        localStorage.setItem("tracepulse-onboarding-v1", "done");
        localStorage.setItem("tracepulse-lang", "en");
    });
    page.on("pageerror", error => { throw error; });
    await context.route("**/static/js/snmp-definitions.js*", route => route.fulfill({ path: "frontend/dist/snmp-definitions.js", contentType: "application/javascript" }));
});

async function openRow(page: Page, path: string) {
    const category = path.startsWith("profile.tables") ? "interfaces" : path.startsWith("profile.sensors") || path.startsWith("common.hardware") ? "hardware" : path === "profile" ? "profile" : "system";
    await page.locator(`#template-tab-${category}`).click();
    await page.locator(`[data-template-row="${path}"]`).getByRole("button", { name: /^Edit:/ }).click();
    return page.getByRole("dialog");
}

async function editField(page: Page, row: string, path: string, value: string) {
    const dialog = await openRow(page, row);
    await dialog.locator(`[data-template-path="${path}"]`).fill(value);
    await dialog.getByRole("button", { name: "Apply to draft", exact: true }).click();
    await expect(dialog).not.toBeVisible();
}

async function createProfile(page: Page, prefix: string) {
    await page.locator(".template-toolbar").getByRole("button", { name: "Create profile", exact: true }).click();
    const dialog = page.getByRole("dialog");
    await dialog.locator("#template-new-prefix").fill(prefix);
    await dialog.getByRole("button", { name: "Create profile", exact: true }).click();
}

async function addItem(page: Page, category: string, label: string) {
    await page.locator(`#template-tab-${category}`).click();
    await page.locator("#snmp-definitions").getByRole("button", { name: "+ Add item", exact: true }).click();
    await page.getByRole("dialog").getByLabel("Item type", { exact: true }).selectOption({ label });
    await page.getByRole("dialog").getByRole("button", { name: "Add", exact: true }).click();
    return page.getByRole("dialog");
}

for (const [width, language, theme] of [[1280, "en", "light"], [375, "ja", "dark"]] as const) {
    test(`remaining definition loading save and import progress at ${width} ${language}`, async ({ page, request }) => {
        const view = (await (await request.get("/api/snmp/definitions")).json()).data;
        await page.setViewportSize({ width, height: 900 });
        await page.addInitScript(lang => localStorage.setItem("tracepulse-lang", lang), language);
        let finish: ((success: boolean) => void) | undefined;
        let requests = 0;
        await page.route("**/api/snmp/definitions", async route => {
            requests++;
            const success = await new Promise<boolean>(resolve => { finish = resolve; });
            await route.fulfill(success ? { json: { ok: true, data: view } } : { status: 500, json: { error: "Definition failed" } });
        });
        await page.route("**/api/snmp/definitions/convert", async route => {
            requests++;
            const success = await new Promise<boolean>(resolve => { finish = resolve; });
            await route.fulfill(success ? { json: { ok: true, data: { document: { name: "Progress import", sysObjectID_prefix: "1.3.6.1.4.1.65099", enterprise_id: 65099, cpu: { candidates: [] }, memory: {}, sensors: {}, metrics: [], tables: {} } } } } : { status: 500, json: { error: "Import failed" } });
        });
        await page.goto("/settings/snmp");
        await page.locator("#theme-select").selectOption(theme);
        const root = page.locator("#snmp-definitions");
        const progress = root.locator(".operation-progress");
        await expect(progress.getByRole("status")).toHaveText(language === "ja" ? /監視定義を読み込んでいます.*経過 [1-9]\d*秒/ : /Loading monitoring definitions.*[1-9]\d*s elapsed/);
        await expect(root.locator("button").first()).toBeDisabled();
        finish!(true);
        await expect(progress).toHaveCount(0);
        await root.locator(".template-save").click();
        await expect(progress).toBeInViewport();
        await expect(progress.getByRole("status")).toContainText(language === "ja" ? "監視定義を保存・適用しています" : "Saving and applying monitoring definitions");
        await expect(root.locator(".template-save")).toBeDisabled();
        await expect.poll(() => requests).toBe(2);
        await progress.screenshot({ path: `test-results/definition-progress-${width}-${theme}.png` });
        finish!(false);
        await expect(root.getByRole("alert")).toContainText("Definition failed");
        await expect(progress).toHaveCount(0);
        await root.locator(".template-save").click();
        await expect.poll(() => requests).toBe(3);
        finish!(true);
        await expect(progress).toHaveCount(0);
        const file = { name: "progress.toml", mimeType: "application/toml", buffer: Buffer.from('name = "Progress import"\nsysObjectID_prefix = "1.3.6.1.4.1.65099"\n') };
        await root.locator("#template-import").setInputFiles(file);
        await expect(progress.getByRole("status")).toContainText(language === "ja" ? "TOMLをインポートしています" : "Importing TOML");
        await expect.poll(() => requests).toBe(4);
        finish!(false);
        await expect(root.getByRole("alert")).toContainText("Import failed");
        await expect(progress).toHaveCount(0);
        await root.locator("#template-import").setInputFiles(file);
        await expect.poll(() => requests).toBe(5);
        finish!(true);
        await expect(progress).toHaveCount(0);
        await expect(root.locator("#template-profile")).toHaveValue("1.3.6.1.4.1.65099");
        await expect(root.locator("[data-template-status]")).toContainText(language === "ja" ? "未保存" : "Unsaved");
    });
}

test("profile selector and action buttons share a baseline and control height", async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem("tracepulse-lang", "ja"));
    await page.goto("/settings/snmp");
    const toolbar = page.locator(".template-toolbar");
    const profile = page.locator("#template-profile");
    for (const width of [1920, 1280, 375]) {
        await page.setViewportSize({ width, height: 900 });
        for (const id of ["generic", "@common"]) {
            await profile.selectOption(id);
            for (const theme of ["dark", "light"]) {
                await page.locator("#theme-select").selectOption(theme);
                await expect(profile).toHaveCSS("height", "40px");
                const selectBounds = (await profile.boundingBox())!;
                const buttons = await toolbar.locator("button").all();
                for (const control of buttons) {
                    const bounds = (await control.boundingBox())!;
                    expect(bounds.height).toBeGreaterThanOrEqual(40);
                    if (width > 600) {
                        expect(bounds.height).toBe(40);
                        if (Math.abs(bounds.y - selectBounds.y) < 40) {
                            expect(Math.abs(bounds.y - selectBounds.y)).toBeLessThanOrEqual(1);
                            expect(Math.abs(bounds.y + bounds.height - selectBounds.y - selectBounds.height)).toBeLessThanOrEqual(1);
                        }
                    } else expect(bounds.y).toBeGreaterThanOrEqual(selectBounds.y + selectBounds.height);
                }
                expect(await toolbar.evaluate(element => element.scrollWidth <= element.clientWidth)).toBeTruthy();
                await toolbar.screenshot({ path: `test-results/template-toolbar-${width}-${id === "@common" ? "common" : "profile"}-${theme}.png` });
            }
        }
    }
});

test("category tables use cancellable modals, collapsed details and a sticky action bar", async ({ page }) => {
    await page.goto("/settings/snmp");
    const root = page.locator("#snmp-definitions");
    await expect(root.getByRole("tab")).toHaveCount(4);
    await expect(root.locator("fieldset")).toHaveCount(0);
    await expect(root.locator(".template-toolbar")).toHaveCSS("position", "sticky");
    const original = await root.locator('[data-template-row="profile.cpu.candidates.0"]').textContent() || "";
    let dialog = await openRow(page, "profile.cpu.candidates.0");
    await dialog.locator('[data-template-path="profile.cpu.candidates.0"]').fill("1.3.6.1.4.1.65000.999");
    await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(root.locator('[data-template-row="profile.cpu.candidates.0"]')).toHaveText(original);
    await expect(root.locator("[data-template-status]")).toBeEmpty();
    await root.getByRole("button", { name: "CPU settings", exact: true }).click();
    dialog = page.getByRole("dialog");
    await expect(dialog.locator("details")).not.toHaveAttribute("open");
    await dialog.getByText("Advanced settings", { exact: true }).click();
    await expect(dialog.getByLabel("Table value selection", { exact: true })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(dialog).not.toBeVisible();
    await root.getByRole("tab", { name: "CPU / memory", exact: true }).focus();
    await page.keyboard.press("ArrowRight");
    await expect(root.getByRole("tab", { name: "Interfaces / optical power", exact: true })).toBeFocused();
    await expect(root.locator('[data-template-row="profile.tables.interfaces.in_octets"]')).toContainText("1.3.6.1.2.1.2.2.1.10");
    for (const width of [1280, 375]) {
        await page.setViewportSize({ width, height: 900 });
        for (const theme of ["dark", "light"]) {
            await page.locator("#theme-select").selectOption(theme);
            expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
            await root.locator(".template-table-scroll").evaluate(element => { element.scrollLeft = 0; });
            await root.screenshot({ path: `test-results/template-table-${width}-${theme}.png` });
            dialog = await openRow(page, "profile.tables.interfaces.in_octets");
            expect(await dialog.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
            await dialog.screenshot({ path: `test-results/template-modal-${width}-${theme}.png` });
            await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
        }
    }
    await page.locator("#lang-select").selectOption("ja");
    await expect(root.getByRole("tab", { name: "インターフェース・光レベル", exact: true })).toHaveAttribute("aria-selected", "true");
    for (const width of [1280, 375]) {
        await page.setViewportSize({ width, height: 900 });
        await root.locator(".template-table-scroll").evaluate(element => { element.scrollLeft = 0; });
        await root.screenshot({ path: `test-results/template-table-ja-${width}.png` });
    }
});

test("sensor, DOM and common OID modal additions preserve unrelated profile definitions", async ({ page, request }) => {
    const read = async () => (await (await request.get("/api/snmp/definitions")).json()).data;
    const original = await read();
    const generic = original.profiles.find((profile: { id: string }) => profile.id === "generic").document;
    try {
        await page.goto("/settings/snmp");
        let dialog = await addItem(page, "hardware", "Temperature sensors");
        for (const [key, oid] of [["descr_prefix", "1.3.6.1.4.1.65000.80.1"], ["value_prefix", "1.3.6.1.4.1.65000.80.2"], ["state_prefix", "1.3.6.1.4.1.65000.80.3"]]) {
            await dialog.locator(`[data-template-path="profile.sensors.temperature.0.${key}"]`).fill(oid);
        }
        await dialog.getByRole("button", { name: "Apply to draft", exact: true }).click();
        dialog = await addItem(page, "interfaces", "Optical power metric");
        await dialog.locator('[data-template-path="profile.tables.dom.metrics.0.oid"]').fill("1.3.6.1.4.1.65000.81.2");
        await dialog.getByRole("button", { name: "Apply to draft", exact: true }).click();
        await page.locator("#snmp-definitions").getByRole("button", { name: "Optical table settings", exact: true }).click();
        dialog = page.getByRole("dialog");
        await dialog.getByText("Advanced settings", { exact: true }).click();
        await dialog.locator('[data-template-path="profile.tables.dom.base_oid"]').fill("1.3.6.1.4.1.65000.81");
        await dialog.getByRole("button", { name: "Apply to draft", exact: true }).click();
        await page.locator("#template-profile").selectOption("@common");
        dialog = await addItem(page, "hardware", "Additional hardware sensor OIDs");
        await dialog.locator('[data-template-path="common.hardware_oids.0"]').fill("temperature|1.3.6.1.4.1.65000.82.0");
        await dialog.getByRole("button", { name: "Apply to draft", exact: true }).click();
        const row = page.locator('[data-template-row="common.hardware_oids.0"]');
        await expect(row.locator("code")).toHaveText("1.3.6.1.4.1.65000.82.0");
        await expect(row).toContainText("Configured");
        await page.getByRole("button", { name: "Save and apply", exact: true }).click();
        await expect(page.locator("[data-template-status]")).toHaveText("Saved and applied");
        const saved = await read();
        expect(saved.definitions.profiles.generic.cpu).toEqual(generic.cpu);
        expect(saved.definitions.profiles.generic.tables.interfaces).toEqual(generic.tables.interfaces);
        expect(saved.definitions.profiles.generic.sensors.temperature[0].value_prefix).toBe("1.3.6.1.4.1.65000.80.2");
        expect(saved.definitions.profiles.generic.tables.dom.metrics[0].scale).toBe(0.01);
        expect(saved.definitions.common.hardware_oids).toContain("temperature|1.3.6.1.4.1.65000.82.0");
    } finally {
        const current = await read();
        expect((await (await request.post("/api/snmp/definitions", { data: { generation: current.generation, definitions: original.definitions } })).json()).ok).toBe(true);
    }
});

test("definition source is a readable localized metadata row across profiles and themes", async ({ page, request }) => {
    const response = await (await request.get("/api/snmp/definitions")).json();
    response.data.definitions.profiles = {};
    const sources = [
        ["generic", "bundled", "同梱テンプレート"],
        ["1.3.6.1.4.1.9", "byo", "外部TOMLテンプレート"],
        ["1.3.6.1.4.1.2636", "database", "ユーザー定義（SQLite）"],
    ];
    for (const [id, source] of sources) response.data.profiles.find((profile: { id: string }) => profile.id === id).source = source;
    await page.route("**/api/snmp/definitions", route => route.fulfill({ json: response }));
    await page.goto("/settings/snmp");
    await page.locator("#lang-select").selectOption("ja");
    const sourceInfo = page.locator("#snmp-definitions .template-source");
    for (const width of [1280, 375]) {
        await page.setViewportSize({ width, height: 900 });
        for (const theme of ["dark", "light"]) {
            await page.locator("#theme-select").selectOption(theme);
            if (theme === "light") {
                await expect(sourceInfo).toHaveCSS("background-color", "rgb(241, 245, 249)");
                await expect(sourceInfo.locator("dd")).toHaveCSS("color", "rgb(15, 23, 42)");
            }
            for (const [id, source, label] of sources) {
                await page.locator("#template-profile").selectOption(id);
                await expect(sourceInfo).toHaveAttribute("data-template-source", source);
                await expect(sourceInfo.locator("dt")).toHaveText("定義元");
                await expect(sourceInfo.locator("dd")).toHaveText(label);
                await expect(sourceInfo.locator("dd")).toHaveCSS("font-weight", "600");
                expect(await sourceInfo.evaluate(element => element.scrollWidth <= element.clientWidth)).toBeTruthy();
                if (id === "generic") await sourceInfo.screenshot({ path: `test-results/template-source-${width}-${theme}.png` });
            }
        }
    }
    await page.locator("#lang-select").selectOption("en");
    await expect(sourceInfo.locator("dt")).toHaveText("Definition source");
    await expect(sourceInfo.locator("dd")).toHaveText("User definition (SQLite)");
    await page.locator("#template-profile").selectOption("@common");
    await expect(sourceInfo).toHaveCount(0);
});

test("diagnostics stays separate and settings opens the localized definition editor", async ({ page }) => {
    const requests: string[] = [];
    page.on("request", (request) => requests.push(request.url()));
    await page.goto("/diagnostics");
    await expect(page.locator("#diag-run-btn")).toBeVisible();
    await expect(page.locator("main .settings-section")).toHaveCount(1);
    await expect(page.locator("[data-i18n=diagnostics_help]")).toHaveCount(1);
    await expect(page.getByLabel("Community", { exact: true })).toHaveAttribute("placeholder", "Leave blank to use configured community");
    await page.locator("#lang-select").selectOption("ja");
    await expect(page.getByLabel("コミュニティ", { exact: true })).toHaveAttribute("placeholder", "空欄の場合は設定済みのコミュニティを使用");
    await page.locator("#diag-run-btn").click();
    expect(await page.locator("#diag-ip").evaluate((input: HTMLInputElement) => input.validity.valueMissing)).toBe(true);
    await page.locator("#lang-select").selectOption("en");
    await expect(page.locator("#snmp-definitions")).toHaveCount(0);
    await expect(page.locator('script[src="/static/js/snmp-definitions.js"]')).toHaveCount(0);
    expect(requests.some((url) => url.endsWith("/api/snmp/definitions"))).toBe(false);
    await page.getByRole("link", { name: "Settings", exact: true }).click();
    await page.getByRole("link", { name: "Manage SNMP Settings", exact: true }).click();
    await expect(page).toHaveURL(/\/settings\/snmp$/);
    const root = page.locator("#snmp-definitions");
    await expect(root.locator('[data-template-row="profile.cpu.candidates.0"]')).toContainText("CPU utilization OID 1");
    await root.getByRole("tab", { name: "Interfaces / optical power", exact: true }).click();
    await expect(root.locator('[data-template-row="profile.tables.interfaces.in_octets"]')).toContainText("Received traffic (ifInOctets)");
    await expect(root).not.toContainText("profile.tables");
    await expect(root).not.toContainText("profile.cpu");
    for (const id of ["1.3.6.1.4.1.9", "1.3.6.1.4.1.2636", "1.3.6.1.4.1.1182"]) {
        await page.locator("#template-profile").selectOption(id);
        await expect(root).not.toContainText(/profile\./);
    }
    await page.locator("#template-profile").selectOption("generic");
    await page.locator("#lang-select").selectOption("ja");
    await expect(page.getByRole("heading", { name: "SNMP設定", exact: true })).toBeVisible();
    await expect(root.locator('[data-template-row="profile.tables.interfaces.in_octets"]')).toContainText("受信トラフィック（ifInOctets）");
    await expect(root.locator('[data-template-row="profile.tables.interfaces.name"]')).toContainText("インターフェース名（ifDescr）");
    await root.getByRole("tab", { name: "CPU・メモリ", exact: true }).click();
    await expect(root.locator('[data-template-row="profile.cpu.candidates.0"]')).toContainText("CPU使用率OID 1");
    await page.getByRole("link", { name: "設定に戻る", exact: true }).click();
    await expect(page).toHaveURL(/\/settings$/);
});

test("connection and definition saves are independent and failed credential saves retain input", async ({ page, request }) => {
    await page.goto("/settings/snmp");
    const root = page.locator("#snmp-definitions");
    await expect(page.locator("#template-profile")).toHaveValue("generic");
    const original = (await (await request.get("/api/snmp/definitions")).json()).data;
    await editField(page, "profile", "profile.name", "Unsaved profile draft");
    await page.locator("#community").fill("connection-only-test-secret");
    await page.getByRole("button", { name: "Show community", exact: true }).click();
    await expect(page.locator("#community")).toHaveAttribute("type", "text");
    let settingsPayload: unknown;
    await page.route("**/api/settings", async (route) => {
        settingsPayload = route.request().postDataJSON();
        await route.fulfill({ json: { ok: true } });
    });
    await page.getByRole("button", { name: "Save Connection Settings", exact: true }).click();
    await expect(page.locator("#snmp-connection-status")).not.toBeEmpty();
    expect(settingsPayload).toEqual({ default_community: "connection-only-test-secret" });
    await expect(page.locator("#community")).toHaveValue("");
    await expect(page.locator("#community")).toHaveAttribute("type", "password");
    await expect(page.locator("#template-profile option:checked")).toContainText("Unsaved profile draft");
    expect((await (await request.get("/api/snmp/definitions")).json()).data.generation).toBe(original.generation);
    await page.unroute("**/api/settings");
    await page.route("**/api/settings", (route) => route.fulfill({ json: { error: "Connection write failed" } }));
    await page.locator("#community").fill("retry-secret");
    await page.getByRole("button", { name: "Save Connection Settings", exact: true }).click();
    await expect(page.locator("#snmp-connection-error")).toContainText("Connection write failed");
    await expect(page.locator("#community")).toHaveValue("retry-secret");
    await expect(page.locator("#save-snmp-connection")).toBeEnabled();
    page.on("dialog", (dialog) => dialog.accept());
    await page.reload();
    await expect(page.locator("#template-profile")).toHaveValue("generic");
    await page.locator("#community").fill("not-for-toml");
    let payload: { definitions?: unknown; default_community?: unknown } = {};
    await page.route("**/api/snmp/definitions", async (route) => {
        payload = route.request().postDataJSON();
        await route.fulfill({ json: { ok: true, data: original } });
    });
    await root.getByRole("button", { name: "Save and apply", exact: true }).click();
    await expect(root).toContainText("Saved and applied");
    expect(payload.default_community).toBeUndefined();
    expect(JSON.stringify(payload)).not.toContain("not-for-toml");
    await expect(page.locator("#community")).toHaveValue("not-for-toml");
});

test("definition APIs reject invalid edits and stale generations, and roundtrip TOML", async ({ request }) => {
    const read = async () => (await (await request.get("/api/snmp/definitions")).json()).data;
    const original = await read();
    try {
        const definitions = structuredClone(original.definitions);
        definitions.common.cpu_oid = "1.3.6.1.4.1.65000.99.0";
        const save = await (await request.post("/api/snmp/definitions", { data: { generation: original.generation, definitions } })).json();
        expect(save.ok).toBe(true);
        expect(save.data.applied_generation).toBe(original.generation + 1);
        const stale = await (await request.post("/api/snmp/definitions", { data: { generation: original.generation, definitions } })).json();
        expect(stale.error).toContain("reload");
        definitions.common.cpu_oid = "MIB::symbol";
        const invalid = await (await request.post("/api/snmp/definitions", { data: { generation: save.data.generation, definitions } })).json();
        expect(invalid.error).toContain("invalid numeric OID");
        expect((await read()).generation).toBe(save.data.generation);
        const generic = original.profiles.find((profile: { id: string }) => profile.id === "generic");
        const exported = await (await request.post("/api/snmp/definitions/convert", { data: { document: generic.document } })).json();
        expect(exported.ok).toBe(true);
        const imported = await (await request.post("/api/snmp/definitions/convert", { data: { toml: exported.data.toml } })).json();
        expect(imported.data.document).toEqual(generic.document);
        const legacy = await (await request.post("/api/diagnostics/oids", { data: { cpu_oid_override: "1.3.6.1.2.1.1.3.0" } })).json();
        expect(legacy.error).toContain("SQLite");
    } finally {
        const current = await read();
        expect((await (await request.post("/api/snmp/definitions", { data: { generation: current.generation, definitions: original.definitions } })).json()).ok).toBe(true);
    }
});

test("visual editing adds and removes OIDs, applies common settings, previews, reloads and resets", async ({ page, request }) => {
    const original = (await (await request.get("/api/snmp/definitions")).json()).data;
    page.on("dialog", (dialog) => dialog.accept());
    try {
        await page.goto("/settings/snmp");
        const root = page.locator("#snmp-definitions");
        await expect(root.getByRole("heading", { name: "OID additions / SNMP monitoring definitions" })).toBeVisible();
        const profile = page.locator("#template-profile");
        await expect(profile).toHaveValue("generic");
        await editField(page, "profile", "profile.name", "GUI RFC 日本語");
        await page.locator("#template-tab-system").click();
        await root.getByRole("button", { name: "+ Add item", exact: true }).click();
        await page.getByRole("dialog").getByLabel("Item type", { exact: true }).selectOption({ label: "CPU utilization OID" });
        await page.getByRole("dialog").getByRole("button", { name: "Add", exact: true }).click();
        await page.getByRole("dialog").locator('[data-template-path="profile.cpu.candidates.1"]').fill("1.3.6.1.4.1.65000.3.0");
        await page.getByRole("dialog").getByRole("button", { name: "Apply to draft", exact: true }).click();
        await profile.selectOption("@common");
        if (await root.locator('[data-template-row="common.cpu_oid"]').count()) await editField(page, "common.cpu_oid", "common.cpu_oid", "1.3.6.1.4.1.65000.4.0");
        else {
            await root.getByRole("button", { name: "+ Add item", exact: true }).click();
            await page.getByRole("dialog").getByLabel("Item type", { exact: true }).selectOption({ label: "Additional CPU utilization OID" });
            await page.getByRole("dialog").getByRole("button", { name: "Add", exact: true }).click();
            await page.getByRole("dialog").locator('[data-template-path="common.cpu_oid"]').fill("1.3.6.1.4.1.65000.4.0");
            await page.getByRole("dialog").getByRole("button", { name: "Apply to draft", exact: true }).click();
        }
        await root.getByRole("button", { name: "Save and apply", exact: true }).click();
        await expect(root).toContainText("Saved and applied");
        await page.reload();
        await expect(profile).toHaveValue("generic");
        await expect(profile.locator("option:checked")).toContainText("GUI RFC 日本語");
        await expect(root.locator('[data-template-row="profile.cpu.candidates.1"]')).toContainText("1.3.6.1.4.1.65000.3.0");
        await profile.selectOption("@common");
        await expect(root.locator('[data-template-row="common.cpu_oid"]')).toContainText("1.3.6.1.4.1.65000.4.0");
        await createProfile(page, "1.3.6.1.4.1.65000.77");
        await editField(page, "profile", "profile.name", "Specific model");
        await root.locator("details").filter({ has: page.locator("#template-preview-oid") }).locator("summary").click();
        await root.locator("#template-preview-oid").fill("1.3.6.1.4.1.65000.77.1");
        await root.getByRole("button", { name: "Preview profile selection", exact: true }).click();
        await expect(root).toContainText("Preview profile selection: Specific model");
        await root.getByRole("button", { name: "Save and apply", exact: true }).click();
        await expect(root).toContainText("Saved and applied");
        await profile.selectOption("generic");
        await page.locator("#template-tab-system").click();
        await root.locator('[data-template-row="profile.cpu.candidates.1"]').getByRole("button", { name: "Remove", exact: true }).click();
        await expect(root.locator('[data-template-row="profile.cpu.candidates.1"]')).toHaveCount(0);
        await root.getByRole("button", { name: "Save and apply", exact: true }).click();
        await expect(root).toContainText("Saved and applied");
        await root.getByRole("button", { name: "Reset / remove user profile", exact: true }).click();
        await root.getByRole("button", { name: "Save and apply", exact: true }).click();
        await expect(profile.locator("option:checked")).toContainText("Generic RFC Network Device");
        await profile.selectOption("1.3.6.1.4.1.65000.77");
        await root.getByRole("button", { name: "Reset / remove user profile", exact: true }).click();
        await root.getByRole("button", { name: "Save and apply", exact: true }).click();
        await expect(profile.locator('option[value="1.3.6.1.4.1.65000.77"]')).toHaveCount(0);
        await expect(root).toContainText("Saved and applied");
    } finally {
        const current = (await (await request.get("/api/snmp/definitions")).json()).data;
        expect((await (await request.post("/api/snmp/definitions", { data: { generation: current.generation, definitions: original.definitions } })).json()).ok).toBe(true);
    }
});

test("creating a profile does not fork another vendor and concurrent saves retain edits", async ({ page, context, request }) => {
    const original = (await (await request.get("/api/snmp/definitions")).json()).data;
    page.on("dialog", (dialog) => dialog.accept());
    const other = await context.newPage();
    other.on("dialog", (dialog) => dialog.accept());
    try {
        await page.goto("/settings/snmp");
        const root = page.locator("#snmp-definitions");
        await page.locator("#template-profile").selectOption("1.3.6.1.4.1.9");
        await createProfile(page, "1.3.6.1.4.1.65002");
        await root.getByRole("button", { name: "Save and apply", exact: true }).click();
        await expect(root).toContainText("Saved and applied");
        const saved = (await (await request.get("/api/snmp/definitions")).json()).data;
        expect(Object.keys(saved.definitions.profiles).sort()).toEqual([...new Set([...Object.keys(original.definitions.profiles), "1.3.6.1.4.1.65002"])].sort());
        await other.goto("/settings/snmp");
        await expect(other.locator("#template-profile")).toHaveValue("generic");
        await editField(other, "profile", "profile.name", "Unsaved concurrent edit");
        await editField(page, "profile", "profile.name", "First writer");
        await root.getByRole("button", { name: "Save and apply", exact: true }).click();
        await expect(root).toContainText("Saved and applied");
        await other.getByRole("button", { name: "Save and apply", exact: true }).click();
        await expect(other.locator("#snmp-definitions").getByRole("alert")).toContainText("reload");
        await expect(other.locator("#template-profile option:checked")).toContainText("Unsaved concurrent edit");
        await expect(other.locator("#snmp-definitions")).toContainText("Unsaved changes");
    } finally {
        await other.close();
        const current = (await (await request.get("/api/snmp/definitions")).json()).data;
        expect((await (await request.post("/api/snmp/definitions", { data: { generation: current.generation, definitions: original.definitions } })).json()).ok).toBe(true);
    }
});

test("TOML import preserves vendor DOM and metrics and errors retain unsaved edits", async ({ page, request }) => {
    const original = (await (await request.get("/api/snmp/definitions")).json()).data;
    page.on("dialog", (dialog) => dialog.accept());
    try {
        await page.goto("/settings/snmp");
        const root = page.locator("#snmp-definitions");
        await expect(page.locator("#template-profile")).toHaveValue("generic");
        await root.locator("#template-import").setInputFiles({
            name: "custom.toml", mimeType: "application/toml",
            buffer: Buffer.from(`name = "Imported optical"
sysObjectID_prefix = "1.3.6.1.4.1.65001"
[[metrics]]
key = "uptime"
oid = "1.3.6.1.2.1.1.3.0"
unit = "ticks"
label = "Uptime"
data_type = "counter"
[tables.dom]
base_oid = "1.3.6.1.4.1.65001.1"
[[tables.dom.metrics]]
key = "rx_power"
oid = "1.3.6.1.4.1.65001.1.2"
unit = "dBm"
label = "Rx Power"
scale = 0.01
`),
        });

        await expect(page.locator("#template-profile")).toHaveValue("1.3.6.1.4.1.65001");
        const optical = await openRow(page, "profile.tables.dom.metrics.0");
        await expect(optical.locator('[data-template-path="profile.tables.dom.metrics.0.scale"]')).toHaveValue("0.01");
        await optical.getByRole("button", { name: "Cancel", exact: true }).click();
        await editField(page, "profile.metrics.0", "profile.metrics.0.oid", "bad.oid");
        await root.getByRole("button", { name: "Save and apply", exact: true }).click();
        await expect(root.getByRole("alert")).toContainText("invalid");
        await expect(page.locator("#template-profile option:checked")).toContainText("Imported optical");
        await editField(page, "profile.metrics.0", "profile.metrics.0.oid", "1.3.6.1.2.1.1.3.0");
        await root.getByRole("button", { name: "Save and apply", exact: true }).click();
        await expect(root).toContainText("Saved and applied");
        const downloadPromise = page.waitForEvent("download");
        await root.getByRole("button", { name: "Export TOML", exact: true }).click();
        const download = await downloadPromise;
        expect(download.suggestedFilename()).toBe("1-3-6-1-4-1-65001.toml");
        await page.locator("#lang-select").selectOption("ja");
        await expect(root.getByRole("heading")).toHaveText("OID追加・SNMP監視定義");
        await page.setViewportSize({ width: 375, height: 812 });
        await page.locator("#theme-select").selectOption("light");
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    } finally {
        const current = (await (await request.get("/api/snmp/definitions")).json()).data;
        expect((await (await request.post("/api/snmp/definitions", { data: { generation: current.generation, definitions: original.definitions } })).json()).ok).toBe(true);
    }
});
