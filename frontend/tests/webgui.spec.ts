import { expect, test, type Page } from "@playwright/test";

type DiscoveryReply = { status?: number; json?: unknown; abort?: boolean };
async function registrationFixture(page: Page) {
    const devices = Array.from({ length: 5 }, (_, index) => ({ ip: `198.51.100.${index + 201}`, name: `registration-${index + 1}`, status: "online" }));
    let scanDevices = devices;
    let scans = 0;
    let finish: ((reply: DiscoveryReply) => void) | undefined;
    const payloads: Array<Array<{ ip: string; name: string; community: string }>> = [];
    await page.route("**/api/discovery/scan", route => {
        scans++;
        return route.fulfill({ json: { job_id: `registration-${scans}`, total: scanDevices.length } });
    });
    await page.route("**/api/discovery/scan/registration-*", route => route.fulfill({ json: { status: "done", scanned: scanDevices.length, total: scanDevices.length, elapsed: 1, devices: scanDevices } }));
    await page.route("**/api/discovery/register", async route => {
        expect(route.request().headers()["x-csrf-token"]).toBe("registration-csrf");
        payloads.push(route.request().postDataJSON());
        const reply = await new Promise<DiscoveryReply>(resolve => { finish = resolve; });
        if (reply.abort) await route.abort("failed");
        else await route.fulfill({ status: reply.status ?? 200, json: reply.json });
    });
    await page.goto("/discovery");
    await page.evaluate(ip => { window.tracepulseCsrf = "registration-csrf"; window.DISCOVERY_EXISTING = new Set([ip]); }, devices[0].ip);
    const scan = async (next = devices, community = "scan-only-secret") => {
        scanDevices = next;
        await page.locator("#community").fill(community);
        await page.locator("#cidr").fill("198.51.100.192/28");
        await page.locator("#scan-btn").click();
        await expect(page.locator("#results-body tr")).toHaveCount(next.length);
        await expect(page.locator("#results-body tr").first()).toContainText(next[0].ip);
    };
    await scan();
    return { devices, payloads, scan, get scans() { return scans; }, reply: (reply: DiscoveryReply) => finish!(reply), row: (index: number) => page.locator("#results-body tr").filter({ hasText: devices[index].ip }) };
}

for (const [width, language, theme] of [[1280, "en", "light"], [1280, "ja", "dark"], [375, "en", "dark"], [375, "ja", "light"]] as const) {
    test(`discovery individual registration and partial bulk results at ${width} ${language} ${theme}`, async ({ page }) => {
        await page.setViewportSize({ width, height: 900 });
        await page.addInitScript(lang => { localStorage.setItem("tracepulse-onboarding-v1", "done"); localStorage.setItem("tracepulse-lang", lang); }, language);
        const setup = await registrationFixture(page);
        await page.locator("#theme-select").selectOption(theme);
        expect(setup.payloads).toHaveLength(0);
        await expect(setup.row(0).locator(".badge-registered")).toHaveText(language === "ja" ? "登録済み" : "Registered");
        await expect(setup.row(0).locator("input")).toBeDisabled();
        await expect(page.locator(".register-device")).toHaveCount(4);
        await page.locator("#select-all").check();
        await page.locator("#community").fill("changed-after-scan-secret");
        const originalButton = await setup.row(1).locator(".register-device").elementHandle();
        await setup.row(1).locator(".register-device").click();
        await expect(setup.row(1).getByRole("progressbar")).toBeVisible();
        await expect(setup.row(1).getByRole("status")).toHaveText(language === "ja" ? "登録中…" : "Registering…");
        await expect(page.locator("#results-section .operation-progress").getByRole("status")).toHaveText(language === "ja" ? /経過 [1-9]\d*秒/ : /[1-9]\d*s elapsed/);
        await originalButton!.evaluate(button => button.dispatchEvent(new MouseEvent("click")));
        await page.locator("#register-btn").dispatchEvent("click");
        expect(setup.payloads).toHaveLength(1);
        expect(setup.payloads[0]).toEqual([{ ip: setup.devices[1].ip, name: setup.devices[1].name, community: "scan-only-secret" }]);
        setup.reply({ json: { registered: [setup.devices[1].ip], skipped: [], errors: [] } });
        await expect(setup.row(1).locator(".badge-registered")).toBeVisible();
        await expect(setup.row(1).locator("input")).not.toBeChecked();
        await expect(setup.row(1).locator("input")).toBeDisabled();
        await expect(page.locator("#select-count")).toContainText("3");
        await page.locator("#register-btn").click();
        await expect.poll(() => setup.payloads.length).toBe(2);
        expect(setup.payloads[1].map(device => device.ip)).toEqual(setup.devices.slice(2).map(device => device.ip));
        expect(setup.payloads[1].every(device => device.community === "scan-only-secret")).toBeTruthy();
        setup.reply({ json: { registered: [setup.devices[2].ip], skipped: [setup.devices[3].ip], errors: [`${setup.devices[4].ip}: Device registration limit reached; scan-only-secret internal-stack-marker`] } });
        await expect(page.locator("#results-body .badge-registered")).toHaveCount(4);
        await expect(setup.row(4).getByRole("alert")).toHaveText(language === "ja" ? "機器の登録上限に達しています。" : "The device registration limit has been reached.");
        await expect(setup.row(4).locator("input")).toBeChecked();
        await expect(setup.row(4).locator(".register-device")).toBeEnabled();
        await expect(page.locator("#select-count")).toContainText("1");
        for (const [reply, message] of [
            [{ status: 403, json: { error: "private-token internal-stack-marker" } }, language === "ja" ? "登録が許可されていません。権限とセッションを確認してください。" : "Registration is not permitted. Check your permissions and session."],
            [{ abort: true }, language === "ja" ? "サーバーに接続できませんでした。再試行してください。" : "Could not reach the server. Please retry."],
            [{ json: { registered: [], skipped: [], errors: [] } }, language === "ja" ? "登録に失敗しました。再試行してください。" : "Registration failed. Please retry."],
        ] as Array<[DiscoveryReply, string]>) {
            const before = setup.payloads.length;
            await setup.row(4).locator(".register-device").click();
            await expect.poll(() => setup.payloads.length).toBe(before + 1);
            setup.reply(reply);
            await expect(setup.row(4).getByRole("alert")).toHaveText(message);
            await expect(setup.row(4).locator(".register-device")).toBeEnabled();
        }
        await expect(page.locator("body")).not.toContainText("scan-only-secret");
        await expect(page.locator("body")).not.toContainText("internal-stack-marker");
        await expect(page.locator("body")).not.toContainText("private-token");
        if (width === 375) {
            expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBeTruthy();
            expect(await page.locator(".discovery-results-scroll").evaluate(element => element.scrollWidth > element.clientWidth)).toBeTruthy();
        }
        await setup.row(4).screenshot({ path: `test-results/discovery-row-register-${width}-${language}-${theme}.png` });
        await setup.row(4).locator(".register-device").click();
        await expect.poll(() => setup.payloads.length).toBe(6);
        setup.reply({ json: { registered: [], skipped: [setup.devices[4].ip], errors: [] } });
        await expect(page.locator("#results-body .badge-registered")).toHaveCount(5);
        await expect(page.locator("#select-all")).toBeDisabled();
        await expect(page.locator("#select-all")).not.toBeChecked();
        await expect(page.locator("#register-action")).toBeHidden();
        await expect(page.locator("#results-section .operation-progress")).toHaveCount(0);
        expect(setup.scans).toBe(1);
        const known = await page.evaluate(() => [...window.DISCOVERY_EXISTING!]);
        expect(known.sort()).toEqual(setup.devices.map(device => device.ip).sort());
        await page.locator("#lang-select").selectOption(language === "ja" ? "en" : "ja");
        await expect(setup.row(4).locator(".badge-registered")).toHaveText(language === "ja" ? "Registered" : "登録済み");
    });
}

test("discovery registration ignores stale row failures after rescan and keeps scan credentials", async ({ page }) => {
    await page.addInitScript(() => { localStorage.setItem("tracepulse-onboarding-v1", "done"); localStorage.setItem("tracepulse-lang", "en"); });
    const setup = await registrationFixture(page);
    await setup.row(1).locator(".register-device").click();
    await expect.poll(() => setup.payloads.length).toBe(1);
    const replacement = [{ ip: setup.devices[1].ip, name: "rescanned-device", status: "online" }];
    await setup.scan(replacement, "new-scan-secret");
    setup.reply({ json: { registered: [], skipped: [], errors: [`${setup.devices[1].ip}: internal-stack-marker`] } });
    await expect(page.locator("#results-body .register-device")).toBeEnabled();
    await expect(page.locator("#results-body [role=alert]")).toHaveCount(0);
    await expect(page.locator("#results-body")).toContainText("rescanned-device");
    await page.locator(".register-device").click();
    await expect.poll(() => setup.payloads.length).toBe(2);
    expect(setup.payloads[0][0].community).toBe("scan-only-secret");
    expect(setup.payloads[1][0]).toEqual({ ip: replacement[0].ip, name: replacement[0].name, community: "new-scan-secret" });
    setup.reply({ json: { registered: [replacement[0].ip], skipped: [], errors: [] } });
    await expect(page.locator("#results-body .badge-registered")).toBeVisible();
    await setup.scan([setup.devices[2]], "third-scan-secret");
    await page.locator(".register-device").click();
    await expect.poll(() => setup.payloads.length).toBe(3);
    await setup.scan([setup.devices[3]], "fourth-scan-secret");
    setup.reply({ json: { registered: [setup.devices[2].ip], skipped: [], errors: [] } });
    await expect(page.locator(".register-device")).toBeEnabled();
    await expect(page.locator("#results-body .badge-registered")).toHaveCount(0);
    expect(await page.evaluate(ip => window.DISCOVERY_EXISTING!.has(ip), setup.devices[2].ip)).toBeTruthy();
    expect(await page.evaluate(ip => window.DISCOVERY_EXISTING!.has(ip), setup.devices[3].ip)).toBeFalsy();
});

test("discovery registration rejects malformed success responses and clears progress", async ({ page }) => {
    await page.addInitScript(() => { localStorage.setItem("tracepulse-onboarding-v1", "done"); localStorage.setItem("tracepulse-lang", "en"); });
    const setup = await registrationFixture(page);
    await setup.row(1).locator(".register-device").click();
    await expect.poll(() => setup.payloads.length).toBe(1);
    setup.reply({ json: { registered: "internal-stack-marker", skipped: [], errors: [] } });
    await expect(setup.row(1).getByRole("alert")).toHaveText("Registration failed. Please retry.");
    await expect(setup.row(1).locator(".register-device")).toBeEnabled();
    await expect(page.locator("#results-section .operation-progress")).toHaveCount(0);
    await expect(setup.row(1).locator(".badge-registered")).toHaveCount(0);
});

