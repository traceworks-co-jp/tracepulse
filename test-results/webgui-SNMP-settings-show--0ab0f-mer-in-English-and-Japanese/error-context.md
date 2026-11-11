# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: webgui.spec.ts >> SNMP settings show the template disclaimer in English and Japanese
- Location: frontend\tests\webgui.spec.ts:675:5

# Error details

```
Error: expect(locator).toContainText(expected) failed

Locator: locator('[data-i18n=\'snmp_template_disclaimer\']')
Expected substring: "TraceWorks assumes no liability"
Received string:    "TracePulse uses RFC-compliant baseline monitoring and automatically selects bundled vendor templates for recognized devices. When using proprietary vendor OIDs or third-party templates, please ensure compliance with your respective hardware vendor's EULA and support agreements. TraceWorks Co., Ltd. assumes no liability for unauthorized access to proprietary vendor metrics."
Timeout: 5000ms

Call log:
  - Expect "toContainText" locator('[data-i18n=\'snmp_template_disclaimer\']') with timeout 5000ms
  - waiting for locator('[data-i18n=\'snmp_template_disclaimer\']')
    14 × locator resolved to <p class="field-hint" data-i18n="snmp_template_disclaimer">TracePulse uses RFC-compliant baseline monitoring…</p>
       - unexpected value "TracePulse uses RFC-compliant baseline monitoring and automatically selects bundled vendor templates for recognized devices. When using proprietary vendor OIDs or third-party templates, please ensure compliance with your respective hardware vendor's EULA and support agreements. TraceWorks Co., Ltd. assumes no liability for unauthorized access to proprietary vendor metrics."

```

```yaml
- paragraph: TracePulse uses RFC-compliant baseline monitoring and automatically selects bundled vendor templates for recognized devices. When using proprietary vendor OIDs or third-party templates, please ensure compliance with your respective hardware vendor's EULA and support agreements. TraceWorks Co., Ltd. assumes no liability for unauthorized access to proprietary vendor metrics.
```

# Test source

