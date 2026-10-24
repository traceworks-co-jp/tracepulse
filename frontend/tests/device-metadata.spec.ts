import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
        localStorage.setItem("tracepulse-onboarding-v1", "done");
        localStorage.setItem("tracepulse-lang", "en");
    });
});

test("Japanese bulk naming previews numeric IP order and persists through reload", async ({ page, request }) => {
    const ips = ["192.0.2.210", "192.0.2.202"];
    try {
        const registration = await request.post("/api/discovery/register", {
            data: ips.map((ip) => ({ ip, name: `sys-${ip}`, community: "public" })),
        });
        expect(registration.ok()).toBeTruthy();
        await page.goto("/");
        await page.evaluate(() => window.refreshDashboard?.());
        for (const ip of ips) await page.getByRole("checkbox", { name: `Select device ${ip}`, exact: true }).check();
        await page.locator("#bulk-rename").click();
        const dialog = page.locator("#bulk-rename-dialog");
        await dialog.getByLabel("Prefix", { exact: true }).fill("大阪支店SW-");
        await dialog.getByRole("button", { name: "Preview", exact: true }).click();
        await expect(dialog.locator("#rename-preview li")).toHaveText([
            "192.0.2.202 → 大阪支店SW-01", "192.0.2.210 → 大阪支店SW-02",
        ]);
        await dialog.getByLabel("Suffix", { exact: true }).fill("-本館");
        await expect(dialog.getByRole("button", { name: "Apply names" })).toBeDisabled();
        await expect(dialog.locator("#rename-preview li")).toHaveCount(0);
        await dialog.getByRole("button", { name: "Preview", exact: true }).click();
        await expect(dialog.locator("#rename-preview li").first()).toHaveText("192.0.2.202 → 大阪支店SW-01-本館");
        await dialog.getByRole("button", { name: "Apply names" }).click();
        await expect(dialog).not.toBeVisible();
        await expect(page.locator("#dash-tbody")).toContainText("大阪支店SW-01-本館");
        await page.reload();
        await page.evaluate(() => window.refreshDashboard?.());
        await expect(page.locator("#device-search")).toHaveCount(1);
        await expect(page.locator("#dash-table thead th")).toHaveCount(8);
        await expect(page.locator("#dash-tbody")).toContainText("大阪支店SW-02-本館");
        const response = await request.get("/api/devices");
        const devices = await response.json();
        const device = devices.find((item: { ip: string }) => item.ip === ips[1]);
        expect(device.name).toBe(`sys-${ips[1]}`);
        expect(device.display_name).toBe("大阪支店SW-01-本館");
    } finally {
        for (const ip of ips) await request.delete(`/api/device/${ip}`);
    }
});

test("incremental search covers alias, sysName, IP and tags while selection order survives filtering", async ({ page }) => {
    const devices = [
        { id: 1, ip: "192.0.2.10", name: "core-ten", display_name: "東京本社SW", tags: ["東京HQ", "Core"], status: "online" },
        { id: 2, ip: "192.0.2.2", name: "edge-two", display_name: null, tags: ["大阪"], status: "online" },
    ];
    await page.route("**/api/devices", (route) => route.fulfill({ json: devices }));
    let payload: { device_ids: number[]; sort_by: string } | undefined;
    await page.route("**/api/devices/bulk-rename/preview", async (route) => {
        payload = route.request().postDataJSON();
        await route.fulfill({ json: { ok: true, devices: [] } });
    });
    await page.goto("/");
    await page.evaluate(() => window.refreshDashboard?.());
    const search = page.locator("#device-search");
    await expect(search).toHaveCount(1);
    for (const term of ["東京本社", "core-ten", "192.0.2.10", "東京HQ", "cOrE"]) {
        await search.fill(term);
        await expect(page.locator("#dash-tbody tr")).toHaveCount(1);
        await expect(page.locator("#dash-tbody")).toContainText("東京本社SW");
    }
    await search.fill("大阪");
    await page.locator("[data-device-id='2']").check();
    await search.fill("東京HQ");
    await page.locator("[data-device-id='1']").check();
    await expect(page.locator("#device-selection-count")).toContainText("2");
    await page.locator("#bulk-rename").click();
    await page.getByLabel("Sort order").selectOption("selection_order");
    await page.getByRole("button", { name: "Preview", exact: true }).click();
    await expect.poll(() => payload?.device_ids).toEqual([2, 1]);
    expect(payload?.sort_by).toBe("selection_order");
    await page.getByRole("button", { name: "Cancel", exact: true }).click();
    await page.evaluate(() => window.setLanguage?.("ja"));
    await expect(page.locator("#bulk-rename")).toHaveText("一括リネーム（連番付与）");
    await page.setViewportSize({ width: 375, height: 667 });
    await page.locator("#bulk-rename").click();
    await expect(page.getByLabel("接頭辞", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "プレビュー", exact: true })).toBeVisible();
    expect(await page.locator("#bulk-rename-dialog").evaluate((element) => element.scrollWidth <= element.clientWidth)).toBeTruthy();
});

