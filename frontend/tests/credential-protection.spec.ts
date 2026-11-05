import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
        localStorage.setItem("tracepulse-onboarding-v1", "done");
        localStorage.setItem("tracepulse-lang", "en");
    });
});

for (const path of ["/settings", "/settings/snmp"]) {
    test(`SNMP credential translations follow initial and changed language on ${path}`, async ({ page }) => {
        await page.addInitScript(() => localStorage.setItem("tracepulse-lang", "ja"));
        await page.route("**/static/js/*.js*", async route => {
            const filename = new URL(route.request().url()).pathname.split("/").pop()!;
            await route.fulfill({ path: `frontend/dist/${filename}`, contentType: "application/javascript" });
        });
        await page.goto(path);
        if (path === "/settings") {
            await expect(page.locator("#snmp-connection-form")).toHaveCount(0);
            await expect(page.locator("#community")).toHaveCount(0);
            await page.getByRole("link", { name: "SNMP設定を管理", exact: true }).click();
            await expect(page).toHaveURL(/\/settings\/snmp$/);
        }
        const form = page.locator("#snmp-connection-form");
        await expect(form.locator("legend")).toHaveText("SNMP認証情報");
        await expect(form.locator("#global-snmp-hint")).toHaveText("空欄の場合は保存済みの共通設定を維持します。");
        for (const [field, label] of [
            ["version", "SNMPバージョン"], ["v3_username", "v3 ユーザー名"],
            ["v3_auth_protocol", "v3 認証方式"], ["v3_auth_password", "v3 認証パスワード"],
            ["v3_priv_protocol", "v3 暗号化方式"], ["v3_priv_password", "v3 暗号化パスワード"],
        ]) {
            await expect(form.locator(`label[for=global-snmp-${field}] > span`)).toHaveText(label);
        }
        await expect(form.locator("#global-snmp-version option:checked")).toHaveText("変更なし");
        await expect(form.locator('#global-snmp-v3_auth_protocol option[value="sha256"]')).toHaveText("SHA-256");
        await expect(form.locator('#global-snmp-v3_priv_protocol option[value="aes128"]')).toHaveText("AES-128");
        await expect(form.locator('#global-snmp-v3_auth_protocol option[value="none"]')).toHaveText("なし");
        const password = form.locator("#global-snmp-v3_auth_password");
        await expect(password).toHaveAttribute("placeholder", "空欄の場合は保存済みの共通設定を維持します。");
        await password.fill("unsaved-language-test");
        await form.locator("#global-snmp-version").selectOption("3");
        await page.locator("#lang-select").selectOption("en");
        await expect(form.locator("legend")).toHaveText("SNMP credentials");
        await expect(password).toHaveAttribute("placeholder", "Leave blank to keep existing defaults.");
        await expect(form.locator('#global-snmp-v3_auth_protocol option[value="none"]')).toHaveText("None");
        await page.locator("#lang-select").selectOption("ja");
        await expect(form.locator("legend")).toHaveText("SNMP認証情報");
        await expect(password).toHaveValue("unsaved-language-test");
        await expect(password).toHaveAttribute("type", "password");
        await expect(form.locator("#global-snmp-version")).toHaveValue("3");
        await page.reload();
        await expect(form.locator("legend")).toHaveText("SNMP認証情報");
        await expect(password).toHaveValue("");
        if (path === "/settings/snmp") {
            await page.setViewportSize({ width: 375, height: 812 });
            expect(await form.evaluate(element => element.scrollWidth <= element.clientWidth)).toBeTruthy();
            await page.screenshot({ path: "test-results/snmp-credentials-ja-mobile.png", fullPage: true });
        }
    });
}