test.describe("interface error breakdown overlay", () => {
    test.use({ hasTouch: true });
    test.beforeEach(async ({ page, request }) => {
        page.on("pageerror", error => { throw error; });
        await page.route("**/static/js/device-detail.js*", route => route.fulfill({ path: "frontend/dist/device-detail.js", contentType: "application/javascript" }));
        await request.post("/api/discovery/register", { data: [{ ip: "192.0.2.250", name: "tooltip-test", community: "public" }] });
    });
    test.afterEach(async ({ request }) => { await request.delete("/api/device/192.0.2.250"); });
    for (const [width, height] of [[1280, 900], [375, 667], [800, 240]]) {
        test(`all breakdown rows remain accessible outside clipped tables at ${width}x${height}`, async ({ page }) => {
            await page.setViewportSize({ width, height });
            await page.addInitScript(() => {
                localStorage.setItem("tracepulse-onboarding-v1", "done");
                localStorage.setItem("tracepulse-lang", "ja");
            });
            const interfaces = Array.from({ length: 4 }, (_, index) => ({
                if_index: index + 1, if_name: `Gi0/${index + 1}`, link_status: "up", sampled_at: "2026-11-02T00:00:00Z",
                metrics: { in_errors: 12, out_errors: 3, in_errors_delta: 1, out_errors_delta: 2, in_discards_delta: 3, out_discards_delta: 4, late_collisions_delta: 5 },
                error_breakdown: { fcs_errors_delta: 6, alignment_errors_delta: 7, frame_too_longs_delta: 8, internal_mac_receive_errors_delta: 9 },
            }));
            await page.route("**/api/device/192.0.2.250", route => route.fulfill({ json: { ip: "192.0.2.250", name: "tooltip-test", status: "online", interfaces, metrics: [], if_series: [], alerts: [], spikes: [] } }));
            for (const theme of ["dark", "light"]) {
                await page.goto("/device/192.0.2.250");
                await page.locator("#theme-select").selectOption(theme);
                await expect(page.locator("#if-tbody tr")).toHaveCount(4);
                const wrapper = page.locator(".interface-table-scroll");
                await wrapper.evaluate(element => {
                    Object.assign((element as HTMLElement).style, { overflow: "auto", height: "180px", maxWidth: "360px", marginLeft: "auto" });
                });
                for (const row of [1, 3]) {
                    const trigger = page.locator("#if-tbody tr").nth(row).locator(".err-cell").first();
                    await trigger.scrollIntoViewIfNeeded();
                    await trigger.hover();
                    const popup = page.locator("body > .err-pop.is-open");
                    await expect(popup).toBeVisible();
                    await expect(popup.locator(".err-pop-row")).toHaveCount(9);
                    const bounds = (await popup.boundingBox())!;
                    expect(bounds.x).toBeGreaterThanOrEqual(7);
                    expect(bounds.y).toBeGreaterThanOrEqual(7);
                    expect(bounds.x + bounds.width).toBeLessThanOrEqual(width - 7);
                    expect(bounds.y + bounds.height).toBeLessThanOrEqual(height - 7);
                    const last = popup.locator(".err-pop-row").last();
                    await last.scrollIntoViewIfNeeded();
                    await expect(last.locator(".v")).toHaveText("+9");
                    expect(await last.evaluate(element => {
                        const rect = element.getBoundingClientRect();
                        return element.contains(document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2));
                    })).toBeTruthy();
                    await popup.screenshot({ path: `test-results/interface-errors-${width}-${height}-${theme}-row${row}.png` });
                    await page.keyboard.press("Escape");
                    await expect(popup).toHaveCount(0);
                    await trigger.focus();
                    await expect(popup).toBeVisible();
                    await page.keyboard.press("Escape");
                    await trigger.press("Enter");
                    await expect(popup).toBeVisible();
                    await page.keyboard.press("Escape");
                    await trigger.tap();
                    await expect(popup).toBeVisible();
                    await page.evaluate(() => window.renderInterfaces?.([]));
                    await expect(popup).toHaveCount(0);
                    await page.evaluate(data => window.renderInterfaces?.(data), interfaces);
                }
            }
        });
    }
});

for (const [width, theme, language] of [[1280, "light", "en"], [375, "dark", "ja"]] as const) {
    test(`device detail loading and refresh show elapsed progress at ${width} ${theme} ${language}`, async ({ page, request }) => {
        const ip = "192.0.2.245";
        await request.post("/api/discovery/register", { data: [{ ip, name: "detail-progress", community: "public" }] });
        try {
            await page.setViewportSize({ width, height: 900 });
            await page.addInitScript(({ language, theme }) => {
                localStorage.setItem("tracepulse-onboarding-v1", "done");
                localStorage.setItem("tracepulse-lang", language);
                localStorage.setItem("tracepulse-theme", theme);
            }, { language, theme });
            await page.route("**/api/settings", route => route.fulfill({ json: { polling: { interval_seconds: 600 } } }));
            let finish: ((success: boolean) => void) | undefined;
            let requests = 0;
            await page.route(`**/api/device/${ip}`, async route => {
                requests++;
                const success = await new Promise<boolean>(resolve => { finish = resolve; });
                await route.fulfill(success ? { json: { ip, name: "detail-progress", status: "online", interfaces: [], metrics: [], hardware_sensors: [], alerts: [], spikes: [] } } : { status: 500, json: { error: "Detail failed" } });
            });
            await page.goto(`/device/${ip}`);
            await page.locator("#theme-select").selectOption(theme);
            const progress = page.locator("#device-refresh").locator("..").locator(".operation-progress");
            await expect(progress).toBeInViewport();
            await expect(progress.getByRole("status")).toContainText(language === "ja" ? "デバイス情報を読み込んでいます" : "Loading device details");
            await expect(progress.getByRole("status")).toHaveText(language === "ja" ? /経過 [1-9]\d*秒/ : /[1-9]\d*s elapsed/);
            await page.evaluate(() => { window.loadDeviceDetail?.(); window.loadDeviceDetail?.(); });
            expect(requests).toBe(1);
            await page.locator("#device-refresh").locator("..").screenshot({ path: `test-results/device-loading-progress-${width}-${theme}.png` });
            finish!(true);
            await expect(progress).toHaveCount(0);
            await expect(page.locator("#dev-title")).toHaveText(`detail-progress (${ip})`);
            await page.evaluate(() => window.loadDeviceDetail?.());
            await expect(progress).toBeVisible();
            await expect(progress.getByRole("status")).toContainText(language === "ja" ? "デバイス情報を更新しています" : "Updating device details");
            await expect(page.locator("#dev-title")).toHaveText(`detail-progress (${ip})`);
            await expect.poll(() => requests).toBe(2);
            finish!(false);
            await expect(progress).toHaveCount(0);
            await expect(page.locator("#dev-title")).toContainText("HTTP 500");
            await page.evaluate(() => window.loadDeviceDetail?.());
            await expect.poll(() => requests).toBe(3);
            finish!(true);
            await expect(progress).toHaveCount(0);
            await expect(page.locator("#dev-title")).toHaveText(`detail-progress (${ip})`);
        } finally { await request.delete(`/api/device/${ip}`); }
    });
}

for (const [width, language, theme] of [[1280, "en", "light"], [375, "ja", "dark"]] as const) {
    test(`remaining discovery registration progress at ${width} ${language}`, async ({ page }) => {
        await page.setViewportSize({ width, height: 900 });
        await page.addInitScript(lang => {
            localStorage.setItem("tracepulse-onboarding-v1", "done");
            localStorage.setItem("tracepulse-lang", lang);
        }, language);
        const devices = [1, 2, 3].map(id => ({ ip: `198.51.100.${id}`, name: `register-${id}`, status: "online" }));
        await page.route("**/api/discovery/scan", route => route.fulfill({ json: { job_id: "progress", total: 3 } }));
        await page.route("**/api/discovery/scan/progress", route => route.fulfill({ json: { status: "done", scanned: 3, total: 3, elapsed: 1, devices } }));
        let finish: ((success: boolean) => void) | undefined;
        let requests = 0;
        await page.route("**/api/discovery/register", async route => {
            requests++;
            expect(route.request().postDataJSON()).toHaveLength(3);
            const success = await new Promise<boolean>(resolve => { finish = resolve; });
            await route.fulfill(success ? { json: { registered: devices.map(device => device.ip), skipped: [], errors: [] } } : { status: 500, json: { error: "Registration failed" } });
        });
        await page.goto("/discovery");
        await page.locator("#theme-select").selectOption(theme);
        await page.locator("#cidr").fill("198.51.100.0/29");
        await page.locator("#scan-btn").click();
        await expect(page.locator("#results-body tr")).toHaveCount(3);
        await page.locator("#select-all").check();
        await page.locator("#register-btn").click();
        const progress = page.locator("#results-section .operation-progress");
        await expect(progress).toBeInViewport();
        await expect(progress.getByRole("status")).toHaveText(language === "ja" ? /3台の機器を登録.*経過 [1-9]\d*秒/ : /Registering 3 devices.*[1-9]\d*s elapsed/);
        await expect(page.locator("#register-btn")).toBeDisabled();
        await expect(page.locator("#select-all")).toBeDisabled();
        await expect(page.locator(".row-cb").first()).toBeDisabled();
        await page.locator("#register-btn").dispatchEvent("click");
        expect(requests).toBe(1);
        await progress.screenshot({ path: `test-results/discovery-register-progress-${width}-${theme}.png` });
        finish!(false);
        await expect(progress).toHaveCount(0);
        await expect(page.locator("#register-result")).toBeVisible();
        await expect(page.locator("#register-result")).toContainText(language === "ja" ? "失敗: 3台" : "Failed: 3");
        await expect(page.locator(".registration-error").first()).toHaveText(language === "ja" ? "登録に失敗しました。再試行してください。" : "Registration failed. Please retry.");
        await expect(page.locator("#register-btn")).toBeEnabled();
        await expect(page.locator("#select-all")).toBeEnabled();
        await page.locator("#register-btn").click();
        await expect.poll(() => requests).toBe(2);
        finish!(true);
        await expect(progress).toHaveCount(0);
        await expect(page.locator("#register-result")).toHaveClass("reg-success");
        await expect(page.locator("#register-result")).toContainText(language === "ja" ? "登録: 3台" : "Registered: 3");
        await expect(page.locator("#results-body .badge-registered")).toHaveCount(3);
        await expect(page.locator("#select-all")).toBeDisabled();
    });
}