test("encoded IPv6 device paths support metadata edits, detail pages and removal", async ({ request }) => {
    const ip = "2001:db8::210";
    const encoded = encodeURIComponent(ip);
    try {
        expect((await request.post("/api/discovery/register", {
            data: [{ ip, name: "ipv6-switch", community: "public" }],
        })).ok()).toBeTruthy();
        expect((await request.post(`/api/device/${encoded}/metadata`, {
            data: { display_name: "東京IPv6-SW", tags: ["IPv6"] },
        })).ok()).toBeTruthy();
        const response = await request.get(`/device/${encoded}`);
        expect(response.ok()).toBeTruthy();
        expect(await response.text()).not.toContain("Device not found.");
        const devices = await (await request.get("/api/devices")).json();
        expect(devices.find((device: { ip: string }) => device.ip === ip).display_name).toBe("東京IPv6-SW");
    } finally {
        expect((await request.delete(`/api/device/${encoded}`)).ok()).toBeTruthy();
    }
});

test("alias and tag chips edit Japanese safely, retaining edits after API failure", async ({ page }) => {
    await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
    await page.route("**/api/device/*", (route) => route.fulfill({
        json: { ip: "192.0.2.1", name: "original-sysName", display_name: "東京SW", tags: ["Core"], status: "online" },
    }));
    let payload: { display_name: string | null; tags: string[] } | undefined;
    let fail = true;
    await page.route("**/api/device/*/metadata", async (route) => {
        payload = route.request().postDataJSON();
        await route.fulfill(fail ? { status: 400, json: { error: "Save rejected" } } : { json: { ok: true } });
    });
    await page.goto("/");
    await page.evaluate(() => {
        document.body.insertAdjacentHTML("beforeend", "<div><h1 id='dev-title'></h1></div>");
    });
    await page.addScriptTag({ url: "/static/js/device-detail.js" });
    await page.addScriptTag({ content: `TracePulseDeviceDetail.renderDetail({
        ip:"192.0.2.1",name:"original-sysName",display_name:"東京SW",tags:["Core"]
    });` });
    await expect(page.locator("#dev-title")).toContainText("東京SW");
    await expect(page.locator("#device-identity")).toContainText("original-sysName");
    await page.locator("#edit-device-metadata").click();
    const dialog = page.locator("#device-metadata-dialog");
    await dialog.getByLabel("Display name", { exact: true }).fill("大阪支店SW-01");
    await dialog.getByLabel("Remove Core", { exact: true }).click();
    await dialog.getByLabel("Tags", { exact: true }).fill("東京HQ");
    await dialog.getByRole("button", { name: "Add tag", exact: true }).click();
    await dialog.getByLabel("Tags", { exact: true }).fill("<img src=x onerror=alert(1)>");
    await dialog.getByLabel("Tags", { exact: true }).press("Enter");
    await expect(dialog.locator("#device-tag-chips img")).toHaveCount(0);
    await dialog.getByRole("button", { name: "Save", exact: true }).click();
    await expect(dialog.getByRole("alert")).toContainText("Save rejected");
    await expect(dialog.getByLabel("Display name", { exact: true })).toHaveValue("大阪支店SW-01");
    expect(payload).toEqual({ display_name: "大阪支店SW-01", tags: ["東京HQ", "<img src=x onerror=alert(1)>"] });
    fail = false;
    await dialog.getByRole("button", { name: "Save", exact: true }).click();
    await expect(dialog).not.toBeVisible();
});
