import { expect, test, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";

type AlertFixture = {
    id: number;
    device_id: number;
    device_name: string;
    device_ip: string;
    alert_type: string;
    severity: string;
    details: string;
    created_at: string;
    acknowledged_at: string | null;
};

async function mockAlerts(page: Page): Promise<{ items: AlertFixture[]; requests: URL[] }> {
    const now = Date.now();
    const items: AlertFixture[] = Array.from({ length: 45 }, (_, index) => ({
        id: index + 1,
        device_id: 1,
        device_name: `router-${index + 1}`,
        device_ip: `192.0.2.${index + 1}`,
        alert_type: index % 2 === 0 ? "link_down" : "cpu_high",
        severity: index % 3 === 0 ? "critical" : index % 3 === 1 ? "warning" : "info",
        details: index === 39 ? "match-target on router-40" : `Alert details for router-${index + 1}`,
        created_at: new Date(now - index * 60 * 60 * 1000).toISOString(),
        acknowledged_at: index % 4 === 0 ? new Date(now - 30 * 60 * 1000).toISOString() : null,
    }));
    const requests: URL[] = [];
    await page.route("**/api/alerts**", async (route) => {
        const request = route.request();
        const url = new URL(request.url());
        requests.push(url);
        if (request.method() === "GET") {
            const params = url.searchParams;
            const query = (params.get("q") || "").toLowerCase();
            const filtered = items.filter((item) =>
                (!params.get("severity") || item.severity === params.get("severity")) &&
                (params.get("status") === "acknowledged" ? !!item.acknowledged_at :
                    params.get("status") === "unacknowledged" ? !item.acknowledged_at : true) &&
                (!query || [item.device_name, item.device_ip, item.alert_type, item.details]
                    .some((value) => value.toLowerCase().includes(query))));
            const total = filtered.length;
            const offset = Number(params.get("offset") || 0);
            const limit = Number(params.get("limit") || 50);
            return await route.fulfill({ json: {
                total,
                unacknowledged_count: items.filter((item) =>
                    !item.acknowledged_at &&
                    (!params.get("severity") || item.severity === params.get("severity")) &&
                    (!query || [item.device_name, item.device_ip, item.alert_type, item.details]
                        .some((value) => value.toLowerCase().includes(query)))).length,
                items: filtered.slice(offset, offset + limit),
            } });
        }
        if (url.pathname === "/api/alerts/ack-all") {
            const body = request.postDataJSON() as { severity?: string; q?: string };
            const query = (body.q || "").toLowerCase();
            let updated = 0;
            items.forEach((item) => {
                if (!item.acknowledged_at &&
                    (!body.severity || item.severity === body.severity) &&
                    (!query || [item.device_name, item.device_ip, item.alert_type, item.details]
                        .some((value) => value.toLowerCase().includes(query)))) {
                    item.acknowledged_at = new Date().toISOString();
                    updated++;
                }
            });
            return await route.fulfill({ json: { ok: true, updated } });
        }
        if (url.pathname === "/api/alerts/clear") {
            const body = request.postDataJSON() as { mode: string };
            const previousLength = items.length;
            if (body.mode === "acknowledged") {
                for (let index = items.length - 1; index >= 0; index--) {
                    if (items[index].acknowledged_at) items.splice(index, 1);
                }
            } else {
                items.splice(0, items.length);
            }
            return await route.fulfill({ json: { ok: true, deleted: previousLength - items.length } });
        }
        const id = Number(url.pathname.match(/\/api\/alerts\/(\d+)\/ack$/)?.[1]);
        const item = items.find((alert) => alert.id === id);
        if (!item) return await route.fulfill({ status: 404, json: { error: "alert not found" } });
        item.acknowledged_at ||= new Date().toISOString();
        return await route.fulfill({ json: { ok: true, id, acknowledged_at: item.acknowledged_at } });
    });
    return { items, requests };
}

test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
        localStorage.setItem("tracepulse-onboarding-v1", "done");
        localStorage.setItem("tracepulse-lang", "en");
    });
});