for (const [width, language, theme] of [[1280, "en", "light"], [375, "ja", "dark"]] as const) {
    test(`monitoring graph time ranges filter all charts at ${width} ${language}`, async ({ page, request }) => {
        const ip = "192.0.2.246";
        await request.post("/api/discovery/register", { data: [{ ip, name: "history-test", community: "public" }] });
        try {
            const actual = await request.get(`/api/device/${ip}/history?range=1h`);
            expect(actual.ok()).toBeTruthy();
            expect((await actual.json()).metrics).toEqual([]);
            expect((await request.get(`/api/device/${ip}/history?range=bad`)).status()).toBe(400);
            await page.setViewportSize({ width, height: 900 });
            await page.addInitScript(lang => {
                localStorage.setItem("tracepulse-onboarding-v1", "done");
                localStorage.setItem("tracepulse-lang", lang);
            }, language);
            await page.route("**/api/settings", route => route.fulfill({ json: { polling: { interval_seconds: 600 }, display: { timezone: "utc" } } }));
            await page.route(`**/api/device/${ip}`, route => route.fulfill({ json: {
                ip, name: "history-test", status: "online", interfaces: [{ if_index: 1, if_name: "Gi0/1", link_status: "up", metrics: { bandwidth_utilization: 77.29 } }],
                metrics: [{ t: "2020-01-01T00:00:00Z", cpu: 99, memory: 99 }], alerts: [], spikes: [],
            } }));
            const queries: URLSearchParams[] = [];
            let fail = false, empty = false, delay = false;
            let finishOld: (() => void) | undefined;
            await page.route(`**/api/device/${ip}/history?*`, async route => {
                const params = new URL(route.request().url()).searchParams;
                queries.push(params);
                const range = params.get("range")!;
                if (delay && range === "6h") await new Promise<void>(resolve => { finishOld = resolve; });
                if (fail) { await route.fulfill({ status: 500, json: { error: "History failed" } }); return; }
                const end = range === "custom" ? params.get("end")! : "2026-01-10T12:00:00Z";
                const start = range === "custom" ? params.get("start")! : new Date(new Date(end).getTime() - ({ "1h": 3600, "6h": 21600, "24h": 86400, "7d": 604800 }[range] || 86400) * 1000).toISOString();
                await route.fulfill({ json: {
                    start, end, max_points_per_series: 600,
                    if_series: empty ? [] : [{ if_index: 1, points: [{ t: start, bw: 7.72, in_err: 1, in_dis: 2 }, { t: end, bw: 77.29, in_err: 4, in_dis: 3 }] }],
                    metrics: empty ? [] : [{ t: start, cpu: 0, memory: 20 }, { t: end, cpu: 50, memory: 60 }],
                } });
            });
            await page.goto(`/device/${ip}`);
            await page.locator("#theme-select").selectOption(theme);
            const controls = page.locator("#monitoring-history-controls");
            const range = page.locator("#monitoring-range");
            await expect(range).toHaveValue("24h");
            for (const id of ["bw-chart", "err-chart", "cpu-chart", "memory-chart"]) await expect(page.locator(`#${id} path`)).toHaveCount(1);
            for (const value of ["1h", "6h", "24h", "7d"]) {
                await range.selectOption(value);
                await expect.poll(() => queries.at(-1)?.get("range")).toBe(value);
                await expect(page.locator("#monitoring-history-progress .operation-progress")).toHaveCount(0);
            }
            await range.selectOption("custom");
            await page.locator("#monitoring-start").fill("2026-01-02T10:00");
            await page.locator("#monitoring-end").fill("2026-01-02T09:00");
            const before = queries.length;
            await controls.locator("[type=submit]").click();
            await expect(page.locator("#monitoring-history-error")).not.toBeEmpty();
            expect(queries).toHaveLength(before);
            await page.locator("#monitoring-end").fill("2026-03-02T10:00");
            await controls.locator("[type=submit]").click();
            expect(queries).toHaveLength(before);
            await page.locator("#monitoring-end").fill("2026-01-02T12:00");
            await controls.locator("[type=submit]").click();
            await expect.poll(() => queries.at(-1)?.get("range")).toBe("custom");
            const query = queries.at(-1)!;
            expect(new Date(query.get("end")!).getTime() - new Date(query.get("start")!).getTime()).toBe(7200000);
            await expect(page.locator("#monitoring-history-error")).toBeEmpty();
            await expect(page.locator("#if-tbody tr td").nth(9)).toHaveText("77.3%");
            await page.locator("#iface-select-1").uncheck();
            await expect(page.locator("#bw-chart path")).toHaveCount(0);
            await page.locator("#iface-select-1").check();
            await expect(page.locator("#bw-chart path")).toHaveCount(1);
            await controls.screenshot({ path: `test-results/monitoring-range-${width}-${theme}.png` });
            expect(await controls.evaluate(element => element.scrollWidth <= element.clientWidth)).toBeTruthy();
            fail = true;
            await page.locator("#monitoring-history-refresh").click();
            await expect(page.locator("#monitoring-history-error")).toContainText("History failed");
            await expect(page.locator("#monitoring-history-progress .operation-progress")).toHaveCount(0);
            fail = false; empty = true;
            await page.locator("#monitoring-history-refresh").click();
            for (const id of ["bw-chart", "err-chart", "cpu-chart", "memory-chart"]) await expect(page.locator(`#${id}`)).toContainText("N/A");
            empty = false; delay = true;
            await range.selectOption("6h");
            await expect.poll(() => typeof finishOld).toBe("function");
            await range.selectOption("1h");
            await expect.poll(() => queries.at(-1)?.get("range")).toBe("1h");
            finishOld!();
            await expect(page.locator("#monitoring-history-period")).toContainText("11:00");
            await expect(range).toHaveValue("1h");
            for (const id of ["bw-chart", "err-chart", "cpu-chart", "memory-chart"]) await expect(page.locator(`#${id} path`)).toHaveCount(1);
        } finally { await request.delete(`/api/device/${ip}`); }
    });
}

test("recent alert heading aligns with table content across themes and viewport sizes", async ({ page }) => {
    await page.addInitScript(() => {
        localStorage.setItem("tracepulse-onboarding-v1", "done");
        localStorage.setItem("tracepulse-lang", "ja");
    });
    await page.route("**/api/devices", route => route.fulfill({ json: [] }));
    await page.goto("/");
    const section = page.locator(".dashboard-recent-alerts");
    const heading = section.getByRole("heading", { name: "最近のアラート", exact: true });
    for (const width of [1280, 375]) {
        await page.setViewportSize({ width, height: 900 });
        for (const theme of ["dark", "light"]) {
            await page.locator("#theme-select").selectOption(theme);
            await expect(heading).toHaveCSS("padding-left", "16px");
            await expect(heading).toHaveCSS("padding-right", "16px");
            const spacing = await section.evaluate(element => {
                const title = element.querySelector("h2")!;
                const header = element.querySelector("th")!;
                const range = document.createRange();
                range.selectNodeContents(title);
                return {
                    top: parseFloat(getComputedStyle(title).paddingTop),
                    titleLeft: range.getBoundingClientRect().left,
                    cellLeft: header.getBoundingClientRect().left + parseFloat(getComputedStyle(header).paddingLeft),
                    sectionLeft: element.getBoundingClientRect().left,
                };
            });
            expect(spacing.top).toBeGreaterThanOrEqual(12);
            expect(spacing.titleLeft - spacing.sectionLeft).toBeGreaterThanOrEqual(16);
            expect(Math.abs(spacing.titleLeft - spacing.cellLeft)).toBeLessThanOrEqual(1);
            await section.screenshot({ path: `test-results/recent-alerts-${width}-${theme}.png` });
        }
    }
});

test("DOM power columns show dBm and bandwidth uses percentage values without rescaling", async ({ page }) => {
    await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
    await page.goto("/");
    await page.evaluate(() => {
        document.body.insertAdjacentHTML("beforeend", "<table><tbody id='if-tbody'></tbody></table>");
    });
    await page.addScriptTag({ url: "/static/js/device-detail.js" });
    await page.addScriptTag({
        content: `TracePulseDeviceDetail.renderInterfaces([
        {if_index:7,if_name:"xe-0/0/0",link_status:"up",tx_power_dbm:-4.2,rx_power_dbm:-3.5,metrics:{bandwidth_utilization:77.29}},
        {if_index:8,if_name:"ge-0/0/1",link_status:"up",tx_power_dbm:null,rx_power_dbm:null,predictive_status:{rx_optical_power_dbm:-5},metrics:{bandwidth_utilization:7.72}},
        {if_index:9,if_name:"weak",link_status:"up",tx_power_dbm:-20,rx_power_dbm:-40,metrics:{bandwidth_utilization:0}},
        {if_index:10,if_name:"zero",link_status:"up",tx_power_dbm:0,rx_power_dbm:-19.99,metrics:{bandwidth_utilization:100}}
    ]);` });
    const rows = page.locator("#if-tbody tr");
    await expect(rows).toHaveCount(4);
    await expect(rows.nth(0).locator("td").nth(9)).toHaveText("77.3%");
    await expect(rows.nth(1).locator("td").nth(9)).toHaveText("7.7%");
    await expect(rows.nth(2).locator("td").nth(9)).toHaveText("0.0%");
    await expect(rows.nth(3).locator("td").nth(9)).toHaveText("100.0%");
    await expect(rows.nth(0).locator("td").nth(10)).toHaveText("-4.2 dBm");
    await expect(rows.nth(0).locator("td").nth(11)).toHaveText("-3.5 dBm");
    await expect(rows.nth(0).locator(".status-warning")).toHaveCount(0);
    await expect(rows.nth(1).locator("td").nth(10)).toHaveText("N/A");
    await expect(rows.nth(1).locator("td").nth(11)).toHaveText("N/A");
    await expect(rows.nth(2).locator(".status-warning")).toHaveCount(2);
    await expect(rows.nth(2).locator("td").nth(11)).toHaveText("-40.0 dBm");
    await expect(rows.nth(2).locator(".status-warning").first()).toHaveCSS("color", "rgb(251, 191, 36)");
    await expect(rows.nth(3).locator("td").nth(10)).toHaveText("0.0 dBm");
    await expect(rows.nth(3).locator(".status-warning")).toHaveCount(0);
    await page.addScriptTag({ content: "TracePulseDeviceDetail.renderInterfaces([]);" });
    await expect(page.locator("#if-tbody td")).toHaveAttribute("colspan", "13");
});

test("vendor hardware states render normal power and critical failures correctly", async ({ page }) => {
    await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
    await page.goto("/");
    await page.evaluate(() => {
        document.body.insertAdjacentHTML("beforeend", "<div id='hardware-status'></div>");
    });
    await page.addScriptTag({ url: "/static/js/device-detail.js" });
    await page.addScriptTag({
        content: `TracePulseDeviceDetail.renderHardwareStatus([
        {name:"Cisco PSU 1",sensor_type:"power",status:2,status_text:"On",status_kind:"normal",is_alarm:false},
        {name:"Cisco PSU 2",sensor_type:"power",status:8,status_text:"Failed",status_kind:"critical",is_alarm:true},
        {name:"Juniper Fan",sensor_type:"fan",status:6,status_text:"Online",status_kind:"normal",is_alarm:false},
        {name:"Unknown FRU",sensor_type:"power",status:1,status_text:"Unknown",status_kind:"warning",is_alarm:false},
        {name:"RE Temperature",sensor_type:"temperature",value:42,unit:"C",status:null,is_alarm:false}
    ]);` });
    const cards = page.locator("#hardware-status .hardware-card");
    await expect(cards).toHaveCount(5);
    await expect(cards.nth(0)).toHaveClass("hardware-card");
    await expect(cards.nth(0)).toContainText("On");
    await expect(cards.nth(1)).toHaveClass("hardware-card crit");
    await expect(cards.nth(1)).toContainText("Failed");
    await expect(cards.nth(2)).toHaveClass("hardware-card");
    await expect(cards.nth(3)).toHaveClass("hardware-card warn");
    await expect(cards.nth(4)).toHaveClass("hardware-card");
    await expect(cards.nth(4)).toContainText("42 C");
});

