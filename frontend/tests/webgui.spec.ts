import { expect, test } from "@playwright/test";

test("dashboard loads compiled frontend assets without console errors", async ({ page }) => {
    const consoleErrors: string[] = [];
    page.on("console", (message) => {
        if (message.type() === "error") consoleErrors.push(message.text());
    });

    const response = await page.goto("/");
    expect(response?.ok()).toBeTruthy();
    await expect(page.locator("body")).toHaveAttribute("data-title-key", "dashboard_title");
    await expect(page.locator("#dash-table")).toBeVisible();
    await expect(page.locator("script[src='/static/js/dashboard.js']")).toHaveCount(1);
    await expect(page.locator("script[src='/static/js/i18n.js']")).toHaveCount(1);
    expect(consoleErrors).toEqual([]);
});

test("Community settings do not expose notification destinations", async ({ page, request }) => {
    await page.goto("/settings");
    await expect(page.locator(".destination-grid")).toHaveCount(0);
    await expect(page.locator("script[src='/static/js/notifications.js']")).toHaveCount(0);
    expect((await request.get("/static/js/notifications.js")).status()).toBe(404);
    expect((await request.get("/api/notifications")).status()).toBe(404);
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
    await expect(page.locator(".diag-result-card h3")).toHaveText("Ping Result");
    await expect(page.locator(".diag-result-card")).toContainText("100%");
    await expect(page.locator(".diag-result-verdict.failed")).toContainText("NO RESPONSE");
    await expect(page.locator(".diag-result-card")).not.toContainText("null");
    await run("traceroute");
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

test("Port Check and Traceroute return result cards from the server", async ({ page }) => {
    const tracerouteRequests: Array<{ count: number }> = [];
    page.on("websocket", (socket) => {
        socket.on("framesent", (frame) => {
            const request = JSON.parse(String(frame.payload)) as { kind: string; count: number };
            if (request.kind === "traceroute") tracerouteRequests.push(request);
        });
    });
    await page.goto("/");
    await page.addScriptTag({ url: "/static/js/diagnostics.js" });
    await page.evaluate(() => (window as any).openActiveDiagnostic("127.0.0.1", "port", 8080));
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
