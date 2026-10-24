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
        await page.getByRole("button", { name: "Set / change device names", exact: true }).click();
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
        await expect(page.locator("#dash-table thead th")).toHaveCount(9);
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
    await page.getByRole("button", { name: "Set / change device names", exact: true }).click();
    await page.getByLabel("Sort order").selectOption("selection_order");
    await page.getByRole("button", { name: "Preview", exact: true }).click();
    await expect.poll(() => payload?.device_ids).toEqual([2, 1]);
    expect(payload?.sort_by).toBe("selection_order");
    await page.getByRole("button", { name: "Cancel", exact: true }).click();
    await page.evaluate(() => window.setLanguage?.("ja"));
    await expect(page.locator("#bulk-rename")).toHaveText("一括操作");
    await page.setViewportSize({ width: 375, height: 667 });
    await page.locator("#bulk-rename").click();
    await page.getByRole("button", { name: "機器名の設定・変更", exact: true }).click();
    await expect(page.getByLabel("接頭辞", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "プレビュー", exact: true })).toBeVisible();
    expect(await page.locator("#bulk-rename-dialog").evaluate((element) => element.scrollWidth <= element.clientWidth)).toBeTruthy();
});

test("bulk actions append tags, retain aliases, confirm unregister and recover from errors", async ({ page, request }) => {
    const ips = ["192.0.2.231", "192.0.2.232"];
    try {
        await request.post("/api/discovery/register", { data: ips.map((ip) => ({ ip, name: `bulk-${ip}`, community: "test" })) });
        for (const ip of ips) await request.post(`/api/device/${ip}/metadata`, { data: { display_name: "東京SW", tags: ["Existing"] } });
        await page.goto("/");
        await page.locator("#device-search").fill("192.0.2.23");
        await page.locator("#select-visible-devices").check();
        await page.getByRole("button", { name: "Bulk actions", exact: true }).click();
        const chooser = page.locator("#bulk-actions-dialog");
        await expect(chooser).toContainText(ips[0]);
        await expect(chooser).toContainText(ips[1]);
        await chooser.getByRole("button", { name: "Add tags", exact: true }).click();
        const confirm = page.locator("#bulk-action-confirm");
        await confirm.getByLabel("Tags", { exact: true }).fill("東京");
        await confirm.getByRole("button", { name: "Add tag", exact: true }).click();
        await confirm.getByLabel("Tags", { exact: true }).fill("Existing");
        await confirm.getByRole("button", { name: "Add tag", exact: true }).click();
        await confirm.getByLabel("Tags", { exact: true }).fill("東京");
        await confirm.getByRole("button", { name: "Add tag", exact: true }).click();
        await expect(confirm.locator(".device-tag")).toHaveCount(2);
        await confirm.getByRole("button", { name: "Confirm", exact: true }).click();
        await expect(confirm).not.toBeVisible();
        for (const ip of ips) {
            const device = (await (await request.get("/api/devices")).json()).find((item: { ip: string }) => item.ip === ip);
            expect(device.tags).toEqual(["Existing", "東京"]);
            expect(device.display_name).toBe("東京SW");
        }
        await page.locator("#bulk-rename").click();
        await chooser.getByRole("button", { name: "Unregister", exact: true }).click();
        await expect(confirm).toContainText("cannot be undone");
        await confirm.getByRole("button", { name: "Cancel", exact: true }).click();
        expect((await (await request.get("/api/devices")).json()).filter((item: { ip: string }) => ips.includes(item.ip))).toHaveLength(2);
        await page.locator("#bulk-rename").click();
        await chooser.getByRole("button", { name: "Unregister", exact: true }).click();
        await page.route("**/api/devices/bulk-action", (route) => route.fulfill({ status: 500, json: { error: "Temporary failure" } }));
        await confirm.getByRole("button", { name: "Confirm", exact: true }).click();
        await expect(confirm.locator("[role=alert]")).toContainText("Temporary failure");
        await expect(confirm.getByRole("button", { name: "Confirm", exact: true })).toBeEnabled();
        await page.unroute("**/api/devices/bulk-action");
        await confirm.getByRole("button", { name: "Confirm", exact: true }).click();
        await expect(confirm).not.toBeVisible();
        await expect(page.locator("#device-selection-count")).toContainText("0");
        expect((await (await request.get("/api/devices")).json()).filter((item: { ip: string }) => ips.includes(item.ip))).toHaveLength(0);
        await expect(page.getByRole("button", { name: "Bulk actions", exact: true })).toBeDisabled();
    } finally {
        for (const ip of ips) await request.delete(`/api/device/${ip}`);
    }
});

