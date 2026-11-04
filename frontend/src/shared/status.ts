export function deviceStatusClass(status?: string | null): string {
    switch (status?.trim().toLowerCase()) {
        case "online": return "status-online";
        case "offline": return "status-offline";
        case "warning": return "status-warning";
        case "critical": return "status-critical";
        default: return "status-unknown";
    }
}

export function interfaceStatusClass(status?: string): string {
    switch (status?.trim().toLowerCase()) {
        case "up": return "link-up";
        case "down":
        case "lowerlayerdown": return "link-down";
        case "testing": return "status-warning";
        default: return "status-unknown";
    }
}