```ts
  578 |     await expect(items.first()).toHaveCSS("color", "rgb(203, 213, 225)");
  579 |     await expect(items.first().locator("input")).toHaveCSS("accent-color", "rgb(56, 189, 248)");
  580 |     await page.request.delete("/api/device/192.0.2.252");
  581 | });
  582 | 
  583 | test("device traffic and error breakdown panels adapt to light mode", async ({ page }) => {
  584 |     await page.request.post("/api/discovery/register", { data: [{ ip: "192.0.2.253", name: "light-theme-test", community: "public" }] });
  585 |     await page.route("**/api/device/192.0.2.253", route => route.fulfill({
  586 |         json: {
  587 |             ip: "192.0.2.253", name: "light-theme-test", status: "online", metrics: [], alerts: [], spikes: [],
  588 |             interfaces: [{ if_index: 1, if_name: "Gi0/1", link_status: "up", metrics: { in_errors: 1, in_errors_delta: 1 }, error_breakdown: { fcs_errors: 1, fcs_errors_delta: 1 } }],
  589 |         }
  590 |     }));
  591 |     await page.route("**/api/flow/analytics*", route => route.fulfill({
  592 |         json: {
  593 |             summary: { total_bps: 800, total_pps: 1, active_flows: 1, top_protocol: "TCP" },
  594 |             protocols: [{ protocol: "TCP", bytes: 100, percentage: 100, bps: 800, pps: 1 }],
  595 |             applications: [], top_sources: [], top_destinations: [], top_talkers: [], timeseries: [],
  596 |         }
  597 |     }));
  598 |     await page.goto("/device/192.0.2.253");
  599 |     const trafficCard = page.locator("#traffic-protocols .traffic-card").first();
  600 |     await expect(trafficCard).toBeVisible();
  601 |     await page.locator("#theme-select").selectOption("light");
  602 |     await expect(trafficCard).toHaveCSS("background-color", "rgb(255, 255, 255)");
  603 |     await expect(trafficCard.locator("h3").first()).toHaveCSS("color", "rgb(21, 35, 55)");
  604 |     await expect(page.locator("#traffic-protocols .traffic-controls button").first()).toHaveCSS("background-color", "rgb(237, 243, 246)");
  605 |     await expect(page.locator("#error-breakdown-if")).toHaveCSS("background-color", "rgb(255, 255, 255)");
  606 |     await expect(page.locator("#if-tbody .err-pop").first()).toHaveCSS("background-color", "rgb(255, 255, 255)");
  607 |     await page.locator("#theme-select").selectOption("dark");
  608 |     await expect(trafficCard).toHaveCSS("background-color", "rgba(15, 23, 42, 0.6)");
  609 |     await page.request.delete("/api/device/192.0.2.253");
  610 | });
  611 | 
  612 | test("discovery progress and result banners use light status colors", async ({ page }) => {
  613 |     await page.goto("/discovery");
  614 |     await page.evaluate(() => {
  615 |         const progress = document.querySelector<HTMLElement>("#scan-progress")!;
  616 |         progress.style.display = "block";
  617 |         const error = document.querySelector<HTMLElement>("#scan-error")!;
  618 |         error.style.display = "block";
  619 |         const result = document.querySelector<HTMLElement>("#register-result")!;
  620 |         result.className = "reg-success";
  621 |         result.style.display = "block";
  622 |     });
  623 |     await page.locator("#theme-select").selectOption("light");
  624 |     await expect(page.locator(".progress-bar-track")).toHaveCSS("background-color", "rgb(226, 232, 240)");
  625 |     await expect(page.locator("#progress-stats")).toHaveCSS("color", "rgb(51, 65, 85)");
  626 |     await expect(page.locator("#scan-error")).toHaveCSS("background-color", "rgb(254, 242, 242)");
  627 |     await expect(page.locator("#register-result")).toHaveCSS("background-color", "rgb(240, 253, 244)");
  628 |     await page.locator("#theme-select").selectOption("dark");
  629 |     await expect(page.locator(".progress-bar-track")).toHaveCSS("background-color", "rgb(15, 23, 42)");
  630 | });
  631 | 
  632 | test("active diagnostics panel and port dialog adapt to light mode", async ({ page }) => {
  633 |     await page.goto("/diagnostics");
  634 |     await page.locator("#theme-select").selectOption("light");
  635 |     await page.evaluate(() => window.openActiveDiagnostic?.("127.0.0.1", "ping"));
  636 |     const panel = page.locator("#active-diagnostic-panel");
  637 |     await expect(panel).toHaveCSS("background-color", "rgb(255, 255, 255)");
  638 |     await expect(panel.locator("#active-diagnostic-output")).toHaveCSS("color", "rgb(51, 65, 85)");
  639 |     await page.evaluate(() => window.openActiveDiagnosticWithPortPrompt?.("127.0.0.1", "port", 80));
  640 |     const dialog = page.locator(".diag-port-dialog");
  641 |     await expect(dialog).toHaveCSS("background-color", "rgb(255, 255, 255)");
  642 |     await expect(dialog.locator("input")).toHaveCSS("color", "rgb(21, 35, 55)");
  643 |     await page.locator(".diag-port-dialog [data-cancel]").click();
  644 | });
  645 | 
  646 | test("numeric template metrics render exact values and explicit errors safely", async ({ page }) => {
  647 |     await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
  648 |     await page.route("**/api/device/*", (route) => route.fulfill({ json: { interfaces: [], metrics: [] } }));
  649 |     await page.goto("/");
  650 |     await page.evaluate(() => {
  651 |         document.body.insertAdjacentHTML("beforeend", "<div id='template-metrics'></div>");
  652 |     });
  653 |     await page.addScriptTag({ url: "/static/js/device-detail.js" });
  654 |     await expect(page.locator("#template-metrics")).toHaveText("No numeric metrics configured for this device.");
  655 |     await page.addScriptTag({
  656 |         content: `TracePulseDeviceDetail.renderTemplateMetrics([
  657 |         {key:"cpu",label:"<img src=x onerror=alert(1)>",unit:"percent",data_type:"gauge",oid:"1.3.6.1.2.1.1.3.0",value:0,value_text:"0"},
  658 |         {key:"packets",label:"Packets",unit:"count",data_type:"counter",oid:"1.3.6.1.2.1.1.3.0",value:9007199254740992,value_text:"9007199254740993"},
  659 |         {key:"missing",label:"Missing",unit:"celsius",data_type:"gauge",oid:"1.3.6.1.2.1.1.3.0",value:null,error:"OID <unavailable>"}
  660 |     ]);` });
  661 |     const metrics = page.locator("#template-metrics");
  662 |     await expect(metrics.locator(".hardware-card")).toHaveCount(3);
  663 |     await expect(metrics.locator("[data-metric-key='cpu']")).toContainText("0 percent");
  664 |     await expect(metrics.locator("[data-metric-key='cpu']")).toContainText("<img src=x onerror=alert(1)>");
  665 |     await expect(metrics.locator("img")).toHaveCount(0);
  666 |     await expect(metrics.locator("[data-metric-key='packets']")).toContainText("9007199254740993 count");
  667 |     await expect(metrics.locator("[data-metric-key='missing']")).toContainText("N/A");
  668 |     await expect(metrics.getByRole("alert")).toHaveText("OID <unavailable>");
  669 |     await page.addScriptTag({ content: `TracePulseDeviceDetail.renderTemplateMetrics([], "SNMP unavailable");` });
  670 |     await expect(metrics.getByRole("alert")).toHaveText("SNMP unavailable");
  671 |     await page.addScriptTag({ content: `TracePulseDeviceDetail.renderTemplateMetrics([]);` });
  672 |     await expect(metrics).toHaveText("No numeric metrics configured for this device.");
  673 | });
  674 | 
  675 | test("SNMP settings show the template disclaimer in English and Japanese", async ({ page }) => {
  676 |     await page.goto("/settings/snmp");
  677 |     const disclaimer = page.locator("[data-i18n='snmp_template_disclaimer']");
> 678 |     await expect(disclaimer).toContainText("TraceWorks assumes no liability");
      |                              ^ Error: expect(locator).toContainText(expected) failed
  679 |     await page.evaluate(() => window.setLanguage?.("ja"));
  680 |     await expect(disclaimer).toContainText("EULA");
  681 |     await expect(disclaimer).toContainText("無許可アクセス");
  682 | });
  683 | 
  684 | test("setup guide appears only for empty monitoring and remembers Skip", async ({ page }) => {
  685 |     await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
  686 |     await page.goto("/");
  687 |     const guide = page.locator("#setup-guide");
  688 |     await expect(guide).toBeVisible();
  689 |     await expect(guide.locator("h2")).toHaveText("Choose Language");
  690 |     await expect(guide).not.toContainText("言語");
  691 |     await guide.locator("#setup-language").selectOption("en");
  692 |     await guide.getByRole("button", { name: "Continue", exact: true }).click();
  693 |     await expect(guide.locator("h2")).toHaveText("Welcome to TracePulse");
  694 |     expect(await page.evaluate(() => localStorage.getItem("tracepulse-lang"))).toBe("en");
  695 |     const bounds = await guide.boundingBox();
  696 |     expect(bounds!.x + bounds!.width / 2).toBeCloseTo(page.viewportSize()!.width / 2, 0);
  697 |     expect(bounds!.y + bounds!.height / 2).toBeCloseTo(page.viewportSize()!.height / 2, 0);
  698 |     await guide.getByRole("button", { name: "Skip", exact: true }).click();
  699 |     await expect(guide).not.toBeVisible();
  700 |     await expect(page.locator(".setup-launch")).toBeHidden();
  701 |     expect(await page.evaluate(() => localStorage.getItem("tracepulse-onboarding-v1"))).toBe("done");
  702 |     await page.reload();
  703 |     await expect(guide).not.toBeVisible();
  704 |     await expect(page.locator(".setup-launch")).toBeHidden();
  705 |     await page.goto("/settings");
  706 |     const launch = page.getByRole("button", { name: "Setup Guide", exact: true });
  707 |     await launch.click();
  708 |     await expect(guide).toBeVisible();
  709 |     await expect(guide.locator("#setup-language")).toHaveCount(0);
  710 |     await expect(guide.locator("h2")).toHaveText("Welcome to TracePulse");
  711 |     await page.keyboard.press("Escape");
  712 |     await expect(guide).not.toBeVisible();
  713 |     await expect(launch).toBeFocused();
  714 | });
  715 | 
  716 | test("setup guide does not interrupt configured devices or failed initial checks", async ({ page }) => {
  717 |     await page.route("**/api/devices", (route) => route.fulfill({ json: [{ ip: "192.0.2.1" }] }));
  718 |     await page.goto("/");
  719 |     await expect(page.locator(".setup-launch")).toBeVisible();
  720 |     await expect(page.locator("#setup-guide")).not.toBeVisible();
  721 |     await page.route("**/api/devices", (route) => route.fulfill({ status: 500, json: { error: "unavailable" } }));
  722 |     await page.reload();
  723 |     await expect(page.locator(".setup-launch")).toBeVisible();
  724 |     await expect(page.locator("#setup-guide")).not.toBeVisible();
  725 |     await page.getByRole("button", { name: "Setup Guide", exact: true }).click();
  726 |     await expect(page.locator("#setup-guide h2")).toHaveText("Welcome to TracePulse");
  727 |     await expect(page.locator("#setup-language")).toHaveCount(0);
  728 | });
  729 | 
  730 | test("setup guide discovers and registers only selected devices then finishes", async ({ page }) => {
  731 |     let registered: unknown;
  732 |     await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
  733 |     await page.route("**/api/discovery/scan", async (route) => {
  734 |         const body = new URLSearchParams(route.request().postData() || "");
  735 |         expect(body.get("cidr")).toBe("192.0.2.0/24");
  736 |         expect(body.get("community")).toBe("test-secret");
  737 |         expect(body.get("max_hosts")).toBe("254");
  738 |         await route.fulfill({ json: { job_id: "setup-test" } });
  739 |     });
  740 |     await page.route("**/api/discovery/scan/setup-test", (route) => route.fulfill({ json: { status: "done", scanned: 254, total: 254, devices: [{ ip: "192.0.2.1", name: "<router>" }, { ip: "192.0.2.2", name: "switch" }] } }));
  741 |     await page.route("**/api/discovery/register", async (route) => {
  742 |         registered = route.request().postDataJSON();
  743 |         await route.fulfill({ json: { registered: ["192.0.2.1"], skipped: [], errors: [] } });
  744 |     });
  745 |     await page.goto("/");
  746 |     const guide = page.locator("#setup-guide");
  747 |     await guide.locator("#setup-language").selectOption("en");
  748 |     await guide.getByRole("button", { name: "Continue", exact: true }).click();
  749 |     await guide.getByRole("button", { name: "Next", exact: true }).click();
  750 |     await guide.locator("#setup-cidr").fill("192.0.2.0/24");
  751 |     await guide.locator("#setup-community").fill("test-secret");
  752 |     await guide.getByRole("button", { name: "Discover", exact: true }).click();
  753 |     await expect(guide.locator(".setup-device")).toHaveCount(2);
  754 |     await expect(guide.locator(".setup-device").first()).toContainText("<router>");
  755 |     await guide.locator(".setup-device input").nth(1).uncheck();
  756 |     await guide.getByRole("button", { name: "Register selected devices" }).click();
  757 |     await expect(guide.locator("#setup-status")).toContainText("Monitoring starts automatically");
  758 |     expect(registered).toEqual([{ ip: "192.0.2.1", name: "<router>", community: "test-secret" }]);
  759 |     await guide.getByRole("button", { name: "Next", exact: true }).click();
  760 |     await expect(guide).toContainText("Outbound notifications are not available in Community edition");
  761 |     await guide.getByRole("button", { name: "Finish", exact: true }).click();
  762 |     await expect(page.locator(".setup-launch")).toBeHidden();
  763 |     await page.reload();
  764 |     await expect(guide).not.toBeVisible();
  765 |     await expect(page.locator(".setup-launch")).toBeHidden();
  766 |     expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain("test-secret");
  767 | });
  768 | 
  769 | test("setup guide validates ranges and recovers from discovery and registration failures", async ({ page }) => {
  770 |     let attempts = 0;
  771 |     await page.route("**/api/devices", (route) => route.fulfill({ json: [] }));
  772 |     await page.route("**/api/discovery/scan", (route) => {
  773 |         attempts++;
  774 |         return route.fulfill(attempts === 1 ? { status: 500, json: { error: "scan unavailable" } } : { json: { job_id: "retry" } });
  775 |     });
  776 |     await page.route("**/api/discovery/scan/retry", (route) => route.fulfill({ json: { status: "done", devices: [{ ip: "192.0.2.1" }] } }));
  777 |     let registrations = 0;
  778 |     await page.route("**/api/discovery/register", (route) => {
```