test("hardware status cards use readable colors in light mode and retain dark mode styling", async ({ page }) => {
    await page.request.post("/api/discovery/register", { data: [{ ip: "192.0.2.251", name: "hardware-color-test", community: "public" }] });
    await page.route("**/api/device/192.0.2.251", route => route.fulfill({
        json: {
            ip: "192.0.2.251", name: "hardware-color-test", status: "online", metrics: [], alerts: [], spikes: [],
            interfaces: ["up", "down", "unknown", "testing", "dormant", "notPresent", "lowerLayerDown"].map((link_status, index) => ({ if_index: index + 1, if_name: `Gi0/${index + 1}`, link_status })),
            hardware_sensors: [
                { index: 1, name: "Power Supply 1", sensor_type: "power", status: 2, status_text: "On", status_kind: "normal", is_alarm: false },
                { index: 2, name: "Fan 1", sensor_type: "fan", status: 1, status_text: "Warning", status_kind: "warning", is_alarm: false },
                { index: 3, name: "Power Supply 2", sensor_type: "power", status: 8, status_text: "Failed", status_kind: "critical", is_alarm: true },
                { index: 4, name: "Temperature", sensor_type: "temperature", value: 42, unit: "C", status: null, is_alarm: false },
            ],
        }
    }));
    await page.goto("/device/192.0.2.251");
    const cards = page.locator("#hardware-status .hardware-card");
    await expect(cards).toHaveCount(4);
    await page.locator("#theme-select").selectOption("light");
    await expect(cards.nth(0)).toHaveCSS("background-color", "rgb(248, 250, 252)");
    await expect(cards.nth(0).locator(".label")).toHaveCSS("color", "rgb(100, 116, 139)");
    await expect(cards.nth(0).locator(".value")).toHaveCSS("color", "rgb(21, 128, 61)");
    await expect(cards.nth(1).locator(".value")).toHaveCSS("color", "rgb(180, 83, 9)");
    await expect(cards.nth(2).locator(".value")).toHaveCSS("color", "rgb(185, 28, 28)");
    await expect(cards.nth(3).locator(".value")).toHaveCSS("color", "rgb(15, 23, 42)");
    await expect(cards.nth(3)).toContainText("42 C");
    const links = page.locator("#if-tbody tr td:nth-child(3) span");
    await expect(links).toHaveText(["up", "down", "unknown", "testing", "dormant", "notPresent", "lowerLayerDown"]);
    for (const [index, color] of ["rgb(21, 128, 61)", "rgb(185, 28, 28)", "rgb(100, 116, 139)", "rgb(180, 83, 9)", "rgb(100, 116, 139)", "rgb(100, 116, 139)", "rgb(185, 28, 28)"].entries()) {
        await expect(links.nth(index)).toHaveCSS("color", color);
    }
    await expect(cards.nth(1)).toHaveCSS("background-color", "rgb(255, 247, 237)");
    await expect(cards.nth(1)).toHaveCSS("border-top-color", "rgb(253, 186, 116)");
    await expect(cards.nth(2)).toHaveCSS("background-color", "rgb(254, 242, 242)");
    await expect(cards.nth(2)).toHaveCSS("border-top-color", "rgb(252, 165, 165)");
    await page.locator("#theme-select").selectOption("dark");
    await expect(cards.nth(0)).toHaveCSS("background-color", "rgb(15, 23, 42)");
    await expect(cards.nth(0).locator(".value")).toHaveCSS("color", "rgb(74, 222, 128)");
    await expect(cards.nth(1).locator(".value")).toHaveCSS("color", "rgb(251, 191, 36)");
    await expect(cards.nth(2).locator(".value")).toHaveCSS("color", "rgb(248, 113, 113)");
    await expect(cards.nth(3).locator(".value")).toHaveCSS("color", "rgb(248, 250, 252)");
    for (const [index, color] of ["rgb(74, 222, 128)", "rgb(248, 113, 113)", "rgb(148, 163, 184)", "rgb(251, 191, 36)", "rgb(148, 163, 184)", "rgb(148, 163, 184)", "rgb(248, 113, 113)"].entries()) {
        await expect(links.nth(index)).toHaveCSS("color", color);
    }
    for (const width of [1280, 375]) {
        await page.setViewportSize({ width, height: 900 });
        for (const theme of ["light", "dark"]) {
            await page.locator("#theme-select").selectOption(theme);
            await page.locator("#hardware-status").screenshot({ path: `test-results/hardware-state-${width}-${theme}.png` });
        }
    }
    await page.request.delete("/api/device/192.0.2.251");
});

test("interface selection uses light-theme colors and retains dark-theme styling", async ({ page }) => {
    await page.request.post("/api/discovery/register", { data: [{ ip: "192.0.2.252", name: "interface-filter-test", community: "public" }] });
    await page.route("**/api/device/192.0.2.252", route => route.fulfill({
        json: {
            ip: "192.0.2.252", name: "interface-filter-test", status: "online", metrics: [], alerts: [], spikes: [],
            interfaces: Array.from({ length: 4 }, (_, index) => ({ if_index: index + 1, if_name: `Gi0/${index + 1}`, link_status: "up", metrics: {} })),
        }
    }));
    await page.goto("/device/192.0.2.252");
    const filter = page.locator("#iface-filter");
    const items = filter.locator(".iface-filter-item");
    await expect(items).toHaveCount(4);
    await page.locator("#theme-select").selectOption("light");
    await expect(filter).toHaveCSS("background-color", "rgb(248, 250, 252)");
    await expect(filter).toHaveCSS("border-top-color", "rgb(203, 213, 225)");
    await expect(items.first()).toHaveCSS("color", "rgb(51, 65, 85)");
    await expect(items.first().locator("input")).toHaveCSS("accent-color", "rgb(20, 143, 131)");
    await page.locator("#theme-select").selectOption("dark");
    await expect(filter).toHaveCSS("background-color", "rgb(15, 23, 42)");
    await expect(items.first()).toHaveCSS("color", "rgb(203, 213, 225)");
    await expect(items.first().locator("input")).toHaveCSS("accent-color", "rgb(56, 189, 248)");
    await page.request.delete("/api/device/192.0.2.252");
});

test("device traffic and error breakdown panels adapt to light mode", async ({ page }) => {
    await page.request.post("/api/discovery/register", { data: [{ ip: "192.0.2.253", name: "light-theme-test", community: "public" }] });
    await page.route("**/api/device/192.0.2.253", route => route.fulfill({
        json: {
            ip: "192.0.2.253", name: "light-theme-test", status: "online", metrics: [], alerts: [], spikes: [],
            interfaces: [{ if_index: 1, if_name: "Gi0/1", link_status: "up", metrics: { in_errors: 1, in_errors_delta: 1 }, error_breakdown: { fcs_errors: 1, fcs_errors_delta: 1 } }],
        }
    }));
    await page.route("**/api/flow/analytics*", route => route.fulfill({
        json: {
            summary: { total_bps: 800, total_pps: 1, active_flows: 1, top_protocol: "TCP" },
            protocols: [{ protocol: "TCP", bytes: 100, percentage: 100, bps: 800, pps: 1 }],
            applications: [], top_sources: [], top_destinations: [], top_talkers: [], timeseries: [],
        }
    }));
    await page.goto("/device/192.0.2.253");
    const trafficCard = page.locator("#traffic-protocols .traffic-card").first();
    await expect(trafficCard).toBeVisible();
    await page.locator("#theme-select").selectOption("light");
    await expect(trafficCard).toHaveCSS("background-color", "rgb(255, 255, 255)");
    await expect(trafficCard.locator("h3").first()).toHaveCSS("color", "rgb(21, 35, 55)");
    await expect(page.locator("#traffic-protocols .traffic-controls button").first()).toHaveCSS("background-color", "rgb(237, 243, 246)");
    await expect(page.locator("#error-breakdown-if")).toHaveCSS("background-color", "rgb(255, 255, 255)");
    await expect(page.locator("#if-tbody .err-pop").first()).toHaveCSS("background-color", "rgb(255, 255, 255)");
    await page.locator("#theme-select").selectOption("dark");
    await expect(trafficCard).toHaveCSS("background-color", "rgba(15, 23, 42, 0.6)");
    await page.request.delete("/api/device/192.0.2.253");
});

test("discovery progress and result banners use light status colors", async ({ page }) => {
    await page.goto("/discovery");
    await page.evaluate(() => {
        const progress = document.querySelector<HTMLElement>("#scan-progress")!;
        progress.style.display = "block";
        const error = document.querySelector<HTMLElement>("#scan-error")!;
        error.style.display = "block";
        const result = document.querySelector<HTMLElement>("#register-result")!;
        result.className = "reg-success";
        result.style.display = "block";
    });
    await page.locator("#theme-select").selectOption("light");
    await expect(page.locator(".progress-bar-track")).toHaveCSS("background-color", "rgb(226, 232, 240)");
    await expect(page.locator("#progress-stats")).toHaveCSS("color", "rgb(51, 65, 85)");
    await expect(page.locator("#scan-error")).toHaveCSS("background-color", "rgb(254, 242, 242)");
    await expect(page.locator("#register-result")).toHaveCSS("background-color", "rgb(240, 253, 244)");
    await page.locator("#theme-select").selectOption("dark");
    await expect(page.locator(".progress-bar-track")).toHaveCSS("background-color", "rgb(15, 23, 42)");
});

test("active diagnostics panel and port dialog adapt to light mode", async ({ page }) => {
    await page.goto("/diagnostics");
    await page.locator("#theme-select").selectOption("light");
    await page.evaluate(() => window.openActiveDiagnostic?.("127.0.0.1", "ping"));
    const panel = page.locator("#active-diagnostic-panel");
    await expect(panel).toHaveCSS("background-color", "rgb(255, 255, 255)");
    await expect(panel.locator("#active-diagnostic-output")).toHaveCSS("color", "rgb(51, 65, 85)");
    await page.evaluate(() => window.openActiveDiagnosticWithPortPrompt?.("127.0.0.1", "port", 80));
    const dialog = page.locator(".diag-port-dialog");
    await expect(dialog).toHaveCSS("background-color", "rgb(255, 255, 255)");
    await expect(dialog.locator("input")).toHaveCSS("color", "rgb(21, 35, 55)");
    await page.locator(".diag-port-dialog [data-cancel]").click();
});

test("numeric template metrics render exact values and explicit errors safely", async ({ page }) => {
    await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
    await page.route("**/api/device/*", (route) => route.fulfill({ json: { interfaces: [], metrics: [] } }));
    await page.goto("/");
    await page.evaluate(() => {
        document.body.insertAdjacentHTML("beforeend", "<div id='template-metrics'></div>");
    });
    await page.addScriptTag({ url: "/static/js/device-detail.js" });
    await expect(page.locator("#template-metrics")).toHaveText("No numeric metrics configured for this device.");
    await page.addScriptTag({
        content: `TracePulseDeviceDetail.renderTemplateMetrics([
        {key:"cpu",label:"<img src=x onerror=alert(1)>",unit:"percent",data_type:"gauge",oid:"1.3.6.1.2.1.1.3.0",value:0,value_text:"0"},
        {key:"packets",label:"Packets",unit:"count",data_type:"counter",oid:"1.3.6.1.2.1.1.3.0",value:9007199254740992,value_text:"9007199254740993"},
        {key:"missing",label:"Missing",unit:"celsius",data_type:"gauge",oid:"1.3.6.1.2.1.1.3.0",value:null,error:"OID <unavailable>"}
    ]);` });
    const metrics = page.locator("#template-metrics");
    await expect(metrics.locator(".hardware-card")).toHaveCount(3);
    await expect(metrics.locator("[data-metric-key='cpu']")).toContainText("0 percent");
    await expect(metrics.locator("[data-metric-key='cpu']")).toContainText("<img src=x onerror=alert(1)>");
    await expect(metrics.locator("img")).toHaveCount(0);
    await expect(metrics.locator("[data-metric-key='packets']")).toContainText("9007199254740993 count");
    await expect(metrics.locator("[data-metric-key='missing']")).toContainText("N/A");
    await expect(metrics.getByRole("alert")).toHaveText("OID <unavailable>");
    await page.addScriptTag({ content: `TracePulseDeviceDetail.renderTemplateMetrics([], "SNMP unavailable");` });
    await expect(metrics.getByRole("alert")).toHaveText("SNMP unavailable");
    await page.addScriptTag({ content: `TracePulseDeviceDetail.renderTemplateMetrics([]);` });
    await expect(metrics).toHaveText("No numeric metrics configured for this device.");
});

