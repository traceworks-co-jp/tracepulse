import { expect, test } from "@playwright/test";

test("Community HTTP registration accepts 26 and 200 nodes but rejects node 201", async ({ page, request }) => {
    const existing = await (await request.get("/api/devices")).json();
    const registration = (start: number, end: number) => request.post("/api/discovery/register", {
        data: Array.from({ length: end - start + 1 }, (_, offset) => ({
            ip: `198.51.100.${start + offset}`, name: `limit-test-${start + offset}`, community: "test",
        })),
    });
    expect(existing).toEqual([]);
    try {
        const first = await (await registration(1, 26)).json();
        expect(first.registered).toHaveLength(26);
        expect(first.errors).toEqual([]);
        const rest = await (await registration(27, 200)).json();
        expect(rest.registered).toHaveLength(174);
        expect(rest.errors).toEqual([]);
        const devices = await (await request.get("/api/devices")).json();
        expect(devices).toHaveLength(200);
        const excess = await (await registration(201, 201)).json();
        expect(excess.registered).toEqual([]);
        expect(excess.errors).toHaveLength(1);
        expect(excess.errors[0]).toContain("200 nodes per instance");
        expect((await (await request.get("/api/devices")).json())).toHaveLength(200);
        await page.goto("/discovery");
        expect(await page.evaluate(() => window.TRACEPULSE_DISCOVERY_LIMITS)).toEqual({ maxCidrs: 1, nodeLimit: 200 });
    } finally {
        const devices = await (await request.get("/api/devices")).json();
        const ids = devices.filter((device: { ip: string }) => device.ip.startsWith("198.51.100."))
            .map((device: { id: number }) => device.id);
        if (ids.length) {
            const response = await request.post("/api/devices/bulk-action", { data: { action: "unregister", device_ids: ids } });
            expect(response.ok()).toBeTruthy();
            expect((await response.json()).ok).toBe(true);
        }
    }
});