test("manual v3 registration keeps secrets out of responses and renders inherited fields", async ({ page, request }) => {
    const ip = "192.0.2.239";
    const secrets = ["manual-v3-user-marker", "manual-v3-auth-marker", "manual-v3-privacy-marker"];
    try {
        await page.goto("/discovery");
        await page.getByRole("link", { name: "+ Add manually", exact: true }).click();
        await page.locator("#manual-ip").fill(ip);
        await expect(page.locator("#manual-community")).toHaveValue("");
        await page.locator("#manual-snmp-version").selectOption("3");
        await page.locator("#manual-snmp-v3_username").fill(secrets[0]);
        await page.locator("#manual-snmp-v3_auth_password").fill(secrets[1]);
        await page.locator("#manual-snmp-v3_priv_password").fill(secrets[2]);
        await page.locator("#manual-box").getByRole("button", { name: "Add", exact: true }).click();
        await expect(page.locator("#manual-result")).toContainText("registered successfully");
        for (const field of ["v3_username", "v3_auth_password", "v3_priv_password"]) {
            await expect(page.locator(`#manual-snmp-${field}`)).toHaveValue("");
            await expect(page.locator(`#manual-snmp-${field}`)).toHaveAttribute("type", "password");
        }
        for (const endpoint of ["/api/devices", "/api/settings", "/", "/settings", "/settings/snmp"]) {
            const response = await request.get(endpoint);
            const text = await response.text();
            for (const secret of secrets) expect(text).not.toContain(secret);
        }
        const device = (await (await request.get("/api/devices")).json()).find((item: { ip: string }) => item.ip === ip);
        expect(device.snmp_individual_configured).toBe(true);
    } finally { await request.delete(`/api/device/${ip}`); }
});

test("bulk credentials replace selected overrides and can return to global defaults on mobile", async ({ page, request }) => {
    const ips = ["192.0.2.237", "192.0.2.238"];
    try {
        await request.post("/api/discovery/register", { data: ips.map(ip => ({ ip, community: null })) });
        await page.goto("/");
        for (const ip of ips) await page.getByRole("checkbox", { name: `Select device ${ip}`, exact: true }).check();
        await page.locator("#bulk-rename").click();
        await page.getByRole("button", { name: "SNMP credentials", exact: true }).click();
        const dialog = page.locator("#device-snmp-dialog");
        await expect(dialog).toContainText("Blank fields inherit global defaults");
        await dialog.locator("#device-snmp-community").fill("bulk-community-marker");
        await dialog.getByRole("button", { name: "Save", exact: true }).click();
        await expect(dialog).not.toBeVisible();
        let devices = await (await request.get("/api/devices")).json();
        for (const ip of ips) expect(devices.find((device: { ip: string }) => device.ip === ip).community_configured).toBe(true);
        expect(JSON.stringify(devices)).not.toContain("bulk-community-marker");
        await page.setViewportSize({ width: 375, height: 667 });
        await page.locator("#bulk-rename").click();
        await page.getByRole("button", { name: "SNMP credentials", exact: true }).click();
        await expect(dialog.locator("#device-snmp-community")).toHaveValue("");
        expect(await dialog.evaluate(element => element.scrollWidth <= element.clientWidth)).toBeTruthy();
        await page.screenshot({ path: "test-results/snmp-credentials-mobile.png", fullPage: true });
        page.once("dialog", confirmation => confirmation.accept());
        await dialog.getByRole("button", { name: "Use global defaults", exact: true }).click();
        await expect(dialog).not.toBeVisible();
        devices = await (await request.get("/api/devices")).json();
        for (const ip of ips) expect(devices.find((device: { ip: string }) => device.ip === ip).snmp_individual_configured).toBe(false);
    } finally { for (const ip of ips) await request.delete(`/api/device/${ip}`); }
});