test("dashboard column hover text remains readable in both themes without losing sort indicators", async ({ page }) => {
    await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
    await page.goto("/");
    await page.evaluate(() => window.refreshDashboard?.());
    for (const theme of ["light", "dark"]) {
        await page.locator("#theme-select").selectOption(theme);
        await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
        const headers = page.locator("#dash-table th");
        for (const header of await headers.all()) {
            await header.hover();
            const sorted = await header.evaluate((element) =>
                element.classList.contains("sort-asc") || element.classList.contains("sort-desc"));
            await expect(header).toHaveCSS("color", sorted ? "rgb(56, 189, 248)"
                : theme === "light" ? "rgb(15, 23, 42)" : "rgb(226, 232, 240)");
        }
        const nameHeader = page.locator("#th-display_name");
        await nameHeader.click();
        await expect(nameHeader).toHaveClass("sort-asc");
        await nameHeader.hover();
        await expect(nameHeader).toHaveCSS("color", "rgb(56, 189, 248)");
        await nameHeader.click();
        await expect(nameHeader).toHaveClass("sort-desc");
        await expect(nameHeader).toHaveCSS("color", "rgb(56, 189, 248)");
    }
});

test("row action buttons have readable contrast in light and dark themes", async ({ page }) => {
    await page.route("**/api/devices", (route) => route.fulfill({ json: [
        { id: 1, ip: "192.0.2.10", name: "core", tags: [], status: "online" },
    ] }));
    await page.goto("/");
    await page.evaluate(() => window.refreshDashboard?.());
    const button = page.locator(".device-action-button").first();
    await expect(button).toBeVisible();
    for (const theme of ["light", "dark"]) {
        await page.locator("#theme-select").selectOption(theme);
        await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
        if (theme === "light") {
            await expect(button).toHaveCSS("background-color", "rgb(241, 245, 249)");
            await expect(button).toHaveCSS("color", "rgb(30, 41, 59)");
        }
        const contrast = await button.evaluate((element) => {
            const style = getComputedStyle(element);
            const luminance = (color: string) => {
                const channels = color.match(/\d+/g)!.slice(0, 3).map((value) => {
                    const channel = Number(value) / 255;
                    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
                });
                return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
            };
            const foreground = luminance(style.color);
            const background = luminance(style.backgroundColor);
            return (Math.max(foreground, background) + 0.05) / (Math.min(foreground, background) + 0.05);
        });
        expect(contrast, `${theme} action button contrast`).toBeGreaterThanOrEqual(4.5);
        await button.click();
        await expect(page.locator("#bulk-actions-dialog")).toBeVisible();
        await page.keyboard.press("Escape");
    }
});

test("row actions target only that device and allow direct naming, tagging and confirmed unregister", async ({ page, request }) => {
    const ips = ["192.0.2.233", "192.0.2.234"];
    try {
        await request.post("/api/discovery/register", { data: ips.map((ip) => ({ ip, name: `row-${ip}`, community: "test" })) });
        await page.goto("/");
        await page.locator("#device-search").fill("192.0.2.23");
        await page.locator("#select-visible-devices").check();
        const row = page.locator("#dash-tbody tr").filter({ hasText: ips[0] });
        const chooser = page.locator("#bulk-actions-dialog");
        await row.getByRole("button", { name: "Actions", exact: true }).click();
        await expect(chooser).toContainText(ips[0]);
        await expect(chooser).not.toContainText(ips[1]);
        await chooser.getByRole("button", { name: "Set / change device names", exact: true }).click();
        const editor = page.locator("#device-metadata-dialog");
        await editor.getByLabel("Display name", { exact: true }).fill("東京コアSW");
        await editor.getByRole("button", { name: "Save", exact: true }).click();
        await expect(editor).not.toBeVisible();
        await expect(row).toContainText("東京コアSW");
        await row.getByRole("button", { name: "Actions", exact: true }).click();
        await chooser.getByRole("button", { name: "Add tags", exact: true }).click();
        const confirm = page.locator("#bulk-action-confirm");
        await confirm.getByLabel("Tags", { exact: true }).fill("Core");
        await confirm.getByRole("button", { name: "Confirm", exact: true }).click();
        await expect(confirm).not.toBeVisible();
        await expect(row).toContainText("Core");
        const devices = await (await request.get("/api/devices")).json();
        expect(devices.find((device: { ip: string }) => device.ip === ips[1]).tags).toEqual([]);
        await page.evaluate(() => window.setLanguage?.("ja"));
        await row.getByRole("button", { name: "操作", exact: true }).click();
        await chooser.getByRole("button", { name: "登録解除", exact: true }).click();
        await expect(confirm).not.toContainText(ips[1]);
        await confirm.getByRole("button", { name: "キャンセル", exact: true }).click();
        await expect(row).toBeVisible();
        await row.getByRole("button", { name: "操作", exact: true }).click();
        await chooser.getByRole("button", { name: "登録解除", exact: true }).click();
        await confirm.getByRole("button", { name: "確定", exact: true }).click();
        await expect(confirm).not.toBeVisible();
        await expect(row).toHaveCount(0);
        await expect(page.locator("#dash-tbody")).toContainText(ips[1]);
        await expect(page.locator("#device-selection-count")).toContainText("1");
    } finally {
        for (const ip of ips) await request.delete(`/api/device/${ip}`);
    }
});

