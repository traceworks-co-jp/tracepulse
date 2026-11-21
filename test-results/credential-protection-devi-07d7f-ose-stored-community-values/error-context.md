# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: credential-protection.spec.ts >> device APIs and rendered pages never expose stored community values
- Location: frontend\tests\credential-protection.spec.ts:250:5

# Error details

```
Error: expect(locator).not.toBeEmpty() failed

Locator: locator('#snmp-connection-status')
Expected: not empty
Timeout: 5000ms
Error: element(s) not found

Call log:
  - Expect "not toBeEmpty" locator('#snmp-connection-status') with timeout 5000ms
  - waiting for locator('#snmp-connection-status')

```

```yaml
- text: "{\"error\":\"not found\"}"
```

# Test source

```ts
  182 |     const ip = "192.0.2.236";
  183 |     try {
  184 |         await request.post("/api/discovery/register", { data: [{ ip, community: "legacy-credential" }] });
  185 |         const device = (await (await request.get("/api/devices")).json()).find((item: { ip: string }) => item.ip === ip);
  186 |         const credentials = { version: "3", community: "device-community-secret", v3_username: "device-monitor", v3_auth_protocol: "sha256", v3_auth_password: "device-auth-secret", v3_priv_protocol: "aes128", v3_priv_password: "device-privacy-secret" };
  187 |         expect((await request.post("/api/devices/snmp-credentials", { data: { device_ids: [device.id], credentials } })).ok()).toBeTruthy();
  188 |         await page.goto("/");
  189 |         await page.getByRole("checkbox", { name: `Select device ${ip}`, exact: true }).check();
  190 |         const open = async () => {
  191 |             await page.locator("#bulk-rename").click();
  192 |             await page.getByRole("button", { name: "SNMP credentials", exact: true }).click();
  193 |             await expect(page.locator("#device-snmp-dialog [type=submit]")).toBeEnabled();
  194 |         };
  195 |         await open();
  196 |         const dialog = page.locator("#device-snmp-dialog");
  197 |         await expect(dialog.locator("#device-snmp-version")).toHaveValue("3");
  198 |         await expect(dialog.locator("#device-snmp-v3_username")).toHaveValue("device-monitor");
  199 |         await expect(dialog.locator("#device-snmp-v3_auth_protocol")).toHaveValue("sha256");
  200 |         for (const field of ["community", "v3_auth_password", "v3_priv_password"]) {
  201 |             await expect(dialog.locator(`#device-snmp-${field}`)).toHaveValue("");
  202 |             await expect(dialog.locator(`#device-snmp-${field}`)).toHaveAttribute("placeholder", "Configured (enter only to replace)");
  203 |         }
  204 |         const response = await request.get(`/api/device/${ip}/snmp-credentials`);
  205 |         const body = await response.text();
  206 |         for (const secret of [credentials.community, credentials.v3_auth_password, credentials.v3_priv_password]) expect(body).not.toContain(secret);
  207 |         await dialog.locator("#device-snmp-v3_username").fill("updated-monitor");
  208 |         await dialog.getByRole("button", { name: "Save", exact: true }).click();
  209 |         await expect(dialog).not.toBeVisible();
  210 |         await open();
  211 |         await expect(dialog.locator("#device-snmp-v3_username")).toHaveValue("updated-monitor");
  212 |         await expect(dialog.locator("#device-snmp-v3_auth_password")).toHaveAttribute("placeholder", "Configured (enter only to replace)");
  213 |         await page.route("**/api/devices/snmp-credentials", route => route.fulfill({ status: 500, json: { error: "Reset failed" } }));
  214 |         page.once("dialog", confirmation => confirmation.accept());
  215 |         await dialog.getByRole("button", { name: "Use global defaults", exact: true }).click();
  216 |         await expect(dialog.locator("[role=alert]")).toContainText("Reset failed");
  217 |         await expect(dialog.getByRole("button", { name: "Use global defaults", exact: true })).toBeEnabled();
  218 |         await page.unroute("**/api/devices/snmp-credentials");
  219 |         page.once("dialog", confirmation => confirmation.accept());
  220 |         await dialog.getByRole("button", { name: "Use global defaults", exact: true }).click();
  221 |         await expect(dialog).not.toBeVisible();
  222 |         await open();
  223 |         await expect(dialog.locator("[role=status]")).toContainText("global defaults");
  224 |         await expect(dialog.locator("#device-snmp-version")).toHaveValue("");
  225 |         await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  226 |     } finally { await request.delete(`/api/device/${ip}`); }
  227 | });
  228 | 
  229 | test("global v3 settings save encrypted values without returning them to the form", async ({ page, request }) => {
  230 |     try {
  231 |         await page.goto("/settings/snmp");
  232 |         await page.locator("#global-snmp-version").selectOption("3");
  233 |         await page.locator("#global-snmp-v3_username").fill("global-v3-user-marker");
  234 |         await page.locator("#global-snmp-v3_auth_password").fill("global-v3-auth-marker");
  235 |         await page.locator("#global-snmp-v3_priv_password").fill("global-v3-privacy-marker");
  236 |         await page.locator("#save-snmp-connection").click();
  237 |         await expect(page.locator("#snmp-connection-status")).not.toBeEmpty();
  238 |         await expect(page.locator("#global-snmp-v3_auth_password")).toHaveValue("");
  239 |         await page.reload();
  240 |         await expect(page.locator("#global-snmp-v3_priv_password")).toHaveValue("");
  241 |         const settings = await (await request.get("/api/settings")).json();
  242 |         expect(settings.snmp.version).toBe("3");
  243 |         expect(settings.snmp.v3_configured).toBe(true);
  244 |         expect(JSON.stringify(settings)).not.toContain("global-v3-auth-marker");
  245 |         expect(await page.content()).not.toContain("global-v3-privacy-marker");
  246 |         await page.screenshot({ path: "test-results/snmp-credentials-settings.png", fullPage: true });
  247 |     } finally { await request.post("/api/settings", { data: { snmp_credentials: { version: "2c" } } }); }
  248 | });
  249 | 
  250 | test("device APIs and rendered pages never expose stored community values", async ({ page, request }) => {
  251 |     const ip = "192.0.2.241";
  252 |     const secret = "browser-device-secret-291735";
  253 |     try {
  254 |         expect((await request.post("/api/discovery/register", {
  255 |             data: [{ ip, name: "secret-protection-test", community: secret }],
  256 |         })).ok()).toBeTruthy();
  257 |         for (const endpoint of ["/api/devices", "/api/settings", "/settings", "/"]) {
  258 |             const response = await request.get(endpoint);
  259 |             expect(await response.text()).not.toContain(secret);
  260 |         }
  261 |         const devices = await (await request.get("/api/devices")).json();
  262 |         const device = devices.find((item: { ip: string }) => item.ip === ip);
  263 |         expect(device.community).toBe("********");
  264 |         expect(device.community_configured).toBe(true);
  265 |         await page.goto("/");
  266 |         await expect(page.locator("#dash-tbody")).toContainText("secret-protection-test");
  267 |         await expect(page.locator("#th-community, #sort-community")).toHaveCount(0);
  268 |         await expect(page.locator("#dash-table thead th")).toHaveCount(9);
  269 |         await expect(page.locator("#dash-tbody tr").filter({ hasText: ip }).locator("td")).toHaveCount(9);
  270 |         await expect(page.locator("#dash-tbody")).not.toContainText("********");
  271 |         await page.evaluate(() => window.refreshDashboard?.());
  272 |         await expect(page.locator("#dash-tbody")).not.toContainText("********");
  273 |         expect(await page.content()).not.toContain(secret);
  274 |         await page.goto("/settings");
  275 |         await expect(page.locator("#community")).toHaveCount(0);
  276 |         await expect(page.locator("#snmp-connection-form")).toHaveCount(0);
  277 |         await page.getByRole("link", { name: "Manage SNMP Settings", exact: true }).click();
  278 |         await expect(page.locator("#community")).toHaveAttribute("type", "password");
  279 |         await expect(page.locator("#community")).toHaveValue("");
  280 |         await page.locator("#community").fill("replacement-browser-secret");
  281 |         await page.getByRole("button", { name: "Save Connection Settings", exact: true }).click();
> 282 |         await expect(page.locator("#snmp-connection-status")).not.toBeEmpty();
      |                                                                   ^ Error: expect(locator).not.toBeEmpty() failed
  283 |         await expect(page.locator("#community")).toHaveValue("");
  284 |         const settings = await (await request.get("/api/settings")).json();
  285 |         expect(settings.snmp.default_community).toBe("");
  286 |         expect(settings.snmp.community_configured).toBe(true);
  287 |         await page.reload();
  288 |         await expect(page.locator("#community")).toHaveValue("");
  289 |         await page.getByRole("button", { name: "Save Connection Settings", exact: true }).click();
  290 |         await expect(page.locator("#snmp-connection-status")).not.toBeEmpty();
  291 |         await expect(page.locator("#community")).toHaveValue("");
  292 |     } finally {
  293 |         await request.delete(`/api/device/${ip}`);
  294 |     }
  295 | });
  296 | 
  297 | test("SNMP diagnostics sends credentials in a POST body, not the URL", async ({ page }) => {
  298 |     let method = "";
  299 |     let url = "";
  300 |     let body = "";
  301 |     await page.goto("/diagnostics");
  302 |     await page.route("**/diagnostics", async (route) => {
  303 |         method = route.request().method();
  304 |         url = route.request().url();
  305 |         body = route.request().postData() || "";
  306 |         await route.fulfill({ contentType: "text/html", body: "<p id='diagnostic-post-done'>Done</p>" });
  307 |     });
  308 |     await page.locator("input[name='ip']").fill("192.0.2.1");
  309 |     await page.locator("input[name='community']").fill("diagnostic-private-value");
  310 |     await page.locator("#diag-run-btn").click();
  311 |     await expect(page.locator("#diagnostic-post-done")).toBeVisible();
  312 |     expect(method).toBe("POST");
  313 |     expect(url).not.toContain("diagnostic-private-value");
  314 |     expect(new URLSearchParams(body).get("community")).toBe("diagnostic-private-value");
  315 |     expect(page.url()).not.toContain("diagnostic-private-value");
  316 | });
  317 | 
  318 | test("discovery registration retains the typed secret without storing it in result attributes", async ({ page }) => {
  319 |     let community = "";
  320 |     await page.route("**/api/discovery/scan", (route) => route.fulfill({ json: { job_id: "fake", total: 1 } }));
  321 |     await page.route("**/api/discovery/scan/fake", (route) => route.fulfill({
  322 |         json: { status: "done", scanned: 1, total: 1, devices: [{ ip: "192.0.2.240", name: "found-switch", status: "online", community_configured: true }] },
  323 |     }));
  324 |     await page.route("**/api/discovery/register", async (route) => {
  325 |         community = route.request().postDataJSON()[0].community;
  326 |         await route.fulfill({ json: { registered: ["192.0.2.240"], skipped: [], errors: [] } });
  327 |     });
  328 |     await page.goto("/discovery");
  329 |     await page.locator("#cidr").fill("192.0.2.240/32");
  330 |     await page.locator("#community").fill("typed-scan-secret");
  331 |     await page.locator("#scan-btn").click();
  332 |     await expect(page.locator("#results-body")).toContainText("found-switch");
  333 |     expect(await page.locator("#results-body").innerHTML()).not.toContain("typed-scan-secret");
  334 |     await page.locator(".row-cb").check();
  335 |     await page.locator("#register-btn").click();
  336 |     await expect.poll(() => community).toBe("typed-scan-secret");
  337 | });
  338 | 
  339 | test("discovery community eye buttons preserve values and support keyboard, language, and narrow screens", async ({ page }) => {
  340 |     await page.setViewportSize({ width: 375, height: 667 });
  341 |     await page.goto("/discovery");
  342 |     await page.locator("#community").fill("typed-scan-secret");
  343 |     const toggle = page.locator("button[aria-controls=community]");
  344 |     await expect(toggle).toHaveAccessibleName("Show community");
  345 |     await expect(page.locator("#community")).toHaveAttribute("type", "password");
  346 |     await expect(toggle).toHaveAttribute("aria-pressed", "false");
  347 |     const inputBounds = (await page.locator("#community").boundingBox())!;
  348 |     const buttonBounds = (await toggle.boundingBox())!;
  349 |     expect(buttonBounds.x).toBeGreaterThan(inputBounds.x + inputBounds.width / 2);
  350 |     expect(buttonBounds.x + buttonBounds.width).toBeLessThanOrEqual(inputBounds.x + inputBounds.width);
  351 |     await toggle.click();
  352 |     await expect(page.locator("#community")).toHaveAttribute("type", "text");
  353 |     await expect(toggle).toHaveAccessibleName("Hide community");
  354 |     await expect(toggle).toHaveAttribute("aria-pressed", "true");
  355 |     await expect(page.locator("#community")).toHaveValue("typed-scan-secret");
  356 |     await toggle.focus();
  357 |     await page.keyboard.press("Space");
  358 |     await expect(page.locator("#community")).toHaveAttribute("type", "password");
  359 |     await page.evaluate(() => window.setLanguage?.("ja"));
  360 |     await expect(toggle).toHaveAccessibleName("コミュニティを表示");
  361 |     await toggle.click();
  362 |     await expect(toggle).toHaveAccessibleName("コミュニティを隠す");
  363 |     await page.locator("a[data-i18n=add_manually]").click();
  364 |     const manual = page.locator("#manual-community");
  365 |     await manual.fill("manual-secret");
  366 |     const manualToggle = page.locator("button[aria-controls=manual-community]");
  367 |     await expect(manualToggle).toHaveAccessibleName("コミュニティを表示");
  368 |     await manualToggle.click();
  369 |     await expect(manual).toHaveAttribute("type", "text");
  370 |     await expect(manual).toHaveValue("manual-secret");
  371 |     await manualToggle.click();
  372 |     await expect(manual).toHaveAttribute("type", "password");
  373 |     await page.reload();
  374 |     await expect(page.locator("#community")).toHaveAttribute("type", "password");
  375 | });
  376 | 
  377 | test("setup guide community visibility toggles without submitting discovery", async ({ page }) => {
  378 |     let scans = 0;
  379 |     await page.route("**/api/discovery/scan", async (route) => {
  380 |         scans++;
  381 |         await route.fulfill({ json: { error: "Unexpected submission" } });
  382 |     });
```