for (const [width, theme, language] of [[1280, "light", "en"], [375, "dark", "ja"]] as const) {
    test(`SNMP credential save and reset show elapsed progress at ${width} ${theme} ${language}`, async ({ page }) => {
        await page.setViewportSize({ width, height: 900 });
        const devices = Array.from({ length: 50 }, (_, index) => ({ id: index + 1, ip: `192.0.2.${index + 1}`, name: `save-progress-${index + 1}`, status: "online" }));
        await page.route("**/api/devices", route => route.fulfill({ json: devices }));
        let finish: ((success: boolean) => void) | undefined;
        const requests: Array<{ credentials: Record<string, string>; device_ids: number[] }> = [];
        await page.route("**/api/devices/snmp-credentials", async route => {
            requests.push(route.request().postDataJSON());
            const success = await new Promise<boolean>(resolve => { finish = resolve; });
            await route.fulfill(success ? { json: { ok: true, devices: [] } } : { status: 500, json: { error: "Save failed" } });
        });
        await page.goto("/");
        await page.evaluate(() => window.refreshDashboard?.());
        await expect(page.locator("#dash-tbody tr")).toHaveCount(50);
        await page.evaluate(lang => window.setLanguage?.(lang), language);
        await page.locator("#theme-select").selectOption(theme);
        await page.locator("#select-visible-devices").check();
        const open = async () => {
            await page.locator("#bulk-rename").click();
            await page.locator("#bulk-actions-dialog [data-action=snmp]").click();
        };
        await open();
        const dialog = page.locator("#device-snmp-dialog");
        const progress = dialog.locator(".operation-progress");
        await dialog.locator("#device-snmp-community").fill("save-progress-secret");
        await dialog.locator("[type=submit]").click();
        await expect(progress).toBeInViewport();
        await expect(progress.getByRole("status")).toContainText(language === "ja" ? "50台のSNMP認証情報を保存しています" : "Saving SNMP credentials for 50 devices");
        await expect(progress.getByRole("status")).toHaveText(language === "ja" ? /経過 [1-9]\d*秒/ : /[1-9]\d*s elapsed/);
        await expect(progress).not.toContainText("save-progress-secret");
        await expect(dialog.locator("[type=submit]")).toBeDisabled();
        await expect(dialog.locator("[data-reset]")).toBeDisabled();
        await page.keyboard.press("Escape");
        await expect(dialog).toBeVisible();
        await page.evaluate(() => document.querySelector<HTMLFormElement>("#device-snmp-dialog form")!.requestSubmit());
        expect(requests).toHaveLength(1);
        expect(requests[0].device_ids).toHaveLength(50);
        await dialog.screenshot({ path: `test-results/snmp-save-progress-${width}-${theme}.png` });
        finish!(false);
        await expect(dialog.getByRole("alert")).toContainText("Save failed");
        await expect(progress).toHaveCount(0);
        await expect(dialog.locator("[type=submit]")).toBeEnabled();
        await dialog.locator("[type=submit]").click();
        await expect.poll(() => requests.length).toBe(2);
        finish!(true);
        await expect(dialog).not.toBeVisible();
        await expect(progress).toHaveCount(0);
        await open();
        page.once("dialog", confirmation => confirmation.accept());
        await dialog.locator("[data-reset]").click();
        await expect(progress).toBeVisible();
        await expect(progress.getByRole("status")).toContainText(language === "ja" ? "50台の認証情報を共通設定へ戻しています" : "Restoring global defaults for 50 devices");
        await expect.poll(() => requests.length).toBe(3);
        expect(requests[2].credentials).toEqual({});
        finish!(true);
        await expect(dialog).not.toBeVisible();
        await expect(progress).toHaveCount(0);
    });
}

