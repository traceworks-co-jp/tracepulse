import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
        localStorage.setItem("tracepulse-onboarding-v1", "done");
        localStorage.setItem("tracepulse-lang", "en");
    });
});

test("important port toggles persist, distinguish shutdown and recover from failed saves in both languages", async ({ page, request }) => {
    const registration = await request.post("/api/discovery/register", {
        data: [{ ip: "192.0.2.1", name: "fixture-switch", community: "fixture" }],
    });
    expect(registration.ok()).toBeTruthy();
    const registered = (await (await request.get("/api/devices")).json()).find((device: { ip: string }) => device.ip === "192.0.2.1");
    const id = registered.id as number;
    const port = { link_down_alert_enabled: false, topology_link: false,
        if_admin_status: 1, if_oper_status: 2, active: false, observed_at: 1000 };
    const detail = () => ({ id, ip: "192.0.2.1", name: "fixture-switch", status: "online",
        interfaces: [
            { if_index: 7, if_name: "Gi7", link_status: "down", metrics: {}, port_alert: { ...port } },
            { if_index: 8, if_name: "Gi8", link_status: "down", metrics: {},
                port_alert: { ...port, link_down_alert_enabled: true, if_admin_status: 2 } },
            { if_index: 9, if_name: "Gi9", link_status: "down", metrics: {},
                port_alert: { ...port, topology_link: true } },
        ] });
    await page.route("**/api/devices", route => route.fulfill({ json: [] }));
    await page.route("**/api/device/192.0.2.1", route => route.fulfill({ json: detail() }));
    await page.route("**/api/device/192.0.2.1/history?*", route => route.fulfill({ json: {} }));
    let fail = false;
    await page.route(`**/api/devices/${id}/interfaces/7/alert`, async route => {
        expect(route.request().method()).toBe("PUT");
        if (fail) return await route.fulfill({ status: 503, json: { error: "fixture storage unavailable" } });
        port.link_down_alert_enabled = route.request().postDataJSON().link_down_alert_enabled;
        await route.fulfill({ json: port });
    });
    await page.goto("/device/192.0.2.1");
    const toggle = page.locator("[data-if-index='7']");
    await expect(toggle).toHaveAttribute("aria-pressed", "false");
    await expect(toggle.locator("..").locator("..")).not.toHaveClass(/row-crit/);
    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-pressed", "true");
    await expect(toggle.locator("..").locator("..")).toHaveClass(/row-crit/);
    await expect(page.locator("[data-if-index='8']").locator("..").locator("..")).not.toHaveClass(/row-crit/);
    await expect(page.locator("[data-if-index='9']").locator("..").locator("..")).toHaveClass(/row-crit/);
    await page.reload();
    await expect(toggle).toHaveAttribute("aria-pressed", "true");
    fail = true;
    await toggle.click();
    await expect(page.locator("#port-alert-error")).toContainText("fixture storage unavailable");
    await expect(toggle).toBeEnabled();
    await expect(toggle).toHaveAttribute("aria-pressed", "true");
    await page.evaluate(() => window.setLanguage?.("ja"));
    await expect(page.locator("[data-port-alert-heading]")).toHaveText("重要ポート監視");
    fail = false;
    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-pressed", "false");
    await page.evaluate(() => {
        document.body.dataset.role = "viewer";
        window.dispatchEvent(new Event("tracepulse-role-change"));
    });
    await expect(toggle).toBeDisabled();
    await request.delete("/api/device/192.0.2.1");
});

