import { defineConfig, devices } from "@playwright/test";

const baseURL = process.env.TRACEPULSE_TEST_BASE_URL || "http://127.0.0.1:8080";

export default defineConfig({
    testDir: "frontend/tests",
    timeout: 30_000,
    use: {
        baseURL,
        trace: "retain-on-failure",
        ...devices["Desktop Chrome"],
    },
    webServer: {
        command: "cargo run --quiet -- --web",
        url: `${baseURL}/`,
        reuseExistingServer: true,
        timeout: 120_000,
    },
});
