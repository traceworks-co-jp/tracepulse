import { expect, test } from "@playwright/test";

test("DOM power columns show dBm, exact warning threshold, and mixed unsupported ports", async ({ page }) => {
    await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
    await page.goto("/");
    await page.evaluate(() => {
        document.body.insertAdjacentHTML("beforeend", "<table><tbody id='if-tbody'></tbody></table>");
    });
    await page.addScriptTag({ url: "/static/js/device-detail.js" });
    await page.addScriptTag({ content: `TracePulseDeviceDetail.renderInterfaces([
        {if_index:7,if_name:"xe-0/0/0",link_status:"up",tx_power_dbm:-4.2,rx_power_dbm:-3.5},
        {if_index:8,if_name:"ge-0/0/1",link_status:"up",tx_power_dbm:null,rx_power_dbm:null,predictive_status:{rx_optical_power_dbm:-5}},
        {if_index:9,if_name:"weak",link_status:"up",tx_power_dbm:-20,rx_power_dbm:-40},
        {if_index:10,if_name:"zero",link_status:"up",tx_power_dbm:0,rx_power_dbm:-19.99}
    ]);` });
    const rows = page.locator("#if-tbody tr");
    await expect(rows).toHaveCount(4);
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
    await page.addScriptTag({ content: `TracePulseDeviceDetail.renderHardwareStatus([
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

test("numeric template metrics render exact values and explicit errors safely", async ({ page }) => {
    await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
    await page.route("**/api/device/*", (route) => route.fulfill({ json: { interfaces: [], metrics: [] } }));
    await page.goto("/");
    await page.evaluate(() => {
        document.body.insertAdjacentHTML("beforeend", "<div id='template-metrics'></div>");
    });
    await page.addScriptTag({ url: "/static/js/device-detail.js" });
    await expect(page.locator("#template-metrics")).toHaveText("No numeric metrics configured for this device.");
    await page.addScriptTag({ content: `TracePulseDeviceDetail.renderTemplateMetrics([
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
    await page.goto("/settings");
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
    await expect(row.locator("meter")).toHaveAttribute("value", "67");
    await expect(page.locator("#dash-tbody tr").filter({ hasText: "unknown-metrics" }).locator("meter")).toHaveCount(0);
    const untrustedRow = page.locator("#dash-tbody tr").filter({ hasText: "router' onmouseover='alert(1)" });
    await expect(untrustedRow.locator("[onmouseover]")).toHaveCount(0);
    await expect(untrustedRow.locator(".status-unknown")).toHaveCount(2);
    await expect(untrustedRow.locator("button")).toHaveAttribute("data-name", "router'%20onmouseover%3D'alert(1)");
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
