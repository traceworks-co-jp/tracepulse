# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: node-limits.spec.ts >> Community HTTP registration accepts 26 and 200 nodes but rejects node 201
- Location: frontend\tests\node-limits.spec.ts:3:5

# Error details

```
Error: expect(received).toHaveLength(expected)

Expected length: 174
Received length: 171
Received array:  ["198.51.100.27", "198.51.100.28", "198.51.100.29", "198.51.100.30", "198.51.100.31", "198.51.100.32", "198.51.100.33", "198.51.100.34", "198.51.100.35", "198.51.100.36", …]
```

# Test source

```ts
  1  | import { expect, test } from "@playwright/test";
  2  | 
  3  | test("Community HTTP registration accepts 26 and 200 nodes but rejects node 201", async ({ page, request }) => {
  4  |     const existing = await (await request.get("/api/devices")).json();
  5  |     const registration = (start: number, end: number) => request.post("/api/discovery/register", {
  6  |         data: Array.from({ length: end - start + 1 }, (_, offset) => ({
  7  |             ip: `198.51.100.${start + offset}`, name: `limit-test-${start + offset}`, community: "test",
  8  |         })),
  9  |     });
  10 |     expect(existing).toEqual([]);
  11 |     try {
  12 |         const first = await (await registration(1, 26)).json();
  13 |         expect(first.registered).toHaveLength(26);
  14 |         expect(first.errors).toEqual([]);
  15 |         const rest = await (await registration(27, 200)).json();
> 16 |         expect(rest.registered).toHaveLength(174);
     |                                 ^ Error: expect(received).toHaveLength(expected)
  17 |         expect(rest.errors).toEqual([]);
  18 |         const devices = await (await request.get("/api/devices")).json();
  19 |         expect(devices).toHaveLength(200);
  20 |         const excess = await (await registration(201, 201)).json();
  21 |         expect(excess.registered).toEqual([]);
  22 |         expect(excess.errors).toHaveLength(1);
  23 |         expect(excess.errors[0]).toContain("200 nodes per instance");
  24 |         expect((await (await request.get("/api/devices")).json())).toHaveLength(200);
  25 |         await page.goto("/discovery");
  26 |         expect(await page.evaluate(() => window.TRACEPULSE_DISCOVERY_LIMITS)).toEqual({ maxCidrs: 1, nodeLimit: 200 });
  27 |     } finally {
  28 |         const devices = await (await request.get("/api/devices")).json();
  29 |         const ids = devices.filter((device: { ip: string }) => device.ip.startsWith("198.51.100."))
  30 |             .map((device: { id: number }) => device.id);
  31 |         if (ids.length) {
  32 |             const response = await request.post("/api/devices/bulk-action", { data: { action: "unregister", device_ids: ids } });
  33 |             expect(response.ok()).toBeTruthy();
  34 |             expect((await response.json()).ok).toBe(true);
  35 |         }
  36 |     }
  37 | });
  38 | 
```