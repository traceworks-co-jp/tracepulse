import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
    testDir: "frontend/tests",
    timeout: 30_000,
    use: {
        baseURL: "http://127.0.0.1:8080",
        trace: "retain-on-failure",
        ...devices["Desktop Chrome"],
    },
    webServer: {
        command: "cargo run --quiet -- --web",
        url: "http://127.0.0.1:8080/",
        reuseExistingServer: true,
        timeout: 120_000,
    },
});
