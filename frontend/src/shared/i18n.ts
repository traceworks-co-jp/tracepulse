interface TranslationTable {
    [key: string]: string;
}

import { translations } from "./i18n-data";

interface TranslationData {
    en?: TranslationTable;
    ja?: TranslationTable;
}

function translationData(): TranslationData {
    return translations;
}

export function t(key: string, lang = currentLanguage()): string {
    const data = translationData();
    return data[lang as keyof TranslationData]?.[key] || data.en?.[key] || key;
}

export function tf(key: string, params: Record<string, string | number> = {}, lang = currentLanguage()): string {
    let text = t(key, lang);
    Object.entries(params).forEach(([name, value]) => {
        text = text.split(`{${name}}`).join(String(value));
    });
    return text;
}

export function currentLanguage(): string {
    try { return localStorage.getItem("tracepulse-lang") || "en"; } catch { return "en"; }
}

export function setLanguage(lang: string): void {
    try { localStorage.setItem("tracepulse-lang", lang); } catch { /* ignore */ }
    applyLanguage(lang);
}

export function currentTheme(): string {
    try { return localStorage.getItem("tracepulse-theme") || "dark"; } catch { return "dark"; }
}

export function setTheme(theme: string): void {
    try { localStorage.setItem("tracepulse-theme", theme); } catch { /* ignore */ }
    applyTheme(theme);
}

export function applyTheme(theme = currentTheme()): void {
    document.documentElement.dataset.theme = theme;
    const select = document.getElementById("theme-select") as HTMLSelectElement | null;
    if (select && select.value !== theme) select.value = theme;
}

export function applyLanguage(lang = currentLanguage()): void {
    document.documentElement.lang = lang;
    const languageSelect = document.getElementById("lang-select") as HTMLSelectElement | null;
    if (languageSelect && languageSelect.value !== lang) languageSelect.value = lang;
    document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((element) => { element.textContent = t(element.dataset.i18n || "", lang); });
    document.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>("[data-i18n-placeholder]").forEach((element) => { element.placeholder = t(element.dataset.i18nPlaceholder || "", lang); });
    document.querySelectorAll<HTMLElement>("[data-i18n-title]").forEach((element) => { element.title = t(element.dataset.i18nTitle || "", lang); });
    const titleKey = document.body?.dataset.titleKey;
    if (titleKey) document.title = `TracePulse - ${t(titleKey, lang)}`;
    window.applyPageLanguage?.(lang);
}

export function currentTimeZoneSetting(): string {
    try { return localStorage.getItem("tracepulse-timezone") || "utc"; } catch { return "utc"; }
}

export function formatTracePulseTimestamp(value: unknown): string {
    const date = value instanceof Date ? value : parseTracePulseTime(value);
    if (Number.isNaN(date.getTime())) return value ? String(value) : "-";
    const parts = new Intl.DateTimeFormat("en-US", { timeZone: currentTimeZoneSetting() === "jst" ? "Asia/Tokyo" : "UTC", month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false }).formatToParts(date);
    const map: Record<string, string> = {};
    parts.forEach((part) => { if (part.type !== "literal") map[part.type] = part.value; });
    return `${map.month}/${map.day} ${map.hour}:${map.minute}:${map.second}`;
}

export function parseTracePulseTime(value: unknown): Date {
    if (typeof value !== "string" || !value) return new Date(NaN);
    if (/Z$|[+-]\d\d:\d\d$/.test(value)) return new Date(value);
    return new Date(`${value.replace(" ", "T")}Z`);
}

export function formatTracePulseClock(value: unknown): string {
    const date = value instanceof Date ? value : parseTracePulseTime(value);
    if (Number.isNaN(date.getTime())) return "-";
    const parts = new Intl.DateTimeFormat("en-US", { timeZone: currentTimeZoneSetting() === "jst" ? "Asia/Tokyo" : "UTC", hour: "2-digit", minute: "2-digit", hour12: false }).formatToParts(date);
    const map: Record<string, string> = {};
    parts.forEach((part) => { if (part.type !== "literal") map[part.type] = part.value; });
    return `${map.hour}:${map.minute}`;
}

Object.assign(window, { t, tf, currentLanguage, setLanguage, currentTheme, setTheme, applyTheme, applyLanguage, formatTracePulseTimestamp, formatTracePulseClock });

document.addEventListener("DOMContentLoaded", () => {
    applyTheme();
    applyLanguage();
    document.querySelectorAll<HTMLElement>("[data-timestamp]").forEach((element) => { element.textContent = formatTracePulseTimestamp(element.dataset.timestamp); });
});