test("individual SNMP editor loads saved settings, retains masked secrets and saves reset", async ({ page, request }) => {
    const ip = "192.0.2.236";
    try {
        await request.post("/api/discovery/register", { data: [{ ip, community: "legacy-credential" }] });
        const device = (await (await request.get("/api/devices")).json()).find((item: { ip: string }) => item.ip === ip);
        const credentials = { version: "3", community: "device-community-secret", v3_username: "device-monitor", v3_auth_protocol: "sha256", v3_auth_password: "device-auth-secret", v3_priv_protocol: "aes128", v3_priv_password: "device-privacy-secret" };
        expect((await request.post("/api/devices/snmp-credentials", { data: { device_ids: [device.id], credentials } })).ok()).toBeTruthy();
        await page.goto("/");
        await page.getByRole("checkbox", { name: `Select device ${ip}`, exact: true }).check();
        const open = async () => {
            await page.locator("#bulk-rename").click();
            await page.getByRole("button", { name: "SNMP credentials", exact: true }).click();
            await expect(page.locator("#device-snmp-dialog [type=submit]")).toBeEnabled();
        };
        await open();
        const dialog = page.locator("#device-snmp-dialog");
        await expect(dialog.locator("#device-snmp-version")).toHaveValue("3");
        await expect(dialog.locator("#device-snmp-v3_username")).toHaveValue("device-monitor");
        await expect(dialog.locator("#device-snmp-v3_auth_protocol")).toHaveValue("sha256");
        for (const field of ["community", "v3_auth_password", "v3_priv_password"]) {
            await expect(dialog.locator(`#device-snmp-${field}`)).toHaveValue("");
            await expect(dialog.locator(`#device-snmp-${field}`)).toHaveAttribute("placeholder", "Configured (enter only to replace)");
        }
        const response = await request.get(`/api/device/${ip}/snmp-credentials`);
        const body = await response.text();
        for (const secret of [credentials.community, credentials.v3_auth_password, credentials.v3_priv_password]) expect(body).not.toContain(secret);
        await dialog.locator("#device-snmp-v3_username").fill("updated-monitor");
        await dialog.getByRole("button", { name: "Save", exact: true }).click();
        await expect(dialog).not.toBeVisible();
        await open();
        await expect(dialog.locator("#device-snmp-v3_username")).toHaveValue("updated-monitor");
        await expect(dialog.locator("#device-snmp-v3_auth_password")).toHaveAttribute("placeholder", "Configured (enter only to replace)");
        await page.route("**/api/devices/snmp-credentials", route => route.fulfill({ status: 500, json: { error: "Reset failed" } }));
        page.once("dialog", confirmation => confirmation.accept());
        await dialog.getByRole("button", { name: "Use global defaults", exact: true }).click();
        await expect(dialog.locator("[role=alert]")).toContainText("Reset failed");
        await expect(dialog.getByRole("button", { name: "Use global defaults", exact: true })).toBeEnabled();
        await page.unroute("**/api/devices/snmp-credentials");
        page.once("dialog", confirmation => confirmation.accept());
        await dialog.getByRole("button", { name: "Use global defaults", exact: true }).click();
        await expect(dialog).not.toBeVisible();
        await open();
        await expect(dialog.locator("[role=status]")).toContainText("global defaults");
        await expect(dialog.locator("#device-snmp-version")).toHaveValue("");
        await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
    } finally { await request.delete(`/api/device/${ip}`); }
});

test("global v3 settings save encrypted values without returning them to the form", async ({ page, request }) => {
    try {
        await page.goto("/settings/snmp");
        await page.locator("#global-snmp-version").selectOption("3");
        await page.locator("#global-snmp-v3_username").fill("global-v3-user-marker");
        await page.locator("#global-snmp-v3_auth_password").fill("global-v3-auth-marker");
        await page.locator("#global-snmp-v3_priv_password").fill("global-v3-privacy-marker");
        await page.locator("#save-snmp-connection").click();
        await expect(page.locator("#snmp-connection-status")).not.toBeEmpty();
        await expect(page.locator("#global-snmp-v3_auth_password")).toHaveValue("");
        await page.reload();
        await expect(page.locator("#global-snmp-v3_priv_password")).toHaveValue("");
        const settings = await (await request.get("/api/settings")).json();
        expect(settings.snmp.version).toBe("3");
        expect(settings.snmp.v3_configured).toBe(true);
        expect(JSON.stringify(settings)).not.toContain("global-v3-auth-marker");
        expect(await page.content()).not.toContain("global-v3-privacy-marker");
        await page.screenshot({ path: "test-results/snmp-credentials-settings.png", fullPage: true });
    } finally { await request.post("/api/settings", { data: { snmp_credentials: { version: "2c" } } }); }
});