test("device name, hostname and tags occupy separate sortable columns with safe fallbacks", async ({ page }) => {
    const devices = [
        { id: 1, ip: "192.0.2.1", name: "z-host", display_name: "A 東京SW", tags: ["Z-tag", "<script>"], status: "online" },
        { id: 2, ip: "192.0.2.2", name: "b-host", tags: ["A-tag"], status: "online" },
        { id: 3, ip: "192.0.2.3", name: "", tags: [], status: "online" },
    ];
    await page.route("**/api/devices", (route) => route.fulfill({ json: devices }));
    await page.goto("/");
    await page.evaluate(() => window.refreshDashboard?.());
    const first = page.locator("#dash-tbody tr").filter({ has: page.locator("[data-device-id='1']") });
    await expect(first.locator("td").nth(2)).toHaveText("A 東京SW");
    await expect(first.locator("td").nth(3)).toHaveText("z-host");
    await expect(first.locator("td").nth(4).locator(".device-tag")).toHaveText(["Z-tag", "<script>"]);
    await expect(first.locator("script")).toHaveCount(0);
    const fallback = page.locator("#dash-tbody tr").filter({ has: page.locator("[data-device-id='3']") });
    await expect(fallback.locator("td").nth(2)).toHaveText("192.0.2.3");
    await expect(fallback.locator("td").nth(3)).toHaveText("-");
    await expect(fallback.locator("td").nth(4)).toHaveText("-");
    await page.locator("#th-name").click();
    await expect(page.locator("#dash-tbody tr").nth(1).locator("td").nth(3)).toHaveText("b-host");
    await page.locator("#th-display_name").click();
    await expect(page.locator("#dash-tbody tr").first().locator("td").nth(2)).toHaveText("192.0.2.3");
    await page.locator("#th-tags").click();
    await expect(page.locator("#dash-tbody tr").nth(1).locator("td").nth(4)).toHaveText("A-tag");
    await page.evaluate(() => window.setLanguage?.("ja"));
    await expect(page.locator("#th-display_name")).toContainText("機器名");
    await expect(page.locator("#th-tags")).toContainText("タグ");
    await page.locator("#device-search").fill("no-match-value");
    await expect(page.locator("#dash-tbody .empty")).toHaveAttribute("colspan", "9");
});

