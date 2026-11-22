import { expect, test } from "@playwright/test";

for (const [language, width] of [["en", 1280], ["ja", 375]] as const) {
    test(`SNMP diagnostics shows elapsed time and stops on completion (${language})`, async ({ page }) => {
        await page.setViewportSize({ width, height: 900 });
        await page.addInitScript(lang => {
            localStorage.setItem("tracepulse-onboarding-v1", "done");
            localStorage.setItem("tracepulse-lang", lang);
        }, language);
        let finish: (() => void) | undefined;
        await page.route("**/diagnostics", async route => {
            if (route.request().method() !== "POST") return route.continue();
            await new Promise<void>(resolve => { finish = resolve; });
            await route.fulfill({ contentType: "text/html", body: "<html><body>diagnostics complete</body></html>" });
        });
        await page.goto("/diagnostics");
        await page.clock.install();
        await page.evaluate(() => {
            const counts = { cleared: 0 };
            Object.assign(window, { __diagnosticTimerCounts: counts });
            const clear = window.clearInterval;
            window.clearInterval = id => { counts.cleared++; clear.call(window, id); };
        });
        await page.locator("#diag-ip").fill("192.0.2.1");
        await page.locator("#diag-run-btn").click();
        await expect(page.locator("#diag-run-btn")).toBeDisabled();
        await expect.poll(() => !!finish).toBe(true);
        await page.clock.fastForward(3000);
        await expect(page.locator("#diag-progress")).toContainText(language === "ja" ? "診断中… (3秒)" : "Diagnosing… (3s)");
        await expect(page.locator("#diag-progress")).toContainText(language === "ja" ? "ハードウェアセンサー" : "hardware sensors");
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
        finish!();
        await expect(page.locator("body")).toHaveText("diagnostics complete");
        expect(await page.evaluate(() => Reflect.get(window, "__diagnosticTimerCounts").cleared)).toBe(1);
    });
}

test("SNMP diagnostics stops its timer on connection failure", async ({ page }) => {
    await page.goto("/diagnostics");
    let fail: (() => void) | undefined;
    await page.route("**/diagnostics", async route => {
        await new Promise<void>(resolve => { fail = resolve; });
        await route.abort("failed");
    });
    await page.clock.install();
    await page.locator("#diag-ip").fill("192.0.2.1");
    await page.locator("#diag-run-btn").click();
    await expect.poll(() => !!fail).toBe(true);
    await page.clock.fastForward(2000);
    fail!();
    await expect(page.locator("#diag-run-btn")).toBeEnabled();
    const error = await page.locator("#diag-progress").textContent();
    expect(error).toContain("Failed to fetch");
    await page.clock.fastForward(3000);
    await expect(page.locator("#diag-progress")).toHaveText(error!);
});