test("device APIs and rendered pages never expose stored community values", async ({ page, request }) => {
    const ip = "192.0.2.241";
    const secret = "browser-device-secret-291735";
    try {
        expect((await request.post("/api/discovery/register", {
            data: [{ ip, name: "secret-protection-test", community: secret }],
        })).ok()).toBeTruthy();
        for (const endpoint of ["/api/devices", "/api/settings", "/settings", "/"]) {
            const response = await request.get(endpoint);
            expect(await response.text()).not.toContain(secret);
        }
        const devices = await (await request.get("/api/devices")).json();
        const device = devices.find((item: { ip: string }) => item.ip === ip);
        expect(device.community).toBe("********");
        expect(device.community_configured).toBe(true);
        await page.goto("/");
        await expect(page.locator("#dash-tbody")).toContainText("secret-protection-test");
        await expect(page.locator("#th-community, #sort-community")).toHaveCount(0);
        await expect(page.locator("#dash-table thead th")).toHaveCount(9);
        await expect(page.locator("#dash-tbody tr").filter({ hasText: ip }).locator("td")).toHaveCount(9);
        await expect(page.locator("#dash-tbody")).not.toContainText("********");
        await page.evaluate(() => window.refreshDashboard?.());
        await expect(page.locator("#dash-tbody")).not.toContainText("********");
        expect(await page.content()).not.toContain(secret);
        await page.goto("/settings");
        await expect(page.locator("#community")).toHaveCount(0);
        await expect(page.locator("#snmp-connection-form")).toHaveCount(0);
        await page.getByRole("link", { name: "Manage SNMP Settings", exact: true }).click();
        await expect(page.locator("#community")).toHaveAttribute("type", "password");
        await expect(page.locator("#community")).toHaveValue("");
        await page.locator("#community").fill("replacement-browser-secret");
        await page.getByRole("button", { name: "Save Connection Settings", exact: true }).click();
        await expect(page.locator("#snmp-connection-status")).not.toBeEmpty();
        await expect(page.locator("#community")).toHaveValue("");
        const settings = await (await request.get("/api/settings")).json();
        expect(settings.snmp.default_community).toBe("");
        expect(settings.snmp.community_configured).toBe(true);
        await page.reload();
        await expect(page.locator("#community")).toHaveValue("");
        await page.getByRole("button", { name: "Save Connection Settings", exact: true }).click();
        await expect(page.locator("#snmp-connection-status")).not.toBeEmpty();
        await expect(page.locator("#community")).toHaveValue("");
    } finally {
        await request.delete(`/api/device/${ip}`);
    }
});

test("SNMP diagnostics sends credentials in a POST body, not the URL", async ({ page }) => {
    let method = "";
    let url = "";
    let body = "";
    await page.goto("/diagnostics");
    await page.route("**/diagnostics", async (route) => {
        method = route.request().method();
        url = route.request().url();
        body = route.request().postData() || "";
        await route.fulfill({ contentType: "text/html", body: "<p id='diagnostic-post-done'>Done</p>" });
    });
    await page.locator("input[name='ip']").fill("192.0.2.1");
    await page.locator("input[name='community']").fill("diagnostic-private-value");
    await page.locator("#diag-run-btn").click();
    await expect(page.locator("#diagnostic-post-done")).toBeVisible();
    expect(method).toBe("POST");
    expect(url).not.toContain("diagnostic-private-value");
    expect(new URLSearchParams(body).get("community")).toBe("diagnostic-private-value");
    expect(page.url()).not.toContain("diagnostic-private-value");
});