test("SNMP settings show the template disclaimer in English and Japanese", async ({ page }) => {
    await page.goto("/settings/snmp");
    const disclaimer = page.locator("[data-i18n='snmp_template_disclaimer']");
    await expect(disclaimer).toContainText("TraceWorks assumes no liability");
    await page.evaluate(() => window.setLanguage?.("ja"));
    await expect(disclaimer).toContainText("EULA");
    await expect(disclaimer).toContainText("無許可アクセス");
});

test("setup guide appears only for empty monitoring and remembers Skip", async ({ page }) => {
    await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
    await page.goto("/");
    const guide = page.locator("#setup-guide");
    await expect(guide).toBeVisible();
    await expect(guide.locator("h2")).toHaveText("Choose Language");
    await expect(guide).not.toContainText("言語");
    await guide.locator("#setup-language").selectOption("en");
    await guide.getByRole("button", { name: "Continue", exact: true }).click();
    await expect(guide.locator("h2")).toHaveText("Welcome to TracePulse");
    expect(await page.evaluate(() => localStorage.getItem("tracepulse-lang"))).toBe("en");
    const bounds = await guide.boundingBox();
    expect(bounds!.x + bounds!.width / 2).toBeCloseTo(page.viewportSize()!.width / 2, 0);
    expect(bounds!.y + bounds!.height / 2).toBeCloseTo(page.viewportSize()!.height / 2, 0);
    await guide.getByRole("button", { name: "Skip", exact: true }).click();
    await expect(guide).not.toBeVisible();
    await expect(page.locator(".setup-launch")).toBeHidden();
    expect(await page.evaluate(() => localStorage.getItem("tracepulse-onboarding-v1"))).toBe("done");
    await page.reload();
    await expect(guide).not.toBeVisible();
    await expect(page.locator(".setup-launch")).toBeHidden();
    await page.goto("/settings");
    const launch = page.getByRole("button", { name: "Setup Guide", exact: true });
    await launch.click();
    await expect(guide).toBeVisible();
    await expect(guide.locator("#setup-language")).toHaveCount(0);
    await expect(guide.locator("h2")).toHaveText("Welcome to TracePulse");
    await page.keyboard.press("Escape");
    await expect(guide).not.toBeVisible();
    await expect(launch).toBeFocused();
});

test("setup guide does not interrupt configured devices or failed initial checks", async ({ page }) => {
    await page.route("**/api/devices", (route) => route.fulfill({ json: [{ ip: "192.0.2.1" }] }));
    await page.goto("/");
    await expect(page.locator(".setup-launch")).toBeVisible();
    await expect(page.locator("#setup-guide")).not.toBeVisible();
    await page.route("**/api/devices", (route) => route.fulfill({ status: 500, json: { error: "unavailable" } }));
    await page.reload();
    await expect(page.locator(".setup-launch")).toBeVisible();
    await expect(page.locator("#setup-guide")).not.toBeVisible();
    await page.getByRole("button", { name: "Setup Guide", exact: true }).click();
    await expect(page.locator("#setup-guide h2")).toHaveText("Welcome to TracePulse");
    await expect(page.locator("#setup-language")).toHaveCount(0);
});

test("setup guide discovers and registers only selected devices then finishes", async ({ page }) => {
    let registered: unknown;
    await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
    await page.route("**/api/discovery/scan", async (route) => {
        const body = new URLSearchParams(route.request().postData() || "");
        expect(body.get("cidr")).toBe("192.0.2.0/24");
        expect(body.get("community")).toBe("test-secret");
        expect(body.get("max_hosts")).toBe("254");
        await route.fulfill({ json: { job_id: "setup-test" } });
    });
    await page.route("**/api/discovery/scan/setup-test", (route) => route.fulfill({ json: { status: "done", scanned: 254, total: 254, devices: [{ ip: "192.0.2.1", name: "<router>" }, { ip: "192.0.2.2", name: "switch" }] } }));
    await page.route("**/api/discovery/register", async (route) => {
        registered = route.request().postDataJSON();
        await route.fulfill({ json: { registered: ["192.0.2.1"], skipped: [], errors: [] } });
    });
    await page.goto("/");
    const guide = page.locator("#setup-guide");
    await guide.locator("#setup-language").selectOption("en");
    await guide.getByRole("button", { name: "Continue", exact: true }).click();
    await guide.getByRole("button", { name: "Next", exact: true }).click();
    await guide.locator("#setup-cidr").fill("192.0.2.0/24");
    await guide.locator("#setup-community").fill("test-secret");
    await guide.getByRole("button", { name: "Discover", exact: true }).click();
    await expect(guide.locator(".setup-device")).toHaveCount(2);
    await expect(guide.locator(".setup-device").first()).toContainText("<router>");
    await guide.locator(".setup-device input").nth(1).uncheck();
    await guide.getByRole("button", { name: "Register selected devices" }).click();
    await expect(guide.locator("#setup-status")).toContainText("Monitoring starts automatically");
    expect(registered).toEqual([{ ip: "192.0.2.1", name: "<router>", community: "test-secret" }]);
    await guide.getByRole("button", { name: "Next", exact: true }).click();
    await expect(guide).toContainText("Outbound notifications are not available in Community edition");
    await guide.getByRole("button", { name: "Finish", exact: true }).click();
    await expect(page.locator(".setup-launch")).toBeHidden();
    await page.reload();
    await expect(guide).not.toBeVisible();
    await expect(page.locator(".setup-launch")).toBeHidden();
    expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain("test-secret");
});

test("setup guide validates ranges and recovers from discovery and registration failures", async ({ page }) => {
    let attempts = 0;
    await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
    await page.route("**/api/discovery/scan", (route) => {
        attempts++;
        return route.fulfill(attempts === 1 ? { status: 500, json: { error: "scan unavailable" } } : { json: { job_id: "retry" } });
    });
    await page.route("**/api/discovery/scan/retry", (route) => route.fulfill({ json: { status: "done", devices: [{ ip: "192.0.2.1" }] } }));
    let registrations = 0;
    await page.route("**/api/discovery/register", (route) => {
        registrations++;
        return route.fulfill(registrations === 1 ? { json: { registered: [], skipped: [], errors: ["device limit reached"] } } : { json: { registered: ["192.0.2.1"], skipped: [], errors: [] } });
    });
    await page.goto("/");
    const guide = page.locator("#setup-guide");
    await guide.locator("#setup-language").selectOption("en");
    await guide.getByRole("button", { name: "Continue", exact: true }).click();
    await guide.getByRole("button", { name: "Next", exact: true }).click();
    await guide.locator("#setup-cidr").fill("999.0.0.0/24");
    await guide.getByRole("button", { name: "Discover", exact: true }).click();
    await expect(guide.getByRole("alert")).toContainText("valid IPv4 CIDR");
    expect(attempts).toBe(0);
    await guide.locator("#setup-cidr").fill("192.0.2.0/24");
    await guide.getByRole("button", { name: "Discover", exact: true }).click();
    await expect(guide.getByRole("alert")).toContainText("scan unavailable");
    await guide.getByRole("button", { name: "Discover", exact: true }).click();
    await expect(guide.locator(".setup-device")).toHaveCount(1);
    await guide.getByRole("button", { name: "Register selected devices" }).click();
    await expect(guide.getByRole("alert")).toContainText("device limit reached");
    await guide.getByRole("button", { name: "Register selected devices" }).click();
    await expect(guide.locator("#setup-status")).toContainText("Monitoring starts automatically");
});

test("setup guide can be skipped during polling without stale results", async ({ page }) => {
    let polls = 0;
    await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
    await page.route("**/api/discovery/scan", (route) => route.fulfill({ json: { job_id: "running" } }));
    await page.route("**/api/discovery/scan/running", (route) => { polls++; return route.fulfill({ json: { status: "running", scanned: 1, total: 254 } }); });
    await page.goto("/");
    const guide = page.locator("#setup-guide");
    await guide.locator("#setup-language").selectOption("en");
    await guide.getByRole("button", { name: "Continue", exact: true }).click();
    await guide.getByRole("button", { name: "Next", exact: true }).click();
    await guide.locator("#setup-cidr").fill("192.0.2.0/24");
    await guide.getByRole("button", { name: "Discover", exact: true }).click();
    await expect(guide.locator("#setup-status")).toContainText("1 / 254");
    await guide.getByRole("button", { name: "Skip", exact: true }).click();
    await expect(page.locator(".setup-launch")).toBeHidden();
    await page.goto("/settings");
    await page.getByRole("button", { name: "Setup Guide", exact: true }).click();
    await expect(guide.locator("h2")).toHaveText("Welcome to TracePulse");
    await page.clock.install();
    await page.clock.fastForward(5000);
    expect(polls).toBe(1);
    await expect(guide.locator("h2")).toHaveText("Welcome to TracePulse");
});

test("setup guide supports Japanese and small viewports with keyboard focus", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
    await page.goto("/");
    const guide = page.locator("#setup-guide");
    await expect(page.locator("html")).toHaveAttribute("lang", "en");
    await expect(guide.locator("#setup-language")).toBeFocused();
    await expect(guide.locator("h2")).toHaveText("Choose Language");
    await expect(guide).not.toContainText("言語");
    await guide.getByRole("button", { name: "Continue", exact: true }).click();
    await expect(guide.locator("h2")).toHaveText("Choose Language");
    await guide.locator("#setup-language").selectOption("ja");
    await guide.getByRole("button", { name: "Continue", exact: true }).click();
    await expect(page.locator("html")).toHaveAttribute("lang", "ja");
    await expect(page.locator("nav")).toContainText("ダッシュボード");
    await expect(page.locator("#dash-tbody")).toContainText("探索");
    expect(await page.evaluate(() => localStorage.getItem("tracepulse-lang"))).toBe("ja");
    await expect(guide.locator("h2")).toHaveText("TracePulseへようこそ");
    await expect(guide.getByRole("button", { name: "戻る", exact: true })).toBeVisible();
    await guide.getByRole("button", { name: "戻る", exact: true }).click();
    await expect(guide.locator("h2")).toHaveText("Choose Language");
    await expect(guide.getByLabel("Language", { exact: true })).toBeVisible();
    await expect(guide.getByRole("button", { name: "Continue", exact: true })).toBeVisible();
    await guide.locator("#setup-language").selectOption("en");
    await guide.getByRole("button", { name: "Continue", exact: true }).click();
    await expect(page.locator("html")).toHaveAttribute("lang", "en");
    await expect(guide.locator("h2")).toHaveText("Welcome to TracePulse");
    await guide.getByRole("button", { name: "Back", exact: true }).click();
    await guide.locator("#setup-language").selectOption("ja");
    await guide.getByRole("button", { name: "Continue", exact: true }).click();
    await expect(page.locator("html")).toHaveAttribute("lang", "ja");
    await expect(guide.locator("h2")).toHaveText("TracePulseへようこそ");
    await guide.getByRole("button", { name: "次へ", exact: true }).click();
    await page.keyboard.press("Tab");
    await expect(guide.locator("#setup-cidr")).toBeFocused();
    await expect(guide.locator("#setup-community")).toHaveAttribute("type", "password");
    const bounds = await guide.boundingBox();
    expect(bounds!.x).toBeGreaterThanOrEqual(0);
    expect(bounds!.y).toBeGreaterThan(0);
    expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(375);
    expect(await guide.evaluate((element) => element.scrollWidth <= element.clientWidth)).toBeTruthy();
    await guide.getByRole("button", { name: "登録せずに次へ", exact: true }).click();
    await guide.getByRole("link", { name: "アラート設定へ" }).click();
    await expect(page).toHaveURL(/\/settings$/);
    await expect(guide).not.toBeVisible();
    await page.getByRole("button", { name: "セットアップガイド", exact: true }).click();
    await expect(guide.locator("h2")).toHaveText("TracePulseへようこそ");
    await expect(guide.locator("#setup-language")).toHaveCount(0);
});

