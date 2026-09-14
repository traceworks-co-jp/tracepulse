export function bootstrapTheme(): void {
    try {
        document.documentElement.dataset.theme = localStorage.getItem("tracepulse-theme") || "dark";
    } catch {
        document.documentElement.dataset.theme = "dark";
    }
}

bootstrapTheme();