test("alert history filters, paginates, acknowledges, exports and clears acknowledged history", async ({ page }) => {
    const fixture = await mockAlerts(page);
    await page.goto("/alerts");
    const alerts = page.locator("#alerts-page");
    await expect(alerts.locator("#alerts-total")).toHaveText("45");
    await expect(alerts.locator("#alerts-unacknowledged")).toHaveText("33");
    await expect(alerts.locator(".alerts-table tbody tr")).toHaveCount(20);
    await expect(alerts.locator(".alerts-table thead th")).toHaveText([
        "Occurred at", "Device", "IP address", "Type", "Severity", "Details", "Status", "Acknowledged at",
    ]);
    const firstRow = alerts.locator(".alerts-table tbody tr").nth(0);
    await expect(firstRow.locator("td")).toHaveCount(8);
    await expect(firstRow.locator("td").nth(1).getByRole("link")).toHaveText("router-1");
    await expect(firstRow.locator("td").nth(2)).toHaveText("192.0.2.1");
    await expect(firstRow.locator("td").nth(6)).toHaveText("Acknowledged");
    await expect(firstRow.locator("td").nth(6).getByRole("button")).toHaveCount(0);
    await expect(firstRow.locator("td").nth(7)).not.toHaveText("-");
    const secondRow = alerts.locator(".alerts-table tbody tr").nth(1);
    await expect(secondRow.locator("td").nth(6).getByRole("button")).toHaveAccessibleName("Acknowledge");
    await expect(secondRow.locator("td").nth(7)).toHaveText("-");

    const downloadPromise = page.waitForEvent("download");
    await alerts.getByRole("button", { name: "Export CSV", exact: true }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toMatch(/^tracepulse-alerts-\d{4}-\d{2}-\d{2}\.csv$/);
    const downloadPath = await download.path();
    if (!downloadPath) throw new Error("CSV download did not produce a file");
    const csv = (await readFile(downloadPath, "utf8")).replace(/^\uFEFF/, "");
    const [header] = csv.split("\r\n");
    expect(header).toBe('"Occurred at","Device","IP address","Type","Severity","Details","Status","Acknowledged at"');
    expect(csv).toContain('"router-2","192.0.2.2","cpu_high","warning","Alert details for router-2","Unacknowledged","-"');

    await alerts.getByRole("button", { name: "Next", exact: true }).click();
    await expect(alerts.locator(".alerts-table tbody")).toContainText("router-21");
    expect(fixture.requests[fixture.requests.length - 1]?.searchParams.get("offset")).toBe("20");
    await alerts.locator("#alerts-page-size").selectOption("50");
    await expect(alerts.locator(".alerts-table tbody tr")).toHaveCount(45);

    await alerts.getByLabel("Search").fill("match-target");
    await alerts.locator("#alerts-severity").selectOption("critical");
    await expect(alerts.locator("#alerts-total")).toHaveText("1");
    await expect(alerts.locator(".alerts-table tbody tr")).toHaveCount(1);
    await expect(alerts.locator(".alerts-table tbody")).toContainText("router-40");
    await alerts.getByRole("button", { name: "Acknowledge", exact: true }).click();
    await expect.poll(() => fixture.items[39].acknowledged_at).not.toBeNull();
    await expect(alerts.locator(".alert-acknowledged")).toHaveText("Acknowledged");
    await expect(alerts.locator(".alerts-acknowledged-time")).toBeVisible();

    await alerts.getByLabel("Search").fill("");
    await alerts.locator("#alerts-severity").selectOption("");
    await alerts.locator("#alerts-status").selectOption("unacknowledged");
    await expect(alerts.locator("#alerts-total")).toHaveText("32");
    await alerts.getByRole("button", { name: "Acknowledge All", exact: true }).click();
    await expect(alerts.locator("#alerts-total")).toHaveText("0");
    await expect(alerts.locator("#alerts-unacknowledged")).toHaveText("0");
    await expect(alerts.locator(".alerts-message")).toContainText("32 alert(s) acknowledged.");

    await alerts.locator("#alerts-status").selectOption("all");
    await expect(alerts.locator("#alerts-total")).toHaveText("45");

    await alerts.getByRole("button", { name: "Clear History", exact: true }).click();
    const dialog = alerts.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await dialog.getByRole("button", { name: "Clear acknowledged", exact: true }).click();
    await expect(alerts.locator("#alerts-total")).toHaveText("0");
    await expect(alerts.locator(".alerts-message")).toHaveText("Acknowledged alerts cleared.");
    expect(fixture.items).toHaveLength(0);
});

test("alert history clear-all dialog and translations work in both themes", async ({ page }) => {
    const fixture = await mockAlerts(page);
    await page.goto("/alerts");
    const alerts = page.locator("#alerts-page");
    await page.locator("#theme-select").selectOption("light");
    await page.evaluate(() => window.setLanguage?.("ja"));
    await expect(page.getByRole("heading", { name: "アラート履歴", exact: true })).toBeVisible();
    const acknowledgeButton = alerts.locator(".alerts-ack-button").first();
    await expect(acknowledgeButton).toHaveCSS("background-color", "rgb(15, 118, 110)");
    await expect(acknowledgeButton).toHaveCSS("color", "rgb(255, 255, 255)");
    await expect(alerts.locator(".alerts-table thead th")).toHaveText([
        "発生日時", "機器", "IP アドレス", "種別", "重要度", "詳細", "対応状況", "確認日時",
    ]);
    await expect(alerts.getByRole("button", { name: "履歴をクリア", exact: true })).toBeVisible();
    await alerts.getByRole("button", { name: "履歴をクリア", exact: true }).click();
    const dialog = alerts.getByRole("dialog");
    await expect(dialog).toContainText("この操作は取り消せません");
    await dialog.getByRole("button", { name: "すべて削除", exact: true }).click();
    await expect(alerts.locator("#alerts-total")).toHaveText("0");
    await expect.poll(() => fixture.items).toHaveLength(0);
    expect(fixture.items).toHaveLength(0);
});

test("alert history table fits desktop and scrolls horizontally on narrow screens", async ({ page }) => {
    const fixture = await mockAlerts(page);
    fixture.items[1].alert_type = "DEVICE_OFFLINE";
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto("/alerts");
    const tableScroll = page.locator("#alerts-page .alerts-table-scroll");
    await expect(tableScroll.locator(".alerts-table thead th")).toHaveCount(8);
    const typeCell = tableScroll.locator("tbody tr").nth(1).locator("td").nth(3);
    await expect(typeCell).toHaveText("DEVICE_OFFLINE");
    await expect(typeCell).toHaveCSS("white-space", "nowrap");
    const acknowledgeButton = tableScroll.locator(".alerts-ack-button").first();
    await expect(acknowledgeButton).toContainText("✓");
    await expect(acknowledgeButton).toHaveCSS("cursor", "pointer");
    await expect(acknowledgeButton).toHaveCSS("font-weight", "600");
    await expect(acknowledgeButton).toHaveCSS("padding", "6px 12px");
    await expect.poll(async () => tableScroll.evaluate((element) =>
        element.scrollWidth - element.clientWidth,
    )).toBeLessThanOrEqual(2);

    await page.setViewportSize({ width: 375, height: 667 });
    await expect.poll(async () => tableScroll.evaluate((element) =>
        element.scrollWidth - element.clientWidth,
    )).toBeGreaterThan(0);
});

test("dashboard recent alerts link opens the full history page", async ({ page }) => {
    await page.route("**/api/devices", route => route.fulfill({ json: [] }));
    await page.goto("/");
    await expect(page.locator(".dashboard-recent-alerts a[href='/alerts']")).toHaveText("View all →");
    await page.locator(".dashboard-recent-alerts a[href='/alerts']").click();
    await expect(page).toHaveURL(/\/alerts$/);
    await expect(page.getByRole("heading", { name: "Alert History", exact: true })).toBeVisible();
});
