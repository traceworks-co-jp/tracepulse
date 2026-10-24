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
        await expect(page.locator("#dash-tbody")).toContainText("********");
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