test("discovery registration retains the typed secret without storing it in result attributes", async ({ page }) => {
    let community = "";
    await page.route("**/api/discovery/scan", (route) => route.fulfill({ json: { job_id: "fake", total: 1 } }));
    await page.route("**/api/discovery/scan/fake", (route) => route.fulfill({
        json: { status: "done", scanned: 1, total: 1, devices: [{ ip: "192.0.2.240", name: "found-switch", status: "online", community_configured: true }] },
    }));
    await page.route("**/api/discovery/register", async (route) => {
        community = route.request().postDataJSON()[0].community;
        await route.fulfill({ json: { registered: ["192.0.2.240"], skipped: [], errors: [] } });
    });
    await page.goto("/discovery");
    await page.locator("#cidr").fill("192.0.2.240/32");
    await page.locator("#community").fill("typed-scan-secret");
    await page.locator("#scan-btn").click();
    await expect(page.locator("#results-body")).toContainText("found-switch");
    expect(await page.locator("#results-body").innerHTML()).not.toContain("typed-scan-secret");
    await page.locator(".row-cb").check();
    await page.locator("#register-btn").click();
    await expect.poll(() => community).toBe("typed-scan-secret");
});

test("discovery community eye buttons preserve values and support keyboard, language, and narrow screens", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto("/discovery");
    await page.locator("#community").fill("typed-scan-secret");
    const toggle = page.locator("button[aria-controls=community]");
    await expect(toggle).toHaveAccessibleName("Show community");
    await expect(page.locator("#community")).toHaveAttribute("type", "password");
    await expect(toggle).toHaveAttribute("aria-pressed", "false");
    const inputBounds = (await page.locator("#community").boundingBox())!;
    const buttonBounds = (await toggle.boundingBox())!;
    expect(buttonBounds.x).toBeGreaterThan(inputBounds.x + inputBounds.width / 2);
    expect(buttonBounds.x + buttonBounds.width).toBeLessThanOrEqual(inputBounds.x + inputBounds.width);
    await toggle.click();
    await expect(page.locator("#community")).toHaveAttribute("type", "text");
    await expect(toggle).toHaveAccessibleName("Hide community");
    await expect(toggle).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator("#community")).toHaveValue("typed-scan-secret");
    await toggle.focus();
    await page.keyboard.press("Space");
    await expect(page.locator("#community")).toHaveAttribute("type", "password");
    await page.evaluate(() => window.setLanguage?.("ja"));
    await expect(toggle).toHaveAccessibleName("コミュニティを表示");
    await toggle.click();
    await expect(toggle).toHaveAccessibleName("コミュニティを隠す");
    await page.locator("a[data-i18n=add_manually]").click();
    const manual = page.locator("#manual-community");
    await manual.fill("manual-secret");
    const manualToggle = page.locator("button[aria-controls=manual-community]");
    await expect(manualToggle).toHaveAccessibleName("コミュニティを表示");
    await manualToggle.click();
    await expect(manual).toHaveAttribute("type", "text");
    await expect(manual).toHaveValue("manual-secret");
    await manualToggle.click();
    await expect(manual).toHaveAttribute("type", "password");
    await page.reload();
    await expect(page.locator("#community")).toHaveAttribute("type", "password");
});

test("setup guide community visibility toggles without submitting discovery", async ({ page }) => {
    let scans = 0;
    await page.route("**/api/discovery/scan", async (route) => {
        scans++;
        await route.fulfill({ json: { error: "Unexpected submission" } });
    });
    await page.goto("/settings");
    await page.getByRole("button", { name: "Setup Guide", exact: true }).click();
    const guide = page.locator("#setup-guide");
    await guide.getByRole("button", { name: "Next", exact: true }).click();
    const input = guide.locator("#setup-community");
    await guide.locator("#setup-cidr").fill("192.0.2.1/32");
    await input.fill("setup-secret");
    await expect(input).toHaveAttribute("type", "password");
    await guide.getByRole("button", { name: "Show community", exact: true }).click();
    await expect(input).toHaveAttribute("type", "text");
    await expect(input).toHaveValue("setup-secret");
    await guide.getByRole("button", { name: "Hide community", exact: true }).click();
    await expect(input).toHaveAttribute("type", "password");
    expect(scans).toBe(0);
    await guide.getByRole("button", { name: "Back", exact: true }).click();
    await guide.getByRole("button", { name: "Next", exact: true }).click();
    await expect(input).toHaveAttribute("type", "password");
});
