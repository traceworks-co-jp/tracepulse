import { esc } from "./escape";
import { addSecretVisibility } from "./secret-visibility";

export interface SnmpCredentials {
    version?: string; community?: string; v3_username?: string;
    v3_auth_protocol?: string; v3_auth_password?: string;
    v3_priv_protocol?: string; v3_priv_password?: string;
}

const japanese = (): boolean => document.documentElement.lang === "ja";
const text = (key: string, fallback: string): string => window.t?.(key) || fallback;
export const credentialTitle = (): string => text("snmp_credential_title", japanese() ? "SNMP認証情報" : "SNMP credentials");

export function credentialFields(prefix: string, global = false, includeCommunity = true): string {
    if (!document.getElementById("snmp-credential-styles")) {
        const style = document.createElement("style");
        style.id = "snmp-credential-styles";
        style.textContent = `.snmp-credential-fields{display:grid;gap:12px;min-width:0;border:0;padding:0;margin:12px 0}
            .snmp-credential-fields label{display:grid;gap:6px;font-size:.88rem;min-width:0}
            .snmp-credential-fields input,.snmp-credential-fields select{box-sizing:border-box;width:100%;min-width:0;padding:8px;border:1px solid var(--tp-border,#475569);border-radius:4px;background:var(--tp-bg,#0f172a);color:var(--tp-text,#e2e8f0)}
            .snmp-credential-fields p{margin:0;font-size:.82rem;overflow-wrap:anywhere}
            #snmp-connection-form{max-width:680px}#snmp-connection-form .field-row{grid-template-columns:1fr}
            #snmp-connection-form .secret-input input{box-sizing:border-box;width:100%}
            #manual-box .snmp-credential-fields{flex:1 1 100%}
            html[data-theme=light] .snmp-credential-fields input,html[data-theme=light] .snmp-credential-fields select{background:#fff;color:#0f172a;border-color:#cbd5e1}`;
        document.head.append(style);
    }
    const hintKey = global ? "snmp_credential_keep_hint" : "snmp_credential_inherit_hint";
    const hint = text(hintKey, global ? "Leave blank to keep existing defaults." : "Blank fields inherit global defaults. Saving replaces the current device overrides.");
    const optionKey = global ? "snmp_credential_unchanged" : "snmp_credential_global";
    const label = (key: keyof SnmpCredentials, fallback: string): string => `<span data-i18n="snmp_credential_${key}">${esc(text(`snmp_credential_${key}`, fallback))}</span>`;
    const input = (key: keyof SnmpCredentials, fallback: string): string => `<label for="${prefix}-${key}">${label(key, fallback)}<input id="${prefix}-${key}" data-snmp="${key}" type="password" autocomplete="new-password" maxlength="1024" placeholder="${esc(hint)}" data-i18n-placeholder="${hintKey}" aria-describedby="${prefix}-hint"></label>`;
    const protocolLabels: Record<string, string> = { "2c": "SNMPv2c", "3": "SNMPv3", sha1: "SHA-1", sha256: "SHA-256", sha512: "SHA-512", aes128: "AES-128" };
    const select = (key: keyof SnmpCredentials, fallback: string, options: string[]): string => `<label for="${prefix}-${key}">${label(key, fallback)}<select id="${prefix}-${key}" data-snmp="${key}"><option value="" data-i18n="${optionKey}">${esc(text(optionKey, global ? "Unchanged" : "Global default"))}</option>${options.map(value => value === "none"
        ? `<option value="none" data-i18n="snmp_credential_none">${esc(text("snmp_credential_none", "None"))}</option>`
        : `<option value="${value}">${esc(protocolLabels[value] || value)}</option>`).join("")}</select></label>`;
    return `<fieldset class="snmp-credential-fields">
        <legend data-i18n="snmp_credential_title">${esc(credentialTitle())}</legend><p id="${prefix}-hint" data-i18n="${hintKey}">${esc(hint)}</p>
        ${select("version", "SNMP version", ["2c", "3"])}
        ${includeCommunity ? input("community", "Community (SNMPv2c)") : ""}
        ${input("v3_username", "v3 username")}
        ${select("v3_auth_protocol", "v3 authentication", ["sha256", "sha512", "sha1", "none"])}
        ${input("v3_auth_password", "v3 authentication password")}
        ${select("v3_priv_protocol", "v3 privacy", ["aes128", "none"])}
        ${input("v3_priv_password", "v3 privacy password")}
    </fieldset>`;
}

export function readCredentials(container: ParentNode): SnmpCredentials {
    const credentials: SnmpCredentials = {};
    container.querySelectorAll<HTMLInputElement | HTMLSelectElement>("[data-snmp]").forEach(input => {
        if (input.value) credentials[input.dataset.snmp as keyof SnmpCredentials] = input.value;
    });
    return credentials;
}

export function clearCredentials(container: ParentNode): void {
    container.querySelectorAll<HTMLInputElement | HTMLSelectElement>("[data-snmp]").forEach(input => { input.value = ""; });
}

export function initializeGlobalSnmp(): void {
    const form = document.getElementById("snmp-connection-form");
    if (!(form instanceof HTMLFormElement)) return;
    const community = form.querySelector<HTMLInputElement>("#community")!;
    community.dataset.snmp = "community";
    addSecretVisibility(community);
    const button = form.querySelector<HTMLButtonElement>("[type=submit]")!;
    button.insertAdjacentHTML("beforebegin", credentialFields("global-snmp", true, false));
    form.onsubmit = async event => {
        event.preventDefault();
        if (button.disabled) return;
        const credentials = readCredentials(form);
        const { community: defaultCommunity, ...v3Credentials } = credentials;
        const payload = { default_community: defaultCommunity || "", ...(Object.keys(v3Credentials).length ? { snmp_credentials: v3Credentials } : {}) };
        button.disabled = true;
        const controls = form.querySelectorAll<HTMLInputElement | HTMLSelectElement>("input,select");
        controls.forEach(control => { control.disabled = true; });
        const status = form.querySelector<HTMLElement>("[role=status]")!;
        const error = form.querySelector<HTMLElement>("[role=alert]")!;
        status.textContent = ""; error.textContent = "";
        try {
            const response = await fetch("/api/settings", { method: "POST", headers: { "Content-Type": "application/json", ...window.tracepulseRequestHeaders?.() }, body: JSON.stringify(payload) });
            const result: { ok?: boolean; error?: string } = await response.json();
            if (!response.ok || !result.ok) throw new Error(result.error || `HTTP ${response.status}`);
            clearCredentials(form);
            if (community.type === "text") form.querySelector<HTMLButtonElement>("button[aria-controls=community]")?.click();
            status.textContent = window.t?.("settings_saved") || "Saved";
        } catch (reason) { error.textContent = String(reason); }
        finally { button.disabled = false; controls.forEach(control => { control.disabled = false; }); }
    };
}