test("column filters combine with AND, search, refresh and visible-device selection", async ({ page }) => {
    const devices = [
        { id: 1, ip: "192.0.2.10", name: "core-ten", display_name: "東京SW", tags: ["Core", "東京"], status: "online", health_score: 90 },
        { id: 2, ip: "192.0.2.20", name: "edge-twenty", display_name: "大阪SW", tags: ["Edge"], status: "offline", health_score: 20 },
        { id: 3, ip: "2001:db8::1", name: "core-six", display_name: null, tags: ["Core"], status: "online", health_score: null },
    ];
    await page.route("**/api/devices", (route) => route.fulfill({ json: devices }));
    await page.goto("/");
    await page.evaluate(() => window.refreshDashboard?.());
    await expect(page.locator("#dash-tbody tr")).toHaveCount(3);
    const add = page.locator("#device-filter-add");
    const rows = page.locator(".device-filter-row");
    await add.click();
    await rows.nth(0).locator("[data-filter-field]").selectOption("tags");
    await rows.nth(0).locator("[data-filter-operator]").selectOption("equals");
    await rows.nth(0).locator("[data-filter-value]").fill("cOrE");
    await expect(page.locator("#dash-tbody tr")).toHaveCount(2);
    await add.click();
    await rows.nth(1).locator("[data-filter-field]").selectOption("health_score");
    await rows.nth(1).locator("[data-filter-value]").fill("90");
    await expect(page.locator("#dash-tbody tr")).toHaveCount(1);
    await expect(page.locator("#dash-tbody")).toContainText("東京SW");
    await page.locator("#select-visible-devices").check();
    await expect(page.locator("#device-selection-count")).toContainText("1");
    await page.evaluate(() => window.refreshDashboard?.());
    await expect(page.locator("#dash-tbody tr")).toHaveCount(1);
    await rows.nth(1).locator("[data-filter-value]").fill("bad");
    await expect(page.locator("#device-filters [role=alert]")).toContainText("0 to 100");
    await expect(page.locator("#dash-tbody .empty")).toHaveCount(1);
    await rows.nth(1).locator("[data-filter-value]").fill("101");
    await expect(page.locator("#device-filters [role=alert]")).not.toBeEmpty();
    await rows.nth(1).locator("[data-filter-value]").fill("0");
    await expect(page.locator("#device-filters [role=alert]")).toBeEmpty();
    await rows.nth(1).locator("[data-filter-operator]").selectOption("lt");
    await rows.nth(1).locator("[data-filter-value]").fill("90");
    await expect(page.locator("#dash-tbody .empty")).toHaveCount(1);
    await rows.nth(1).locator("[data-filter-operator]").selectOption("lte");
    await expect(page.locator("#dash-tbody")).toContainText("東京SW");
    for (const [operator, value, matches] of [
        ["equals", "90", true], ["not_equals", "90", false], ["gt", "90", false],
        ["gt", "89.9", true], ["gte", "90", true], ["lte", "89.9", false],
    ] as const) {
        await rows.nth(1).locator("[data-filter-operator]").selectOption(operator);
        await rows.nth(1).locator("[data-filter-value]").fill(value);
        await expect(page.locator("#dash-tbody .empty")).toHaveCount(matches ? 0 : 1);
    }
    await rows.nth(1).locator("[data-filter-value]").fill("");
    await expect(page.locator("#dash-tbody tr")).toHaveCount(2);
    await rows.nth(1).locator("[data-filter-remove]").click();
    await rows.nth(0).locator("[data-filter-field]").selectOption("status");
    await rows.nth(0).locator("[data-filter-value]").selectOption("offline");
    await expect(page.locator("#dash-tbody")).toContainText("大阪SW");
    await rows.nth(0).locator("[data-filter-operator]").selectOption("not_equals");
    await expect(page.locator("#dash-tbody tr")).toHaveCount(2);
    await rows.nth(0).locator("[data-filter-field]").selectOption("display_name");
    await rows.nth(0).locator("[data-filter-value]").fill("core-six");
    await expect(page.locator("#dash-tbody .empty")).toHaveCount(1);
    await rows.nth(0).locator("[data-filter-field]").selectOption("name");
    await rows.nth(0).locator("[data-filter-value]").fill("core-six");
    await expect(page.locator("#dash-tbody")).toContainText("2001:db8::1");
    await rows.nth(0).locator("[data-filter-field]").selectOption("ip");
    await rows.nth(0).locator("[data-filter-operator]").selectOption("starts_with");
    await rows.nth(0).locator("[data-filter-value]").fill("192.0.2.");
    await page.locator("#device-search").fill("大阪");
    await expect(page.locator("#dash-tbody tr")).toHaveCount(1);
    await expect(page.locator("#dash-tbody")).toContainText("大阪SW");
    await page.evaluate(() => window.setLanguage?.("ja"));
    await expect(add).toHaveText("条件を追加");
    await expect(rows.nth(0).locator("[data-filter-value]")).toHaveValue("192.0.2.");
    await page.setViewportSize({ width: 375, height: 667 });
    expect(await page.locator("#device-filters").evaluate((element) => element.scrollWidth <= element.clientWidth)).toBeTruthy();
    await page.locator("#device-filter-clear").click();
    await page.locator("#device-search").fill("");
    await expect(page.locator("#dash-tbody tr")).toHaveCount(3);
    await expect(page.locator("[data-device-id='1']")).toBeChecked();
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
