export function formatBps(value: unknown): string {
    const numericValue = Number(value || 0);
    if (numericValue >= 1_000_000_000) return `${(numericValue / 1_000_000_000).toFixed(1)} Gbps`;
    if (numericValue >= 1_000_000) return `${(numericValue / 1_000_000).toFixed(1)} Mbps`;
    if (numericValue >= 1_000) return `${(numericValue / 1_000).toFixed(1)} Kbps`;
    return `${Math.round(numericValue)} bps`;
}

export function fmtNum(value: unknown): string {
    return new Intl.NumberFormat("en-US").format(Number(value || 0));
}

export function parseTracePulseTime(iso: unknown): Date {
    if (typeof iso !== "string" || iso.length === 0) return new Date(NaN);
    if (/Z$|[+-]\d\d:\d\d$/.test(iso)) return new Date(iso);
    return new Date(`${iso.replace(" ", "T")}Z`);
}

export function tracePulseTimeZone(): string {
    try {
        return localStorage.getItem("tracepulse-timezone") === "jst" ? "Asia/Tokyo" : "UTC";
    } catch {
        return "UTC";
    }
}

export function formatTracePulseTime(value: Date): string {
    const parts = new Intl.DateTimeFormat("en-US", {
        timeZone: tracePulseTimeZone(),
        month: "numeric",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false,
    }).formatToParts(value);
    const map: Record<string, string> = {};
    parts.forEach((part) => {
        if (part.type !== "literal") map[part.type] = part.value;
    });
    return `${map.month}/${map.day} ${map.hour}:${map.minute}:${map.second}`;
}

export function formatTracePulseClock(value: Date): string {
    const parts = new Intl.DateTimeFormat("en-US", {
        timeZone: tracePulseTimeZone(),
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false,
    }).formatToParts(value);
    const map: Record<string, string> = {};
    parts.forEach((part) => {
        if (part.type !== "literal") map[part.type] = part.value;
    });
    return `${map.hour}:${map.minute}:${map.second}`;
}

export function fmtTime(iso: unknown): string {
    if (!iso) return "-";
    const date = parseTracePulseTime(iso);
    return Number.isNaN(date.getTime()) ? String(iso) : formatTracePulseTime(date);
}