test("setup guide language selection can be dismissed without changing the language", async ({ page }) => {
    await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
    await page.goto("/");
    const guide = page.locator("#setup-guide");
    await expect(guide.locator("#setup-language")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(guide).not.toBeVisible();
    expect(await page.evaluate(() => localStorage.getItem("tracepulse-lang"))).toBeNull();
    await page.reload();
    await expect(page.locator(".setup-launch")).toBeHidden();
    await expect(guide).not.toBeVisible();
    await page.goto("/settings");
    await page.getByRole("button", { name: "Setup Guide", exact: true }).click();
    await expect(guide.locator("h2")).toHaveText("Welcome to TracePulse");
    await expect(guide.locator("#setup-language")).toHaveCount(0);
});

test("setup guide closes a manual dashboard launch without leaving focus on a hidden button", async ({ page }) => {
    await page.route("**/api/devices", (route) => route.fulfill({ json: [{ ip: "192.0.2.1" }] }));
    await page.goto("/");
    await page.getByRole("button", { name: "Setup Guide", exact: true }).click();
    await page.keyboard.press("Escape");
    await expect(page.locator(".setup-launch")).toBeHidden();
    await expect(page.locator("main h1")).toBeFocused();
    await page.reload();
    await expect(page.locator(".setup-launch")).toBeHidden();
});

test("dashboard loads compiled frontend assets without console errors", async ({ page }) => {
    const consoleErrors: string[] = [];
    page.on("console", (message) => {
        if (message.type() === "error") consoleErrors.push(message.text());
    });

    const response = await page.goto("/");
    expect(response?.ok()).toBeTruthy();
    await expect(page.locator("body")).toHaveAttribute("data-title-key", "dashboard_title");
    await expect(page.locator("#dash-table")).toBeVisible();
    await expect(page.locator("nav .brand")).toHaveAttribute("href", "/");
    await expect(page.locator("nav .brand")).toHaveAttribute("aria-label", "TracePulse");
    await expect(page.locator("nav .brand-logo-dark")).toHaveAttribute("src", "/static/brand/tracepulse-logo.svg?v=4");
    await expect(page.locator("nav .brand-logo-light")).toHaveAttribute("src", "/static/brand/tracepulse-logo-light.svg?v=4");
    await expect(page.locator("nav .brand-logo-dark")).toHaveCSS("height", "30px");
    await expect(page.locator("nav .brand-name")).toHaveText("TracePulse");
    await expect(page.locator("nav .brand-name-trace")).toHaveCSS("color", "rgb(0, 210, 255)");
    await expect(page.locator("nav .brand-name-pulse")).toHaveCSS("color", "rgb(16, 185, 129)");
    await expect(page.locator("nav .brand")).toHaveCSS("gap", "10px");
    await expect(page.locator("nav .brand-name")).toHaveCSS("font-size", "19px");
    await expect(page.locator("nav .brand-name")).toHaveCSS("font-weight", "700");
    await page.evaluate(() => { document.documentElement.dataset.theme = "dark"; });
    await expect(page.locator("nav .brand-logo-dark")).toBeVisible();
    await expect(page.locator("nav .brand-logo-light")).toBeHidden();
    await expect(page.locator("nav .brand-logo-dark")).toHaveCSS("filter", "none");
    await page.evaluate(() => { document.documentElement.dataset.theme = "light"; });
    await expect(page.locator("nav .brand-logo-dark")).toBeHidden();
    await expect(page.locator("nav .brand-logo-light")).toBeVisible();
    await expect(page.locator("nav .brand-logo-light")).toHaveCSS("filter", "none");
    await expect(page.locator("nav .brand-name-trace")).toHaveCSS("color", "rgb(30, 41, 59)");
    await expect(page.locator("nav .brand-name-pulse")).toHaveCSS("color", "rgb(0, 184, 148)");
    await expect(page.locator("link[rel='icon'][type='image/svg+xml']")).toHaveAttribute("href", "/favicon.svg?v=3");
    await expect(page.locator("link[rel='icon'][media='(prefers-color-scheme: light)']")).toHaveAttribute("href", "/favicon.ico?v=3");
    await expect(page.locator("link[rel='icon'][media='(prefers-color-scheme: dark)']")).toHaveAttribute("href", "/favicon-dark.ico?v=3");
    await expect(page.locator("link[rel='apple-touch-icon']")).toHaveAttribute("href", "/apple-touch-icon.png?v=3");
    await expect(page.locator("script[src^='/static/js/dashboard.js']")).toHaveCount(1);
    await expect(page.locator("script[src^='/static/js/i18n.js']")).toHaveCount(1);
    expect((await page.request.get("/favicon.ico?v=3")).headers()["content-type"]).toContain("image/x-icon");
    expect((await page.request.get("/favicon-dark.ico?v=3")).headers()["content-type"]).toContain("image/x-icon");
    const svgFavicon = await page.request.get("/favicon.svg");
    expect(svgFavicon.headers()["content-type"]).toContain("image/svg+xml");
    expect(await svgFavicon.text()).toContain("prefers-color-scheme: dark");
    expect((await page.request.get("/apple-touch-icon.png")).headers()["content-type"]).toContain("image/png");
    expect((await page.request.get("/static/brand/tracepulse-logo.svg")).headers()["content-type"]).toContain("image/svg+xml");
    const lightLogo = await page.request.get("/static/brand/tracepulse-logo-light.svg");
    expect(lightLogo.headers()["content-type"]).toContain("image/svg+xml");
    expect(await lightLogo.text()).toContain("#00B894");
    expect(consoleErrors).toEqual([]);
});

test("favicon assets contain the new symbol and render in both color schemes", async ({ page }) => {
    await page.goto("/");
    for (const filename of ["favicon.ico", "favicon-dark.ico"]) {
        const response = await page.request.get(`/${filename}?v=3`);
        expect(response.ok()).toBeTruthy();
        const ico = await response.body();
        expect(ico.readUInt16LE(2)).toBe(1);
        expect(ico.readUInt16LE(4)).toBe(3);
        expect([0, 1, 2].map(index => ico.readUInt8(6 + index * 16))).toEqual([16, 32, 48]);
    }
    const touch = await (await page.request.get("/apple-touch-icon.png?v=3")).body();
    expect(touch.readUInt32BE(16)).toBe(180);
    expect(touch.readUInt32BE(20)).toBe(180);
    for (const theme of ["light", "dark"] as const) {
        await page.emulateMedia({ colorScheme: theme });
        await page.evaluate(value => window.setTheme?.(value), theme);
        const coloredPixels = await page.evaluate(async theme => {
            const image = new Image();
            image.src = "/favicon.svg?v=3";
            await image.decode();
            const canvas = document.createElement("canvas");
            canvas.width = canvas.height = 64;
            const context = canvas.getContext("2d")!;
            context.drawImage(image, 0, 0, 64, 64);
            const pixels = context.getImageData(0, 0, 64, 64).data;
            const expected = theme === "light" ? [0, 184, 148] : [16, 185, 129];
            let count = 0;
            for (let index = 0; index < pixels.length; index += 4) {
                if (pixels[index] === expected[0] && pixels[index + 1] === expected[1]
                    && pixels[index + 2] === expected[2] && pixels[index + 3] === 255) count++;
            }
            return count;
        }, theme);
        expect(coloredPixels).toBeGreaterThan(0);
    }
});

test("Community logos render in both themes without changing navigation size", async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem("tracepulse-onboarding-v1", "done"));
    await page.goto("/");
    for (const width of [1280, 375]) {
        await page.setViewportSize({ width, height: 900 });
        let brandWidth: number | undefined;
        for (const theme of ["dark", "light"]) {
            await page.evaluate((value) => window.setTheme?.(value), theme);
            const logo = page.locator(`nav .brand-logo-${theme}`);
            await expect(logo).toBeVisible();
            await expect(logo).toHaveCSS("height", width === 375 ? "26px" : "30px");
            await expect.poll(() => logo.evaluate(element => (element as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
            const bounds = await page.locator("nav .brand").boundingBox();
            if (brandWidth === undefined) brandWidth = bounds!.width;
            else expect(bounds!.width).toBe(brandWidth);
            const controls = await page.locator("nav .nav-controls").boundingBox();
            expect(bounds!.x).toBeGreaterThanOrEqual(0);
            expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(controls!.x);
            expect(controls!.x + controls!.width).toBeLessThanOrEqual(width);
        }
    }
});

test("interface diagnostic badges reject untrusted status values", async ({ page }) => {
    await page.goto("/");
    await page.addScriptTag({ url: "/static/js/device-detail.js" });
    const markup = await page.evaluate(() => {
        const tbody = document.createElement("tbody");
        tbody.id = "if-tbody";
        document.body.append(tbody);
        window.renderInterfaces?.([
            { if_index: 1, if_name: "uplink", link_status: "up", health_status: "l1_error" },
            { if_index: 2, if_name: "untrusted", link_status: "up", health_status: "unknown' onmouseover='alert(1)" },
        ]);
        return tbody.innerHTML;
    });
    expect(markup).toContain("diagnostic-badge l1_error");
    expect(markup).toContain("L1 Error");
    expect(markup).toContain("diagnostic-badge down");
    expect(markup).not.toContain("onmouseover");
    expect(markup).not.toContain("diagnostic-badge unknown");
    await expect(page.locator("#if-tbody [onmouseover]")).toHaveCount(0);
    await expect(page.locator("#if-tbody .diagnostic-badge.down")).toHaveText("Down");
});

test("Community settings do not expose notification destinations", async ({ page, request }) => {
    await page.goto("/settings");
    await expect(page.locator(".destination-grid")).toHaveCount(0);
    await expect(page.locator("script[src='/static/js/notifications.js']")).toHaveCount(0);
    expect((await request.get("/static/js/notifications.js")).status()).toBe(404);
    expect((await request.get("/api/notifications")).status()).toBe(404);
});

test("threshold validation shows field errors and a fixed toast at the focused input", async ({ page }) => {
    await page.addInitScript(() => {
        localStorage.setItem("tracepulse-onboarding-v1", "done");
        localStorage.setItem("tracepulse-lang", "ja");
    });
    let saves = 0;
    await page.route("**/api/settings", async route => {
        saves++;
        await route.fulfill({ json: { ok: true } });
    });
    await page.goto("/settings");
    const warning = page.locator("#cpu_warning");
    const critical = page.locator("#cpu_critical");
    const toast = page.locator("#toast-err");
    for (const width of [1280, 375]) {
        await page.setViewportSize({ width, height: 900 });
        for (const theme of ["dark", "light"]) {
            await page.locator("#theme-select").selectOption(theme);
            await page.evaluate(() => window.resetDefaults?.());
            await warning.fill("90");
            await critical.fill("80");
            await critical.press("Tab");
            await expect(warning).toHaveAttribute("aria-invalid", "true");
            await expect(critical).toHaveAttribute("aria-invalid", "true");
            await expect(warning).toHaveCSS("border-top-color", "rgb(239, 68, 68)");
            await expect(page.locator("#cpu_warning-error")).toHaveText("WarningはCritical未満の値を指定してください。");
            await expect(page.locator("#cpu_critical-error")).toHaveText("CriticalはWarningより大きい値を指定してください。");
            await expect(warning).toHaveAttribute("aria-describedby", /cpu_warning-error/);
            const before = saves;
            await page.getByRole("button", { name: "設定を保存", exact: true }).click();
            expect(saves).toBe(before);
            await expect(warning).toBeFocused();
            await expect(warning).toBeInViewport();
            await expect(page.locator("#cpu_warning-error")).toBeInViewport();
            await expect(toast).toHaveText("入力値に誤りがあります。赤枠の項目をご確認ください。");
            await expect(toast).toBeInViewport();
            await expect(page.locator(".settings-toasts")).toHaveCSS("position", "fixed");
            const top = (await toast.boundingBox())!.y;
            await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
            await expect(toast).toBeInViewport();
            expect(Math.abs((await toast.boundingBox())!.y - top)).toBeLessThanOrEqual(1);
            await warning.scrollIntoViewIfNeeded();
            await page.screenshot({ path: `test-results/settings-validation-${width}-${theme}.png`, fullPage: false });
            await warning.fill("70");
            await expect(warning).toHaveAttribute("aria-invalid", "false");
            await expect(critical).toHaveAttribute("aria-invalid", "false");
            await expect(page.locator("#cpu_warning-error")).toBeHidden();
            await warning.fill("0");
            await warning.press("Tab");
            await expect(page.locator("#cpu_warning-error")).toHaveText("0.01以上100以下の%値を、小数第2位までで入力してください。");
            await warning.fill("70");
            await page.getByRole("button", { name: "設定を保存", exact: true }).click();
            await expect(page.locator("#toast-ok")).toBeInViewport();
            await expect(toast).toBeHidden();
            expect(saves).toBe(before + 1);
        }
    }
});

test("metric settings use percentages and validate Warning before saving", async ({ page }) => {
    let payload: any;
    let saves = 0;
    await page.route("**/api/settings", async (route) => {
        saves++;
        payload = route.request().postDataJSON();
        await route.fulfill({ json: { ok: true } });
    });
    await page.goto("/settings");
    await page.evaluate(() => window.resetDefaults?.());
    await expect(page.locator(".metric-thresholds input")).toHaveCount(8);
    await expect(page.locator("#error_rate_warning")).toHaveValue("5");
    await expect(page.locator("#error_rate_critical")).toHaveValue("10");
    await page.locator("#error_rate_warning").fill("1");
    await page.locator("#error_rate_critical").fill("2");
    await page.locator("#cpu_warning").fill("95");
    await page.locator("#cpu_critical").fill("90");
    await page.getByRole("button", { name: "Save Settings", exact: true }).click();
    await expect(page.locator("#toast-err")).toBeVisible();
    expect(saves).toBe(0);
    await page.locator("#cpu_warning").fill("80");
    await page.getByRole("button", { name: "Save Settings", exact: true }).click();
    await expect(page.locator("#toast-ok")).toBeVisible();
    expect(payload.error_rate).toEqual({ warning: 1, critical: 2 });
    expect(payload.cpu).toEqual({ warning: 80, critical: 90 });
    expect(payload).not.toHaveProperty("error_rate_threshold");
    await expect(page.getByRole("heading", { name: "Health Score (Display Only)" })).toBeVisible();
    await page.setViewportSize({ width: 375, height: 667 });
    await page.evaluate(() => window.setLanguage?.("ja"));
    await expect(page.getByRole("heading", { name: "ヘルススコア（表示専用）" })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBeTruthy();
});

test("dashboard health score remains separate from metric alarm status", async ({ page }) => {
    await page.route("**/api/devices", (route) => route.fulfill({
        json: [
            { ip: "192.0.2.1", name: "cpu-alarm", status: "critical", health_score: 67, health_status: "warning" },
            { ip: "192.0.2.2", name: "unknown-metrics", status: "online", health_score: null },
            { ip: "192.0.2.3' onmouseover='alert(1)", name: "router' onmouseover='alert(1)", status: "online' onmouseover='alert(1)", health_score: 50, health_status: "warning' onmouseover='alert(1)" },
        ]
    }));
    await page.goto("/");
    await page.evaluate(() => window.refreshDashboard?.());
    const row = page.locator("#dash-tbody tr").filter({ hasText: "cpu-alarm" });
    await expect(row).toContainText("critical");
    await expect(row).toContainText("67/100");
    await expect(row.getByRole("meter")).toHaveAttribute("aria-valuenow", "67");
    await expect(page.locator("#dash-tbody tr").filter({ hasText: "unknown-metrics" }).getByRole("meter")).toHaveCount(0);
    const untrustedRow = page.locator("#dash-tbody tr").filter({ hasText: "router' onmouseover='alert(1)" });
    await expect(untrustedRow.locator("[onmouseover]")).toHaveCount(0);
    await expect(untrustedRow.locator(".status-unknown")).toHaveCount(3);
    await expect(untrustedRow.locator("button")).toHaveAttribute("data-action-device", "192.0.2.3' onmouseover='alert(1)");
    await expect(untrustedRow.locator("button")).toHaveText("Actions");
});

test("dashboard health meters follow API health status across themes and viewports", async ({ page }) => {
    const statuses = ["online", "warning", "critical", "unknown"];
    await page.route("**/api/devices", route => route.fulfill({
        json: statuses.map((health_status, index) => ({
            ip: `192.0.2.${index + 1}`, name: `health-${health_status}`, status: health_status, health_score: [90, 67, 44, 50][index], health_status,
        }))
    }));
    for (const width of [1280, 375]) {
        await page.setViewportSize({ width, height: 900 });
        await page.goto("/");
        await page.evaluate(() => window.refreshDashboard?.());
        const rows = page.locator("#dash-tbody tr");
        await expect(rows).toHaveCount(4);
        for (const theme of ["light", "dark"]) {
            await page.locator("#theme-select").selectOption(theme);
            const colors = theme === "light" ? ["rgb(21, 128, 61)", "rgb(180, 83, 9)", "rgb(185, 28, 28)", "rgb(100, 116, 139)"] : ["rgb(74, 222, 128)", "rgb(251, 191, 36)", "rgb(248, 113, 113)", "rgb(148, 163, 184)"];
            for (const [index, status] of statuses.entries()) {
                const row = rows.filter({ hasText: `health-${status}` });
                const score = row.locator("td").nth(6);
                await expect(score.locator("span").first()).toHaveCSS("color", colors[index]);
                await expect(score.locator(".health-meter-fill")).toHaveCSS("background-color", colors[index]);
                await expect(row.locator("td").nth(5).locator("span")).toHaveCSS("color", colors[index]);
                const meter = row.getByRole("meter");
                await expect(meter).toHaveAttribute("aria-valuemin", "0");
                await expect(meter).toHaveAttribute("aria-valuemax", "100");
                expect(await meter.evaluate(element => {
                    const fill = element.querySelector<HTMLElement>(".health-meter-fill")!;
                    return Math.abs(fill.getBoundingClientRect().width / element.getBoundingClientRect().width * 100 - Number(element.getAttribute("aria-valuenow"))) < 1;
                })).toBeTruthy();
            }
            await page.locator(".dashboard-table-scroll").screenshot({ path: `test-results/health-states-${width}-${theme}.png` });
        }
    }
});

test("discovery result states distinguish offline warnings and unknown across themes", async ({ page }) => {
    const statuses = ["online", "offline", "warning", "critical", "unknown"];
    await page.route("**/api/discovery/scan", route => route.fulfill({ json: { job_id: "status-test", total: 5 } }));
    await page.route("**/api/discovery/scan/status-test", route => route.fulfill({
        json: {
            status: "done", scanned: 5, total: 5, elapsed: 1,
            devices: statuses.map((status, index) => ({ ip: `192.0.2.${index + 1}`, name: `discovered-${status}`, status })),
        }
    }));
    await page.goto("/discovery");
    await page.locator("#cidr").fill("192.0.2.0/29");
    await page.locator("#scan-btn").click();
    const rows = page.locator("#results-body tr");
    await expect(rows).toHaveCount(5);
    for (const width of [1280, 375]) {
        await page.setViewportSize({ width, height: 900 });
        for (const theme of ["light", "dark"]) {
            await page.locator("#theme-select").selectOption(theme);
            const colors = theme === "light" ? ["rgb(21, 128, 61)", "rgb(185, 28, 28)", "rgb(180, 83, 9)", "rgb(185, 28, 28)", "rgb(100, 116, 139)"] : ["rgb(74, 222, 128)", "rgb(248, 113, 113)", "rgb(251, 191, 36)", "rgb(248, 113, 113)", "rgb(148, 163, 184)"];
            for (const [index, status] of statuses.entries()) {
                const state = rows.filter({ hasText: `discovered-${status}` }).locator("td").nth(3).locator("span");
                await expect(state).toHaveText(status);
                await expect(state).toHaveCSS("color", colors[index]);
            }
        }
    }
});

test("health score legend follows boundaries and language changes", async ({ page }) => {
    await page.goto("/settings");
    await page.evaluate(() => window.setLanguage?.("ja"));
    await page.getByLabel("Warning境界値（Healthyとの境目）", { exact: true }).fill("40");
    await page.getByLabel("Critical境界値（Warningとの境目）", { exact: true }).fill("20");
    await expect(page.locator("#legend-healthy")).toHaveText("40 以上 : Healthy (正常)");
    await expect(page.locator("#legend-warning")).toHaveText("20 〜 39 : Warning (警戒)");
    await expect(page.locator("#legend-critical")).toHaveText("20 未満 : Critical (危険)");
    await expect(page.locator("[data-i18n='score_formula_units']")).toContainText("1%は0.01ではなく1として計算");
    await page.locator("#crit_t").fill("0");
    await expect(page.locator("#legend-critical")).toHaveText("0 未満 : Critical (危険)");
    await page.locator("#crit_t").fill("20");
    await page.evaluate(() => window.setLanguage?.("en"));
    await expect(page.locator("#legend-healthy")).toHaveText("40 or above: Healthy (Normal)");
    await expect(page.locator("#legend-warning")).toHaveText("20–39: Warning (Caution)");
    await expect(page.locator("[data-i18n='score_formula_units']")).toContainText("percentages (0–100)");
    await page.setViewportSize({ width: 375, height: 667 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
    await page.locator("#warn_t").fill("");
    await expect(page.locator("#bar-label")).toBeHidden();
});

test("restarting Ping keeps the latest diagnostic result", async ({ page }) => {
    let requests = 0;
    await page.routeWebSocket("**/ws/diagnostics", (socket) => {
        socket.onMessage(() => {
            requests++;
            socket.send(JSON.stringify({ line: `PING ${requests}: Success - reply in 1ms`, done: true }));
        });
    });
    await page.goto("/");
    await page.addScriptTag({ url: "/static/js/diagnostics.js" });
    const ping = () => page.evaluate(() => (window as any).openActiveDiagnostic("127.0.0.1", "ping"));

    await ping();
    await expect(page.locator("#active-diagnostic-output")).toContainText("PING 1:");
    await ping();
    await expect(page.locator("#active-diagnostic-output")).toContainText("PING 2:");
    await expect(page.locator("#active-diagnostic-output")).not.toContainText("diagnostic connection closed before a response");
});

test("completed basic diagnostics show result cards", async ({ page }) => {
    await page.routeWebSocket("**/ws/diagnostics", (socket) => {
        socket.onMessage((raw) => {
            const request = JSON.parse(String(raw)) as { kind: string };
            const results: Record<string, { line: string; message_type: string; data: Record<string, unknown> }> = {
                ping: { line: "PING 1: Failed - timeout", message_type: "ping_result", data: { target: "127.0.0.1", sent: 5, received: 0, lost: 5, packet_loss_percent: "100%", average_rtt_ms: null, verdict: "NO RESPONSE" } },
                traceroute: { line: "HOP 1: Success - 127.0.0.1 (1ms)", message_type: "traceroute_result", data: { target: "127.0.0.1", hops_probed: 5, responding_hops: 1, verdict: "RESPONSES RECEIVED" } },
                port: { line: "PORT 80: Failed - refused", message_type: "port_result", data: { target: "127.0.0.1:80", status: "UNREACHABLE", message: "Connection refused" } },
            };
            socket.send(JSON.stringify({ ...results[request.kind], done: true }));
        });
    });
    await page.goto("/");
    await page.addScriptTag({ url: "/static/js/diagnostics.js" });
    const run = (kind: string, port?: number) => page.evaluate(({ kind, port }) => (window as any).openActiveDiagnostic("127.0.0.1", kind, port), { kind, port });

    await run("ping");
    await expect(page.locator(".diag-tabs button[data-kind='ping']")).toHaveClass(/active/);
    await expect(page.locator(".diag-tabs button[data-kind='traceroute']")).not.toHaveClass(/active/);
    await expect(page.locator(".diag-result-card h3")).toHaveText("Ping Result");
    await expect(page.locator(".diag-result-card")).toContainText("100%");
    await expect(page.locator(".diag-result-verdict.failed")).toContainText("NO RESPONSE");
    await expect(page.locator(".diag-result-card")).not.toContainText("null");
    await run("traceroute");
    await expect(page.locator(".diag-tabs button[data-kind='traceroute']")).toHaveClass(/active/);
    await expect(page.locator(".diag-tabs button[data-kind='ping']")).not.toHaveClass(/active/);
    await expect(page.locator(".diag-result-card h3")).toHaveText("Traceroute Result");
    await expect(page.locator(".diag-result-card")).toContainText("Responding Hops");
    await run("port", 80);
    await expect(page.locator(".diag-result-card h3")).toHaveText("Port Check Result");
    await expect(page.locator(".diag-result-verdict.failed")).toContainText("UNREACHABLE");
});

test("registered diagnostics show progress and result cards", async ({ page }) => {
    const requestedCounts: number[] = [];
    await page.routeWebSocket("**/ws/diagnostics", (socket) => {
        socket.onMessage((raw) => {
            const request = JSON.parse(String(raw)) as { kind: string; count: number };
            expect(request.kind).toBe("custom_probe");
            requestedCounts.push(request.count);
            socket.send(JSON.stringify({ line: "Probe completed", done: false }));
            socket.send(JSON.stringify({ message_type: "custom_result", data: { target: "127.0.0.1", value: 12.5 }, done: true }));
        });
    });
    await page.goto("/");
    await page.addScriptTag({ path: "frontend/dist/diagnostics.js" });
    await page.evaluate(() => {
        (window as any).registerDiagnosticExtension({
            tabs: [{ kind: "custom_probe", label: "Custom Probe" }],
            resultTitles: { custom_result: "Custom Result" },
            timeouts: { custom_probe: 30_000 },
            counts: { custom_probe: 3 },
            formatValue: (_type: string, key: string, value: unknown) => key === "value" ? `${value}%` : undefined,
        });
        (window as any).openActiveDiagnostic("127.0.0.1", "custom_probe");
    });
    await expect(page.locator(".diag-tabs button[data-kind='custom_probe']")).toBeVisible();
    await expect(page.locator(".diag-result-card h3")).toHaveText("Custom Result");
    await expect(page.locator(".diag-result-card")).toContainText("12.5%");
    await expect(page.locator("#active-diagnostic-output pre")).toContainText("Probe completed");
    expect(requestedCounts).toEqual([3]);
});

test("Ping streams replies from the diagnostics server", async ({ page }) => {
    await page.goto("/");
    await page.addScriptTag({ url: "/static/js/diagnostics.js" });
    await page.evaluate(() => (window as any).openActiveDiagnostic("127.0.0.1", "ping"));
    await expect(page.locator("#active-diagnostic-output")).toContainText("PING 1:");
    await expect(page.locator("#active-diagnostic-output")).toContainText("PING 5:");
    await expect(page.locator(".diag-result-card h3")).toHaveText("Ping Result");
    await expect(page.locator(".diag-result-card")).toContainText("Packet Loss Percent");
});

test("Port Check and Traceroute return result cards from the server", async ({ page, baseURL }) => {
    const tracerouteRequests: Array<{ count: number }> = [];
    page.on("websocket", (socket) => {
        socket.on("framesent", (frame) => {
            const request = JSON.parse(String(frame.payload)) as { kind: string; count: number };
            if (request.kind === "traceroute") tracerouteRequests.push(request);
        });
    });
    await page.goto("/");
    await page.addScriptTag({ url: "/static/js/diagnostics.js" });
    const port = Number(new URL(baseURL!).port || "80");
    await page.evaluate((port) => (window as any).openActiveDiagnostic("127.0.0.1", "port", port), port);
    await expect(page.locator(".diag-result-card h3")).toHaveText("Port Check Result");
    await expect(page.locator(".diag-result-verdict")).toContainText("OPEN");
    await page.evaluate(() => (window as any).openActiveDiagnostic("127.0.0.1", "traceroute"));
    await expect(page.locator("#active-diagnostic-output")).toContainText("HOP 1:");
    await expect(page.locator(".diag-result-card h3")).toHaveText("Traceroute Result");
    await expect(page.locator(".diag-result-card")).toContainText("DESTINATION REACHED");
    await expect(page.locator(".diag-result-verdict.failed")).toHaveCount(0);
    await expect(page.locator(".diag-result-card")).toContainText("Hops Probed1");
    await expect(page.locator("#active-diagnostic-output")).not.toContainText("HOP 2:");
    expect(tracerouteRequests).toHaveLength(1);
    expect(tracerouteRequests[0].count).toBe(30);
});

test("discovery loads TypeScript discovery and topology bundles", async ({ page }) => {
    const consoleErrors: string[] = [];
    page.on("console", (message) => {
        if (message.type() === "error") consoleErrors.push(message.text());
    });

    const response = await page.goto("/discovery");
    expect(response?.ok()).toBeTruthy();
    await expect(page.locator("#cidr")).toBeVisible();
    await expect(page.locator("#topology-canvas")).toHaveCount(1);
    await expect(page.locator("script[src='/static/js/discovery.js']")).toHaveCount(1);
    await expect(page.locator("script[src='/static/js/topology.js']")).toHaveCount(1);
    expect(consoleErrors).toEqual([]);
});

test("topology inspector colors device and interface states across themes and viewports", async ({ page }) => {
    await page.route("**/api/discovery/topology", route => route.fulfill({
        json: {
            seed_ip: "192.0.2.1", edges: [],
            nodes: ["online", "offline", "unknown"].map((status, index) => ({
                id: status, label: `switch-${status}`, ip: `192.0.2.${index + 1}`, status,
                interfaces: ["up", "down", "unknown", "testing"].map((link_status, port) => ({ if_index: port + 1, if_name: `Gi0/${port + 1}`, link_status })),
            })),
        }
    }));
    for (const width of [1280, 375]) {
        await page.setViewportSize({ width, height: 900 });
        await page.goto("/discovery");
        await page.locator("#seed-ip").fill("192.0.2.1");
        await page.locator("#topology-btn").click();
        for (const theme of ["light", "dark"]) {
            await page.locator("#theme-select").selectOption(theme);
            const colors = theme === "light" ? ["rgb(21, 128, 61)", "rgb(185, 28, 28)", "rgb(100, 116, 139)"] : ["rgb(74, 222, 128)", "rgb(248, 113, 113)", "rgb(148, 163, 184)"];
            for (const [index, status] of ["online", "offline", "unknown"].entries()) {
                await page.locator(`[data-node='${status}']`).press("Enter");
                const panel = page.locator("#topology-inspector");
                const deviceState = panel.locator(".inspector-row .inspector-state");
                await expect(deviceState).toHaveText(status);
                await expect(deviceState).toHaveCSS("color", colors[index]);
                const states = panel.locator(".inspector-if .inspector-state");
                await expect(states).toHaveText(["up", "down", "unknown", "testing"]);
                for (const port of [0, 1, 2, 3]) await expect(states.nth(port)).toHaveCSS("color", colors[Math.min(port, 2)]);
                if (status === "online") await panel.screenshot({ path: `test-results/topology-status-${width}-${theme}.png` });
                await page.locator(".inspector-close").click();
            }
        }
    }
});

test("CIDR hints do not move the discovery scan button", async ({ page }) => {
    for (const width of [1280, 700, 375]) {
        await page.setViewportSize({ width, height: 900 });
        await page.goto("/discovery");
        for (const language of ["en", "ja"]) {
            await page.evaluate((lang) => window.setLanguage?.(lang), language);
            const cidr = page.locator("#cidr");
            await cidr.fill("");
            const before = (await page.locator("#scan-btn").boundingBox())!;
            for (const value of ["172.20.20.0/24", "172.20.0.0/16", "invalid", ""]) {
                await cidr.fill(value);
                if (value.endsWith("/24")) await expect(page.locator("#host-hint")).toContainText("254");
                if (value === "invalid" || value === "") await expect(page.locator("#host-hint")).toBeEmpty();
                const after = (await page.locator("#scan-btn").boundingBox())!;
                expect(after.x).toBeCloseTo(before.x, 1);
                expect(after.y).toBeCloseTo(before.y, 1);
                expect(after.width).toBeCloseTo(before.width, 1);
                expect(after.height).toBeCloseTo(before.height, 1);
            }
        }
    }
});

test("flow analytics endpoint returns a valid JSON response", async ({ request }) => {
    const response = await request.get("/api/flow/analytics?window=60s&limit=10");
    expect(response.ok()).toBeTruthy();
    const body = await response.json();
    expect(body).toHaveProperty("window_seconds", 60);
    expect(body).toHaveProperty("summary");
    expect(body).toHaveProperty("protocols");
    expect(body).toHaveProperty("applications");
    expect(body).toHaveProperty("top_sources");
    expect(body).toHaveProperty("top_destinations");
    expect(body).toHaveProperty("top_talkers");
});
