# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: snmp-definitions.spec.ts >> category tables use cancellable modals, collapsed details and a sticky action bar
- Location: frontend\tests\snmp-definitions.spec.ts:127:5

# Error details

```
Error: expect(locator).toBeFocused() failed

Locator:  locator('#snmp-definitions').getByRole('tab', { name: 'Interfaces / optical power', exact: true })
Expected: focused
Received: inactive
Timeout:  5000ms

Call log:
  - Expect "toBeFocused" locator('#snmp-definitions').getByRole('tab', { name: 'Interfaces / optical power', exact: true }) with timeout 5000ms
  - waiting for locator('#snmp-definitions').getByRole('tab', { name: 'Interfaces / optical power', exact: true })
    14 × locator resolved to <button role="tab" type="button" tabindex="-1" aria-selected="false" id="template-tab-interfaces" aria-controls="template-panel">Interfaces / optical power</button>
       - unexpected value "inactive"

```

```yaml
- tab "Interfaces / optical power"
```

# Test source

```ts
  48  |         await page.route("**/api/snmp/definitions", async route => {
  49  |             requests++;
  50  |             const success = await new Promise<boolean>(resolve => { finish = resolve; });
  51  |             await route.fulfill(success ? { json: { ok: true, data: view } } : { status: 500, json: { error: "Definition failed" } });
  52  |         });
  53  |         await page.route("**/api/snmp/definitions/convert", async route => {
  54  |             requests++;
  55  |             const success = await new Promise<boolean>(resolve => { finish = resolve; });
  56  |             await route.fulfill(success ? { json: { ok: true, data: { document: { name: "Progress import", sysObjectID_prefix: "1.3.6.1.4.1.65099", enterprise_id: 65099, cpu: { candidates: [] }, memory: {}, sensors: {}, metrics: [], tables: {} } } } } : { status: 500, json: { error: "Import failed" } });
  57  |         });
  58  |         await page.goto("/settings/snmp");
  59  |         await page.locator("#theme-select").selectOption(theme);
  60  |         const root = page.locator("#snmp-definitions");
  61  |         const progress = root.locator(".operation-progress");
  62  |         await expect(progress.getByRole("status")).toHaveText(language === "ja" ? /監視定義を読み込んでいます.*経過 [1-9]\d*秒/ : /Loading monitoring definitions.*[1-9]\d*s elapsed/);
  63  |         await expect(root.locator("button").first()).toBeDisabled();
  64  |         finish!(true);
  65  |         await expect(progress).toHaveCount(0);
  66  |         await root.locator(".template-save").click();
  67  |         await expect(progress).toBeInViewport();
  68  |         await expect(progress.getByRole("status")).toContainText(language === "ja" ? "監視定義を保存・適用しています" : "Saving and applying monitoring definitions");
  69  |         await expect(root.locator(".template-save")).toBeDisabled();
  70  |         await expect.poll(() => requests).toBe(2);
  71  |         await progress.screenshot({ path: `test-results/definition-progress-${width}-${theme}.png` });
  72  |         finish!(false);
  73  |         await expect(root.getByRole("alert")).toContainText("Definition failed");
  74  |         await expect(progress).toHaveCount(0);
  75  |         await root.locator(".template-save").click();
  76  |         await expect.poll(() => requests).toBe(3);
  77  |         finish!(true);
  78  |         await expect(progress).toHaveCount(0);
  79  |         const file = { name: "progress.toml", mimeType: "application/toml", buffer: Buffer.from('name = "Progress import"\nsysObjectID_prefix = "1.3.6.1.4.1.65099"\n') };
  80  |         await root.locator("#template-import").setInputFiles(file);
  81  |         await expect(progress.getByRole("status")).toContainText(language === "ja" ? "TOMLをインポートしています" : "Importing TOML");
  82  |         await expect.poll(() => requests).toBe(4);
  83  |         finish!(false);
  84  |         await expect(root.getByRole("alert")).toContainText("Import failed");
  85  |         await expect(progress).toHaveCount(0);
  86  |         await root.locator("#template-import").setInputFiles(file);
  87  |         await expect.poll(() => requests).toBe(5);
  88  |         finish!(true);
  89  |         await expect(progress).toHaveCount(0);
  90  |         await expect(root.locator("#template-profile")).toHaveValue("1.3.6.1.4.1.65099");
  91  |         await expect(root.locator("[data-template-status]")).toContainText(language === "ja" ? "未保存" : "Unsaved");
  92  |     });
  93  | }
  94  | 
  95  | test("profile selector and action buttons share a baseline and control height", async ({ page }) => {
  96  |     await page.addInitScript(() => localStorage.setItem("tracepulse-lang", "ja"));
  97  |     await page.goto("/settings/snmp");
  98  |     const toolbar = page.locator(".template-toolbar");
  99  |     const profile = page.locator("#template-profile");
  100 |     for (const width of [1920, 1280, 375]) {
  101 |         await page.setViewportSize({ width, height: 900 });
  102 |         for (const id of ["generic", "@common"]) {
  103 |             await profile.selectOption(id);
  104 |             for (const theme of ["dark", "light"]) {
  105 |                 await page.locator("#theme-select").selectOption(theme);
  106 |                 await expect(profile).toHaveCSS("height", "40px");
  107 |                 const selectBounds = (await profile.boundingBox())!;
  108 |                 const buttons = await toolbar.locator("button").all();
  109 |                 for (const control of buttons) {
  110 |                     const bounds = (await control.boundingBox())!;
  111 |                     expect(bounds.height).toBeGreaterThanOrEqual(40);
  112 |                     if (width > 600) {
  113 |                         expect(bounds.height).toBe(40);
  114 |                         if (Math.abs(bounds.y - selectBounds.y) < 40) {
  115 |                             expect(Math.abs(bounds.y - selectBounds.y)).toBeLessThanOrEqual(1);
  116 |                             expect(Math.abs(bounds.y + bounds.height - selectBounds.y - selectBounds.height)).toBeLessThanOrEqual(1);
  117 |                         }
  118 |                     } else expect(bounds.y).toBeGreaterThanOrEqual(selectBounds.y + selectBounds.height);
  119 |                 }
  120 |                 expect(await toolbar.evaluate(element => element.scrollWidth <= element.clientWidth)).toBeTruthy();
  121 |                 await toolbar.screenshot({ path: `test-results/template-toolbar-${width}-${id === "@common" ? "common" : "profile"}-${theme}.png` });
  122 |             }
  123 |         }
  124 |     }
  125 | });
  126 | 
  127 | test("category tables use cancellable modals, collapsed details and a sticky action bar", async ({ page }) => {
  128 |     await page.goto("/settings/snmp");
  129 |     const root = page.locator("#snmp-definitions");
  130 |     await expect(root.getByRole("tab")).toHaveCount(4);
  131 |     await expect(root.locator("fieldset")).toHaveCount(0);
  132 |     await expect(root.locator(".template-toolbar")).toHaveCSS("position", "sticky");
  133 |     const original = await root.locator('[data-template-row="profile.cpu.candidates.0"]').textContent() || "";
  134 |     let dialog = await openRow(page, "profile.cpu.candidates.0");
  135 |     await dialog.locator('[data-template-path="profile.cpu.candidates.0"]').fill("1.3.6.1.4.1.65000.999");
  136 |     await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  137 |     await expect(root.locator('[data-template-row="profile.cpu.candidates.0"]')).toHaveText(original);
  138 |     await expect(root.locator("[data-template-status]")).toBeEmpty();
  139 |     await root.getByRole("button", { name: "CPU settings", exact: true }).click();
  140 |     dialog = page.getByRole("dialog");
  141 |     await expect(dialog.locator("details")).not.toHaveAttribute("open");
  142 |     await dialog.getByText("Advanced settings", { exact: true }).click();
  143 |     await expect(dialog.getByLabel("Table value selection", { exact: true })).toBeVisible();
  144 |     await page.keyboard.press("Escape");
  145 |     await expect(dialog).not.toBeVisible();
  146 |     await root.getByRole("tab", { name: "CPU / memory", exact: true }).focus();
  147 |     await page.keyboard.press("ArrowRight");
> 148 |     await expect(root.getByRole("tab", { name: "Interfaces / optical power", exact: true })).toBeFocused();
      |                                                                                              ^ Error: expect(locator).toBeFocused() failed
  149 |     await expect(root.locator('[data-template-row="profile.tables.interfaces.in_octets"]')).toContainText("1.3.6.1.2.1.2.2.1.10");
  150 |     for (const width of [1280, 375]) {
  151 |         await page.setViewportSize({ width, height: 900 });
  152 |         for (const theme of ["dark", "light"]) {
  153 |             await page.locator("#theme-select").selectOption(theme);
  154 |             expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  155 |             await root.locator(".template-table-scroll").evaluate(element => { element.scrollLeft = 0; });
  156 |             await root.screenshot({ path: `test-results/template-table-${width}-${theme}.png` });
  157 |             dialog = await openRow(page, "profile.tables.interfaces.in_octets");
  158 |             expect(await dialog.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
  159 |             await dialog.screenshot({ path: `test-results/template-modal-${width}-${theme}.png` });
  160 |             await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  161 |         }
  162 |     }
  163 |     await page.locator("#lang-select").selectOption("ja");
  164 |     await expect(root.getByRole("tab", { name: "インターフェース・光レベル", exact: true })).toHaveAttribute("aria-selected", "true");
  165 |     for (const width of [1280, 375]) {
  166 |         await page.setViewportSize({ width, height: 900 });
  167 |         await root.locator(".template-table-scroll").evaluate(element => { element.scrollLeft = 0; });
  168 |         await root.screenshot({ path: `test-results/template-table-ja-${width}.png` });
  169 |     }
  170 | });
  171 | 
  172 | test("sensor, DOM and common OID modal additions preserve unrelated profile definitions", async ({ page, request }) => {
  173 |     const read = async () => (await (await request.get("/api/snmp/definitions")).json()).data;
  174 |     const original = await read();
  175 |     const generic = original.profiles.find((profile: { id: string }) => profile.id === "generic").document;
  176 |     try {
  177 |         await page.goto("/settings/snmp");
  178 |         let dialog = await addItem(page, "hardware", "Temperature sensors");
  179 |         for (const [key, oid] of [["descr_prefix", "1.3.6.1.4.1.65000.80.1"], ["value_prefix", "1.3.6.1.4.1.65000.80.2"], ["state_prefix", "1.3.6.1.4.1.65000.80.3"]]) {
  180 |             await dialog.locator(`[data-template-path="profile.sensors.temperature.0.${key}"]`).fill(oid);
  181 |         }
  182 |         await dialog.getByRole("button", { name: "Apply to draft", exact: true }).click();
  183 |         dialog = await addItem(page, "interfaces", "Optical power metric");
  184 |         await dialog.locator('[data-template-path="profile.tables.dom.metrics.0.oid"]').fill("1.3.6.1.4.1.65000.81.2");
  185 |         await dialog.getByRole("button", { name: "Apply to draft", exact: true }).click();
  186 |         await page.locator("#snmp-definitions").getByRole("button", { name: "Optical table settings", exact: true }).click();
  187 |         dialog = page.getByRole("dialog");
  188 |         await dialog.getByText("Advanced settings", { exact: true }).click();
  189 |         await dialog.locator('[data-template-path="profile.tables.dom.base_oid"]').fill("1.3.6.1.4.1.65000.81");
  190 |         await dialog.getByRole("button", { name: "Apply to draft", exact: true }).click();
  191 |         await page.locator("#template-profile").selectOption("@common");
  192 |         dialog = await addItem(page, "hardware", "Additional hardware sensor OIDs");
  193 |         await dialog.locator('[data-template-path="common.hardware_oids.0"]').fill("temperature|1.3.6.1.4.1.65000.82.0");
  194 |         await dialog.getByRole("button", { name: "Apply to draft", exact: true }).click();
  195 |         const row = page.locator('[data-template-row="common.hardware_oids.0"]');
  196 |         await expect(row.locator("code")).toHaveText("1.3.6.1.4.1.65000.82.0");
  197 |         await expect(row).toContainText("Configured");
  198 |         await page.getByRole("button", { name: "Save and apply", exact: true }).click();
  199 |         await expect(page.locator("[data-template-status]")).toHaveText("Saved and applied");
  200 |         const saved = await read();
  201 |         expect(saved.definitions.profiles.generic.cpu).toEqual(generic.cpu);
  202 |         expect(saved.definitions.profiles.generic.tables.interfaces).toEqual(generic.tables.interfaces);
  203 |         expect(saved.definitions.profiles.generic.sensors.temperature[0].value_prefix).toBe("1.3.6.1.4.1.65000.80.2");
  204 |         expect(saved.definitions.profiles.generic.tables.dom.metrics[0].scale).toBe(0.01);
  205 |         expect(saved.definitions.common.hardware_oids).toContain("temperature|1.3.6.1.4.1.65000.82.0");
  206 |     } finally {
  207 |         const current = await read();
  208 |         expect((await (await request.post("/api/snmp/definitions", { data: { generation: current.generation, definitions: original.definitions } })).json()).ok).toBe(true);
  209 |     }
  210 | });
  211 | 
  212 | test("definition source is a readable localized metadata row across profiles and themes", async ({ page, request }) => {
  213 |     const response = await (await request.get("/api/snmp/definitions")).json();
  214 |     response.data.definitions.profiles = {};
  215 |     const sources = [
  216 |         ["generic", "bundled", "同梱テンプレート"],
  217 |         ["1.3.6.1.4.1.9", "byo", "外部TOMLテンプレート"],
  218 |         ["1.3.6.1.4.1.2636", "database", "ユーザー定義（SQLite）"],
  219 |     ];
  220 |     for (const [id, source] of sources) response.data.profiles.find((profile: { id: string }) => profile.id === id).source = source;
  221 |     await page.route("**/api/snmp/definitions", route => route.fulfill({ json: response }));
  222 |     await page.goto("/settings/snmp");
  223 |     await page.locator("#lang-select").selectOption("ja");
  224 |     const sourceInfo = page.locator("#snmp-definitions .template-source");
  225 |     for (const width of [1280, 375]) {
  226 |         await page.setViewportSize({ width, height: 900 });
  227 |         for (const theme of ["dark", "light"]) {
  228 |             await page.locator("#theme-select").selectOption(theme);
  229 |             if (theme === "light") {
  230 |                 await expect(sourceInfo).toHaveCSS("background-color", "rgb(241, 245, 249)");
  231 |                 await expect(sourceInfo.locator("dd")).toHaveCSS("color", "rgb(15, 23, 42)");
  232 |             }
  233 |             for (const [id, source, label] of sources) {
  234 |                 await page.locator("#template-profile").selectOption(id);
  235 |                 await expect(sourceInfo).toHaveAttribute("data-template-source", source);
  236 |                 await expect(sourceInfo.locator("dt")).toHaveText("定義元");
  237 |                 await expect(sourceInfo.locator("dd")).toHaveText(label);
  238 |                 await expect(sourceInfo.locator("dd")).toHaveCSS("font-weight", "600");
  239 |                 expect(await sourceInfo.evaluate(element => element.scrollWidth <= element.clientWidth)).toBeTruthy();
  240 |                 if (id === "generic") await sourceInfo.screenshot({ path: `test-results/template-source-${width}-${theme}.png` });
  241 |             }
  242 |         }
  243 |     }
  244 |     await page.locator("#lang-select").selectOption("en");
  245 |     await expect(sourceInfo.locator("dt")).toHaveText("Definition source");
  246 |     await expect(sourceInfo.locator("dd")).toHaveText("User definition (SQLite)");
  247 |     await page.locator("#template-profile").selectOption("@common");
  248 |     await expect(sourceInfo).toHaveCount(0);
```