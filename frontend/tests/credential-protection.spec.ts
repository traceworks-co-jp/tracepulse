import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
        localStorage.setItem("tracepulse-onboarding-v1", "done");
        localStorage.setItem("tracepulse-lang", "en");
    });
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
        await expect(page.locator("#community")).toHaveAttribute("type", "password");
        await expect(page.locator("#community")).toHaveValue("");
        await page.locator("#community").fill("replacement-browser-secret");
        await page.getByRole("button", { name: "Save Settings", exact: true }).click();
        await expect(page.locator("#toast-ok")).toBeVisible();
        await expect(page.locator("#community")).toHaveValue("");
        const settings = await (await request.get("/api/settings")).json();
        expect(settings.snmp.default_community).toBe("");
        expect(settings.snmp.community_configured).toBe(true);
        await page.reload();
        await expect(page.locator("#community")).toHaveValue("");
        await page.getByRole("button", { name: "Save Settings", exact: true }).click();
        await expect(page.locator("#toast-ok")).toBeVisible();
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