test("Community settings expose saved notification channels and global/device maintenance controls", async ({ page }) => {
    await page.route("**/api/devices", route => route.fulfill({ json: [{ id: 1, name: "fixture-switch", ip: "192.0.2.1" }] }));
    const status = { server_time: 1000, global: null as object | null, devices: {} as Record<string, object> };
    let saved: unknown;
    await page.route("**/api/notifications", async route => {
        if (route.request().method() === "POST") {
            saved = route.request().postDataJSON();
            return await route.fulfill({ json: { ok: true } });
        }
        await route.fulfill({ json: { slack: {}, teams: {}, webhook: { enabled: true, webhook_url: "http://127.0.0.1:19090/fixture" },
            flap_guard: {}, retry: {} } });
    });
    let change: { device_id: number | null; enabled: boolean; duration: number | null; reason: string } | undefined;
    await page.route("**/api/maintenance", async route => {
        if (route.request().method() === "POST") {
            change = route.request().postDataJSON();
            const state = { enabled: change!.enabled, muted_until: 4600, reason: "fixture" };
            if (change!.device_id === null) status.global = state;
            else status.devices[String(change!.device_id)] = state;
        }
        await route.fulfill({ json: status });
    });
    await page.goto("/settings");
    await expect(page.locator("#webhook_enabled")).toBeChecked();
    await page.locator("#webhook_url").fill("http://127.0.0.1:19090/updated");
    await page.evaluate(() => window.saveNotifications?.());
    await expect.poll(() => saved).toMatchObject({ webhook: { enabled: true, webhook_url: "http://127.0.0.1:19090/updated" } });
    const controls = page.locator("#core-maintenance");
    await expect(controls.locator(".maintenance-status-badge")).toHaveText("Not muted");
    await expect(controls.getByLabel("Scope")).toBeVisible();
    await expect(controls.getByLabel("Duration")).toBeVisible();
    await expect(controls.getByLabel("Reason")).toHaveAttribute("placeholder", "e.g., Planned switch firmware upgrade");
    await expect(controls.getByRole("button", { name: "Mute", exact: true })).toHaveClass(/btn-primary/);
    await expect(controls.getByRole("button", { name: "Unmute", exact: true })).toHaveClass(/btn-secondary/);
    const formGrid = controls.locator(".maintenance-form-grid");
    await expect(formGrid).toHaveCSS("display", "grid");
    const scopeBox = await controls.getByLabel("Scope").boundingBox();
    const durationBox = await controls.getByLabel("Duration").boundingBox();
    expect(scopeBox).not.toBeNull();
    expect(durationBox).not.toBeNull();
    expect(durationBox!.x).toBeGreaterThan(scopeBox!.x);
    await page.setViewportSize({ width: 390, height: 844 });
    const mobileScopeBox = await controls.getByLabel("Scope").boundingBox();
    const mobileDurationBox = await controls.getByLabel("Duration").boundingBox();
    expect(mobileScopeBox).not.toBeNull();
    expect(mobileDurationBox).not.toBeNull();
    expect(mobileDurationBox!.y).toBeGreaterThan(mobileScopeBox!.y);
    await page.setViewportSize({ width: 1280, height: 720 });
    await controls.getByLabel("Reason").fill("planned switch firmware upgrade");
    await controls.getByRole("button", { name: "Mute", exact: true }).click();
    await expect(controls).toContainText("Muted: collection and recording continue.");
    await expect(controls.locator(".maintenance-status-badge")).toHaveText("Muted");
    await expect(controls.locator(".maintenance-status-details")).toContainText("fixture");
    expect(change?.duration).toBe(3600);
    expect(change?.reason).toBe("planned switch firmware upgrade");
    await page.evaluate(() => document.documentElement.setAttribute("data-theme", "light"));
    await expect(controls.locator(".maintenance-status-badge")).toHaveCSS("color", "rgb(146, 64, 14)");
    await page.evaluate(() => document.documentElement.setAttribute("data-theme", "dark"));
    await expect(controls.locator(".maintenance-status-badge")).toHaveCSS("color", "rgb(252, 211, 77)");
    expect(change?.device_id).toBeNull();
    await controls.locator("select").first().selectOption("1");
    await controls.getByRole("button", { name: "Mute", exact: true }).click();
    await expect.poll(() => change?.device_id).toBe(1);
    await controls.getByRole("button", { name: "Unmute", exact: true }).click();
    await expect.poll(() => change?.enabled).toBe(false);
    await page.evaluate(() => window.setLanguage?.("ja"));
    await expect(controls.getByRole("heading")).toHaveText("アラート静観");
    await expect(controls.getByLabel("期間")).toBeVisible();
    await expect(controls.getByLabel("理由")).toHaveAttribute("placeholder", "例：スイッチのファームウェア更新");
});

test("Community alert settings and maintenance APIs persist and reject cross-origin mutations", async ({ request, baseURL }) => {
    const origin = baseURL!;
    const forbidden = await request.post("/api/maintenance", {
        headers: { Origin: "http://untrusted.invalid" }, data: { enabled: true },
    });
    expect(forbidden.status()).toBe(403);
    const settings = await (await request.get("/api/notifications")).json();
    const saved = await request.post("/api/notifications", { headers: { Origin: origin }, data: settings });
    expect(saved.ok()).toBeTruthy();
    const muted = await request.post("/api/maintenance", {
        headers: { Origin: origin }, data: { enabled: true, duration: 3600, reason: "isolated browser fixture" },
    });
    expect(muted.ok()).toBeTruthy();
    expect((await muted.json()).global.enabled).toBe(true);
    expect((await (await request.get("/api/maintenance")).json()).global.enabled).toBe(true);
    const released = await request.post("/api/maintenance", {
        headers: { Origin: origin }, data: { enabled: false },
    });
    expect(released.ok()).toBeTruthy();
    expect((await released.json()).global.enabled).toBe(false);
});
