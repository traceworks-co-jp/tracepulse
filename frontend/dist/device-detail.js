"use strict";
var TracePulseDeviceDetail = (() => {
  var __defProp = Object.defineProperty;
  var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __hasOwnProp = Object.prototype.hasOwnProperty;
  var __export = (target, all) => {
    for (var name in all)
      __defProp(target, name, { get: all[name], enumerable: true });
  };
  var __copyProps = (to, from, except, desc) => {
    if (from && typeof from === "object" || typeof from === "function") {
      for (let key of __getOwnPropNames(from))
        if (!__hasOwnProp.call(to, key) && key !== except)
          __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
    }
    return to;
  };
  var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

  // frontend/src/pages/device-detail.ts
  var device_detail_exports = {};
  __export(device_detail_exports, {
    isInterfaceSelected: () => isInterfaceSelected,
    loadDeviceDetail: () => loadDeviceDetail,
    onErrorBreakdownIfaceChange: () => onErrorBreakdownIfaceChange,
    refreshTrafficProtocols: () => refreshTrafficProtocols,
    renderAlerts: () => renderAlerts,
    renderBwChart: () => renderBwChart,
    renderCpuChart: () => renderCpuChart,
    renderDetail: () => renderDetail,
    renderErrChart: () => renderErrChart,
    renderErrorBreakdownCard: () => renderErrorBreakdownCard,
    renderHardwareStatus: () => renderHardwareStatus,
    renderInterfaces: () => renderInterfaces,
    renderMemoryChart: () => renderMemoryChart,
    renderSpikes: () => renderSpikes,
    renderSysChart: () => renderSysChart,
    renderTemplateMetrics: () => renderTemplateMetrics,
    renderTrafficProtocols: () => renderTrafficProtocols,
    selectAllInterfaces: () => selectAllInterfaces,
    sparkline: () => sparkline,
    toggleInterfaceSelection: () => toggleInterfaceSelection
  });

  // frontend/src/shared/escape.ts
  function esc(value) {
    return String(value === null || value === void 0 ? "" : value).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/\x22/g, "&quot;").replace(/\x27/g, "&#39;");
  }

  // frontend/src/shared/secret-visibility.ts
  window.addEventListener("tracepulse-language-change", () => {
    document.querySelectorAll(".secret-toggle").forEach((button) => {
      const key = button.getAttribute("aria-pressed") === "true" ? "hide_community" : "show_community";
      const label = window.t?.(key);
      if (label) {
        button.setAttribute("aria-label", label);
        button.title = label;
      }
    });
  });

  // frontend/src/shared/snmp-credentials.ts
  var japanese = () => document.documentElement.lang === "ja";
  var text = (key, fallback) => window.t?.(key) || fallback;
  var credentialTitle = () => text("snmp_credential_title", japanese() ? "SNMP\u8A8D\u8A3C\u60C5\u5831" : "SNMP credentials");
  function credentialFields(prefix, global = false, includeCommunity = true) {
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
    const label = (key, fallback) => `<span data-i18n="snmp_credential_${key}">${esc(text(`snmp_credential_${key}`, fallback))}</span>`;
    const input = (key, fallback) => `<label for="${prefix}-${key}">${label(key, fallback)}<input id="${prefix}-${key}" data-snmp="${key}" type="password" autocomplete="new-password" maxlength="1024" placeholder="${esc(hint)}" data-i18n-placeholder="${hintKey}" aria-describedby="${prefix}-hint"></label>`;
    const protocolLabels = { "2c": "SNMPv2c", "3": "SNMPv3", sha1: "SHA-1", sha256: "SHA-256", sha512: "SHA-512", aes128: "AES-128" };
    const select = (key, fallback, options) => `<label for="${prefix}-${key}">${label(key, fallback)}<select id="${prefix}-${key}" data-snmp="${key}"><option value="" data-i18n="${optionKey}">${esc(text(optionKey, global ? "Unchanged" : "Global default"))}</option>${options.map((value) => value === "none" ? `<option value="none" data-i18n="snmp_credential_none">${esc(text("snmp_credential_none", "None"))}</option>` : `<option value="${value}">${esc(protocolLabels[value] || value)}</option>`).join("")}</select></label>`;
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
  function readCredentials(container) {
    const credentials = {};
    container.querySelectorAll("[data-snmp]").forEach((input) => {
      if (input.value) credentials[input.dataset.snmp] = input.value;
    });
    return credentials;
  }
  function clearCredentials(container) {
    container.querySelectorAll("[data-snmp]").forEach((input) => {
      input.value = "";
    });
  }

  // frontend/src/shared/operation-progress.ts
  function showOperationProgress(container, label, message) {
    if (!document.getElementById("operation-progress-styles")) {
      const style = document.createElement("style");
      style.id = "operation-progress-styles";
      style.textContent = `.operation-progress{box-sizing:border-box;flex-basis:100%;width:100%;padding:12px;border:1px solid var(--tp-border,#475569);border-radius:4px;background:var(--tp-raised,#1a2b40);color:var(--tp-text,#e2e8f0)}.operation-progress progress{display:block;width:100%;height:8px;accent-color:var(--tp-accent,#38c5b2)}.operation-progress p{margin:8px 0 0;overflow-wrap:anywhere}`;
      document.head.append(style);
    }
    const element = document.createElement("div");
    element.className = "operation-progress";
    const bar = document.createElement("progress");
    bar.setAttribute("aria-label", label);
    const status = document.createElement("p");
    status.setAttribute("role", "status");
    status.setAttribute("aria-live", "polite");
    element.append(bar, status);
    container.append(element);
    const started = performance.now();
    const update = () => {
      status.textContent = message(Math.floor((performance.now() - started) / 1e3));
    };
    update();
    const timer = window.setInterval(update, 1e3);
    return () => {
      window.clearInterval(timer);
      element.remove();
    };
  }

  // frontend/src/shared/device-metadata.ts
  if (!document.getElementById("metadata-styles")) {
    const style = document.createElement("style");
    style.id = "metadata-styles";
    style.textContent = `
        .metadata-dialog{box-sizing:border-box;width:min(640px,calc(100% - 24px));max-height:calc(100dvh - 24px);overflow:auto;margin:auto;padding:24px;border:1px solid var(--tp-border,#475569);border-radius:8px;background:var(--tp-surface,#1e293b);color:var(--tp-text,#e2e8f0)}
        .metadata-dialog::backdrop{background:rgb(0 0 0 / .6)}
        .metadata-dialog form,.metadata-dialog label{display:grid;gap:8px}
        .metadata-dialog p,.metadata-dialog li{overflow-wrap:anywhere}
        .metadata-dialog input,.metadata-dialog select,.device-toolbar input{box-sizing:border-box;width:100%;padding:8px;background:var(--tp-bg,#0f172a);color:inherit;border:1px solid #64748b;border-radius:4px;min-width:0}
        .metadata-dialog button,.device-toolbar button,.device-action-button,#edit-device-metadata{padding:8px 12px;cursor:pointer;background:var(--tp-bg,#0f172a);color:inherit;border:1px solid #64748b;border-radius:4px}
        .metadata-dialog button:disabled,.device-toolbar button:disabled{opacity:.5;cursor:default}
        .metadata-actions,.device-toolbar,.device-tags{display:flex;gap:8px;flex-wrap:wrap;align-items:center}
        .metadata-actions{justify-content:flex-end;margin-top:12px}.device-toolbar{margin:12px 0}.device-toolbar input{flex:1 1 220px}
        .bulk-action-progress{margin-top:12px;padding:12px;border:1px solid var(--tp-border,#475569);border-radius:4px;background:var(--tp-raised,#1a2b40)}
        .bulk-action-progress[hidden]{display:none}.bulk-action-progress progress{display:block;width:100%;height:8px;accent-color:var(--tp-accent,#38c5b2)}
        .bulk-action-progress p{margin:8px 0 0}.bulk-action-progress small{color:var(--tp-muted,#94a3b8)}
        .device-tag{display:inline-flex;align-items:center;gap:4px;padding:2px 7px;background:var(--tp-bg,#0f172a);border:1px solid #64748b;border-radius:12px;font-size:.8rem;overflow-wrap:anywhere}
        .device-tags{margin-top:4px}.device-tag button{padding:0 4px}.device-original{font-size:.8rem;color:var(--tp-muted,#94a3b8);overflow-wrap:anywhere}
        .metadata-dialog [role=alert],.device-toolbar [role=alert]{color:#f87171}
        :root[data-theme=light] .metadata-dialog{background:#fff;color:#1e293b}
        :root[data-theme=light] .metadata-dialog input,:root[data-theme=light] .metadata-dialog select,:root[data-theme=light] .metadata-dialog button,:root[data-theme=light] .device-toolbar input,:root[data-theme=light] .device-toolbar button,:root[data-theme=light] .device-action-button,:root[data-theme=light] .device-tag,:root[data-theme=light] #edit-device-metadata{background:#f1f5f9;color:#1e293b}
    `;
    document.head.append(style);
  }
  function deviceLabel(device) {
    if (device.display_name?.trim()) return device.display_name;
    if (device.name?.trim()) return device.name;
    return device.ip || "";
  }
  var copy = {
    en: {
      edit: "Edit alias and tags",
      alias: "Display name",
      tags: "Tags",
      add: "Add tag",
      remove: "Remove",
      cancel: "Cancel",
      save: "Save",
      rename: "Bulk rename",
      prefix: "Prefix",
      start: "Start number",
      padding: "Padding",
      suffix: "Suffix",
      order: "Sort order",
      ip: "IP ascending",
      selection: "Selection order",
      preview: "Preview",
      apply: "Apply names",
      search: "Search alias, hostname, IP or tags",
      select: "Select device",
      all: "Select visible devices",
      selected: "selected",
      failed: "Request failed",
      missing: "Select at least one device",
      tagHint: "Enter a tag, then press Add tag or Enter.",
      clear: "Leave the display name empty to use sysName or IP.",
      bulk: "Bulk actions",
      actions: "Actions",
      bulkTags: "Add tags",
      bulkNames: "Set / change device names",
      unregister: "Unregister",
      targets: "Selected devices",
      appendHint: "Existing tags are retained. Duplicate tags are ignored.",
      unregisterHint: "Unregister these devices and delete their monitoring history? This cannot be undone.",
      unregistering: "Unregistering {count} devices... ({seconds}s elapsed)",
      tagging: "Adding tags to {count} devices... ({seconds}s elapsed)",
      unregisterWait: "Deleting monitoring history may take some time. Please keep this page open.",
      confirm: "Confirm"
    },
    ja: {
      edit: "\u8868\u793A\u540D\u30FB\u30BF\u30B0\u3092\u7DE8\u96C6",
      alias: "\u8868\u793A\u540D",
      tags: "\u30BF\u30B0",
      add: "\u30BF\u30B0\u3092\u8FFD\u52A0",
      remove: "\u524A\u9664",
      cancel: "\u30AD\u30E3\u30F3\u30BB\u30EB",
      save: "\u4FDD\u5B58",
      rename: "\u4E00\u62EC\u30EA\u30CD\u30FC\u30E0\uFF08\u9023\u756A\u4ED8\u4E0E\uFF09",
      prefix: "\u63A5\u982D\u8F9E",
      start: "\u958B\u59CB\u756A\u53F7",
      padding: "\u30BC\u30ED\u57CB\u3081\u6841\u6570",
      suffix: "\u63A5\u5C3E\u8F9E",
      order: "\u4E26\u3073\u9806",
      ip: "IP\u30A2\u30C9\u30EC\u30B9\u6607\u9806",
      selection: "\u9078\u629E\u9806",
      preview: "\u30D7\u30EC\u30D3\u30E5\u30FC",
      apply: "\u540D\u79F0\u3092\u9069\u7528",
      search: "\u8868\u793A\u540D\u30FB\u30DB\u30B9\u30C8\u540D\u30FBIP\u30FB\u30BF\u30B0\u3067\u691C\u7D22",
      select: "\u6A5F\u5668\u3092\u9078\u629E",
      all: "\u8868\u793A\u4E2D\u306E\u6A5F\u5668\u3092\u3059\u3079\u3066\u9078\u629E",
      selected: "\u4EF6\u9078\u629E",
      failed: "\u51E6\u7406\u306B\u5931\u6557\u3057\u307E\u3057\u305F",
      missing: "\u6A5F\u5668\u3092\u9078\u629E\u3057\u3066\u304F\u3060\u3055\u3044",
      tagHint: "\u30BF\u30B0\u3092\u5165\u529B\u3057\u3001\u8FFD\u52A0\u30DC\u30BF\u30F3\u307E\u305F\u306FEnter\u3067\u78BA\u5B9A\u3057\u307E\u3059\u3002",
      clear: "\u8868\u793A\u540D\u3092\u7A7A\u306B\u3059\u308B\u3068sysName\u307E\u305F\u306FIP\u3092\u8868\u793A\u3057\u307E\u3059\u3002",
      bulk: "\u4E00\u62EC\u64CD\u4F5C",
      actions: "\u64CD\u4F5C",
      bulkTags: "\u30BF\u30B0\u4ED8\u3051",
      bulkNames: "\u6A5F\u5668\u540D\u306E\u8A2D\u5B9A\u30FB\u5909\u66F4",
      unregister: "\u767B\u9332\u89E3\u9664",
      targets: "\u9078\u629E\u3057\u305F\u6A5F\u5668",
      appendHint: "\u65E2\u5B58\u30BF\u30B0\u3092\u4FDD\u6301\u3057\u3066\u8FFD\u52A0\u3057\u307E\u3059\u3002\u91CD\u8907\u3059\u308B\u30BF\u30B0\u306F\u8FFD\u52A0\u3057\u307E\u305B\u3093\u3002",
      unregisterHint: "\u4EE5\u4E0B\u306E\u6A5F\u5668\u306E\u767B\u9332\u3068\u76E3\u8996\u5C65\u6B74\u3092\u524A\u9664\u3057\u307E\u3059\u3002\u5143\u306B\u623B\u305B\u307E\u305B\u3093\u3002\u5B9F\u884C\u3057\u307E\u3059\u304B\uFF1F",
      unregistering: "{count}\u53F0\u306E\u767B\u9332\u3092\u89E3\u9664\u3057\u3066\u3044\u307E\u3059...\uFF08\u7D4C\u904E {seconds}\u79D2\uFF09",
      tagging: "{count}\u53F0\u306B\u30BF\u30B0\u3092\u8FFD\u52A0\u3057\u3066\u3044\u307E\u3059...\uFF08\u7D4C\u904E {seconds}\u79D2\uFF09",
      unregisterWait: "\u76E3\u8996\u5C65\u6B74\u306E\u524A\u9664\u306B\u306F\u6642\u9593\u304C\u304B\u304B\u308B\u5834\u5408\u304C\u3042\u308A\u307E\u3059\u3002\u3053\u306E\u30DA\u30FC\u30B8\u3092\u958B\u3044\u305F\u307E\u307E\u304A\u5F85\u3061\u304F\u3060\u3055\u3044\u3002",
      confirm: "\u78BA\u5B9A"
    }
  };
  function metadataText(key) {
    return copy[document.documentElement.lang === "ja" ? "ja" : "en"][key];
  }
  async function post(url, body) {
    const response = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json", "X-CSRF-Token": window.tracepulseCsrf || "" },
      body: JSON.stringify(body)
    });
    const data = await response.json();
    if (!response.ok || data.error || data.ok !== true) throw new Error(data.error || metadataText("failed"));
    return data;
  }
  function dialog(id, title) {
    const existing = document.getElementById(id);
    if (existing instanceof HTMLDialogElement) existing.remove();
    const element = document.createElement("dialog");
    element.id = id;
    element.className = "metadata-dialog";
    element.setAttribute("aria-labelledby", `${id}-title`);
    element.innerHTML = `<h2 id="${id}-title">${esc(title)}</h2>`;
    document.body.append(element);
    return element;
  }
  function field(id, label, value, type = "text", limits = "") {
    return `<label for="${id}">${esc(label)}<input id="${id}" type="${type}" value="${esc(value)}" ${limits}></label>`;
  }
  function editDeviceMetadata(device, saved) {
    if (!device.ip) return;
    const element = dialog("device-metadata-dialog", metadataText("edit"));
    let tags = [...device.tags || []];
    element.insertAdjacentHTML("beforeend", `<form>
        <p>${esc(device.name || "")} (${esc(device.ip)})</p>
        ${field("device-alias", metadataText("alias"), device.display_name || "", "text", "maxlength=512")}
        <p>${esc(metadataText("clear"))}</p>
        <label for="device-tag-input">${esc(metadataText("tags"))}</label>
        <div id="device-tag-chips" class="device-tags"></div>
        <input id="device-tag-input" maxlength="64" aria-describedby="device-tag-hint">
        <p id="device-tag-hint">${esc(metadataText("tagHint"))}</p>
        <button type="button" id="device-add-tag">${esc(metadataText("add"))}</button>
        <button type="button" id="device-snmp-edit">${esc(credentialTitle())}</button>
        <p role="alert"></p><div class="metadata-actions">
        <button type="button" data-cancel>${esc(metadataText("cancel"))}</button>
        <button type="submit">${esc(metadataText("save"))}</button></div></form>`);
    const input = element.querySelector("#device-tag-input");
    element.querySelector("#device-snmp-edit").onclick = () => editSnmpCredentials([device], saved);
    const error = element.querySelector("[role=alert]");
    const renderTags = () => {
      const chips = element.querySelector("#device-tag-chips");
      chips.replaceChildren();
      tags.forEach((tag) => {
        const chip = document.createElement("span");
        chip.className = "device-tag";
        chip.append(document.createTextNode(tag));
        const remove = document.createElement("button");
        remove.type = "button";
        remove.textContent = "\xD7";
        remove.setAttribute("aria-label", `${metadataText("remove")} ${tag}`);
        remove.onclick = () => {
          tags = tags.filter((value) => value !== tag);
          renderTags();
        };
        chip.append(remove);
        chips.append(chip);
      });
    };
    const addTag = () => {
      const tag = input.value.trim();
      if (tag && !tags.includes(tag)) tags.push(tag);
      input.value = "";
      renderTags();
    };
    element.querySelector("#device-add-tag").onclick = addTag;
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter" && !event.isComposing) {
        event.preventDefault();
        addTag();
      }
    });
    element.querySelector("[data-cancel]").onclick = () => element.close();
    element.querySelector("form").onsubmit = async (event) => {
      event.preventDefault();
      addTag();
      const submit = element.querySelector("[type=submit]");
      submit.disabled = true;
      error.textContent = "";
      try {
        await post(`/api/device/${encodeURIComponent(device.ip)}/metadata`, {
          display_name: element.querySelector("#device-alias").value || null,
          tags
        });
        element.close();
        saved();
      } catch (failure) {
        error.textContent = String(failure);
      } finally {
        submit.disabled = false;
      }
    };
    renderTags();
    element.showModal();
  }
  function editSnmpCredentials(devices, saved) {
    const element = dialog("device-snmp-dialog", credentialTitle());
    element.insertAdjacentHTML("beforeend", `<form><p>${esc(metadataText("targets"))}: ${devices.length}</p>
        <ul>${devices.map((device) => `<li>${esc(device.ip || "")} ${esc(deviceLabel(device))}</li>`).join("")}</ul>
        ${credentialFields("device-snmp")}
        <p role="status" aria-live="polite"></p>
        <p role="alert"></p><div class="metadata-actions"><button type="button" data-reset>${document.documentElement.lang === "ja" ? "\u5171\u901A\u8A2D\u5B9A\u3078\u623B\u3059" : "Use global defaults"}</button>
        <button type="button" data-cancel>${esc(metadataText("cancel"))}</button><button type="submit">${esc(metadataText("save"))}</button></div></form>`);
    element.querySelector("[data-cancel]").onclick = () => {
      clearCredentials(element);
      element.close();
    };
    let busy = false;
    let loading = false;
    const individual = devices.length === 1 && Boolean(devices[0].ip);
    const japanese2 = document.documentElement.lang === "ja";
    const status = element.querySelector("[role=status]");
    const failure = element.querySelector("[role=alert]");
    const controls = element.querySelectorAll("input,select,button");
    const setBusy = (value) => {
      busy = value;
      controls.forEach((control) => {
        control.disabled = value;
      });
    };
    const controller = new AbortController();
    let stopSaveProgress;
    element.addEventListener("cancel", (event) => {
      if (busy && !loading) event.preventDefault();
    });
    element.addEventListener("close", () => {
      stopSaveProgress?.();
      controller.abort();
      clearCredentials(element);
    });
    const persist = async (defaults) => {
      if (busy) return;
      const credentials = defaults ? {} : readCredentials(element);
      setBusy(true);
      failure.textContent = "";
      const progressContainer = document.createElement("div");
      failure.before(progressContainer);
      const stopProgress = showOperationProgress(progressContainer, credentialTitle(), (seconds) => {
        const japanese3 = document.documentElement.lang === "ja";
        return japanese3 ? `${devices.length}\u53F0\u306E${defaults ? "\u8A8D\u8A3C\u60C5\u5831\u3092\u5171\u901A\u8A2D\u5B9A\u3078\u623B\u3057\u3066\u3044\u307E\u3059" : "SNMP\u8A8D\u8A3C\u60C5\u5831\u3092\u4FDD\u5B58\u3057\u3066\u3044\u307E\u3059"}...\uFF08\u7D4C\u904E ${seconds}\u79D2\uFF09` : `${defaults ? "Restoring global defaults for" : "Saving SNMP credentials for"} ${devices.length} devices... (${seconds}s elapsed)`;
      });
      stopSaveProgress = () => {
        stopProgress();
        progressContainer.remove();
      };
      progressContainer.scrollIntoView({ block: "nearest" });
      try {
        await post("/api/devices/snmp-credentials", { device_ids: devices.map((device) => device.id), credentials, preserve_secrets: !defaults && individual });
        clearCredentials(element);
        element.close();
        saved();
      } catch (reason) {
        failure.textContent = String(reason);
      } finally {
        stopSaveProgress?.();
        stopSaveProgress = void 0;
        setBusy(false);
      }
    };
    element.querySelector("[data-reset]").onclick = () => {
      if (window.confirm(japanese2 ? "\u500B\u5225\u306ESNMP\u8A8D\u8A3C\u60C5\u5831\u3092\u524A\u9664\u3057\u3001\u5171\u901A\u8A2D\u5B9A\u3078\u623B\u3057\u307E\u3059\u304B\uFF1F" : "Remove the device SNMP overrides and use global defaults?")) void persist(true);
    };
    element.querySelector("form").onsubmit = (event) => {
      event.preventDefault();
      void persist(false);
    };
    element.showModal();
    if (individual) {
      loading = true;
      setBusy(true);
      element.querySelector("[data-cancel]").disabled = false;
      status.textContent = japanese2 ? "\u8A2D\u5B9A\u3092\u8AAD\u307F\u8FBC\u3093\u3067\u3044\u307E\u3059\u2026" : "Loading settings...";
      const hint = element.querySelector("#device-snmp-hint");
      hint.removeAttribute("data-i18n");
      hint.textContent = japanese2 ? "\u4FDD\u5B58\u6E08\u307F\u306E\u30B3\u30DF\u30E5\u30CB\u30C6\u30A3\u3068\u30D1\u30B9\u30EF\u30FC\u30C9\u306F\u8868\u793A\u3057\u307E\u305B\u3093\u3002\u7A7A\u6B04\u306E\u307E\u307E\u4FDD\u5B58\u3059\u308B\u3068\u65E2\u5B58\u5024\u3092\u7DAD\u6301\u3057\u307E\u3059\u3002" : "Stored communities and passwords are not displayed. Leave them blank to keep existing values.";
      void (async () => {
        try {
          const response = await fetch(`/api/device/${encodeURIComponent(devices[0].ip)}/snmp-credentials`, { signal: controller.signal });
          const data = await response.json();
          if (!response.ok || !data.ok || !data.devices) throw new Error(data.error || metadataText("failed"));
          const view = data.devices;
          for (const key of ["version", "v3_username", "v3_auth_protocol", "v3_priv_protocol"]) {
            const input = element.querySelector(`[data-snmp=${key}]`);
            input.value = view.credentials[key] || "";
            if (key === "v3_username" && input instanceof HTMLInputElement) {
              input.type = "text";
              input.autocomplete = "off";
            }
          }
          for (const key of ["community", "v3_auth_password", "v3_priv_password"]) {
            const input = element.querySelector(`[data-snmp=${key}]`);
            input.removeAttribute("data-i18n-placeholder");
            input.placeholder = view.secret_configured[key] ? japanese2 ? "\u8A2D\u5B9A\u6E08\u307F\uFF08\u5909\u66F4\u3059\u308B\u5834\u5408\u306E\u307F\u5165\u529B\uFF09" : "Configured (enter only to replace)" : japanese2 ? "\u7A7A\u6B04\u306E\u5834\u5408\u306F\u5171\u901A\u8A2D\u5B9A\u3092\u4F7F\u7528" : "Blank: use global defaults";
          }
          status.textContent = view.individual_configured ? japanese2 ? "\u73FE\u5728\u306E\u8A2D\u5B9A\uFF1A\u30C7\u30D0\u30A4\u30B9\u500B\u5225\u8A2D\u5B9A" : "Current settings: device overrides" : japanese2 ? "\u73FE\u5728\u306E\u8A2D\u5B9A\uFF1A\u5171\u901A\u8A2D\u5B9A\u3092\u4F7F\u7528" : "Current settings: global defaults";
          loading = false;
          setBusy(false);
        } catch (reason) {
          if (controller.signal.aborted) return;
          status.textContent = "";
          failure.textContent = String(reason);
          element.querySelector("[data-cancel]").disabled = false;
        }
      })();
    }
  }

  // frontend/src/shared/status.ts
  function interfaceStatusClass(status) {
    switch (status?.trim().toLowerCase()) {
      case "up":
        return "link-up";
      case "down":
      case "lowerlayerdown":
        return "link-down";
      case "testing":
        return "status-warning";
      default:
        return "status-unknown";
    }
  }

  // frontend/src/shared/format.ts
  function formatBps(value) {
    const numericValue = Number(value || 0);
    if (numericValue >= 1e9) return `${(numericValue / 1e9).toFixed(1)} Gbps`;
    if (numericValue >= 1e6) return `${(numericValue / 1e6).toFixed(1)} Mbps`;
    if (numericValue >= 1e3) return `${(numericValue / 1e3).toFixed(1)} Kbps`;
    return `${Math.round(numericValue)} bps`;
  }
  function fmtNum(value) {
    return new Intl.NumberFormat("en-US").format(Number(value || 0));
  }
  function parseTracePulseTime(iso) {
    if (typeof iso !== "string" || iso.length === 0) return /* @__PURE__ */ new Date(NaN);
    if (/Z$|[+-]\d\d:\d\d$/.test(iso)) return new Date(iso);
    return /* @__PURE__ */ new Date(`${iso.replace(" ", "T")}Z`);
  }
  function tracePulseTimeZone() {
    try {
      return localStorage.getItem("tracepulse-timezone") === "jst" ? "Asia/Tokyo" : "UTC";
    } catch {
      return "UTC";
    }
  }
  function formatTracePulseTime(value) {
    const parts = new Intl.DateTimeFormat("en-US", {
      timeZone: tracePulseTimeZone(),
      month: "numeric",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false
    }).formatToParts(value);
    const map = {};
    parts.forEach((part) => {
      if (part.type !== "literal") map[part.type] = part.value;
    });
    return `${map.month}/${map.day} ${map.hour}:${map.minute}:${map.second}`;
  }
  function formatTracePulseClock(value) {
    const parts = new Intl.DateTimeFormat("en-US", {
      timeZone: tracePulseTimeZone(),
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false
    }).formatToParts(value);
    const map = {};
    parts.forEach((part) => {
      if (part.type !== "literal") map[part.type] = part.value;
    });
    return `${map.hour}:${map.minute}:${map.second}`;
  }
  function fmtTime(iso) {
    if (!iso) return "-";
    const date = parseTracePulseTime(iso);
    return Number.isNaN(date.getTime()) ? String(iso) : formatTracePulseTime(date);
  }

  // frontend/src/pages/device-detail.ts
  var palette = ["#38bdf8", "#4ade80", "#f59e0b", "#f87171", "#a78bfa", "#34d399"];
  var chartColors = ["#38bdf8", "#4ade80", "#f59e0b", "#f87171", "#a78bfa", "#34d399", "#fb923c", "#e879f9"];
  var analyticsRefreshOptions = [5, 10, 30, 60];
  var analyticsPollingMs = loadAnalyticsPollingMs();
  var deviceRefreshSeconds = 30;
  var errorBreakdownSelectedIf = null;
  var analyticsWindow = "60s";
  var analyticsPaused = false;
  var analyticsTimer = null;
  var deviceRefreshTimer = null;
  var ifaceSelected = /* @__PURE__ */ new Set();
  var lastDeviceDetail = null;
  var ifaceSelectionReady = false;
  var resizeTimer = null;
  var monitoringHistory = null;
  var monitoringRange = "24h";
  var customHistoryStart = "";
  var customHistoryEnd = "";
  var historyController = null;
  var historyGeneration = 0;
  var historyLoading = false;
  var stopHistoryProgress = () => {
  };
  var historyCopy = {
    en: { title: "Monitoring graph period", range: "Time range", hour: "Last hour", six: "Last 6 hours", day: "Last 24 hours", week: "Last 7 days", custom: "Custom range", start: "Start (local time)", end: "End (local time)", apply: "Apply", refresh: "Refresh graphs", invalid: "Choose a valid start and end time, with an end after the start and a range of up to 31 days.", loading: "Loading monitoring history", failed: "Could not load monitoring history" },
    ja: { title: "\u76E3\u8996\u30B0\u30E9\u30D5\u306E\u8868\u793A\u671F\u9593", range: "\u8868\u793A\u671F\u9593", hour: "\u76F4\u8FD11\u6642\u9593", six: "\u76F4\u8FD16\u6642\u9593", day: "\u76F4\u8FD124\u6642\u9593", week: "\u76F4\u8FD17\u65E5", custom: "\u65E5\u6642\u6307\u5B9A", start: "\u958B\u59CB\uFF08\u30ED\u30FC\u30AB\u30EB\u6642\u523B\uFF09", end: "\u7D42\u4E86\uFF08\u30ED\u30FC\u30AB\u30EB\u6642\u523B\uFF09", apply: "\u9069\u7528", refresh: "\u30B0\u30E9\u30D5\u3092\u518D\u8AAD\u8FBC", invalid: "\u958B\u59CB\u30FB\u7D42\u4E86\u65E5\u6642\u3092\u6B63\u3057\u304F\u6307\u5B9A\u3057\u3066\u304F\u3060\u3055\u3044\u3002\u7D42\u4E86\u306F\u958B\u59CB\u3088\u308A\u5F8C\u3001\u6307\u5B9A\u671F\u9593\u306F31\u65E5\u4EE5\u5185\u306B\u3057\u3066\u304F\u3060\u3055\u3044\u3002", loading: "\u76E3\u8996\u5C65\u6B74\u3092\u8AAD\u307F\u8FBC\u3093\u3067\u3044\u307E\u3059", failed: "\u76E3\u8996\u5C65\u6B74\u3092\u8AAD\u307F\u8FBC\u3081\u307E\u305B\u3093\u3067\u3057\u305F" }
  };
  function historyText() {
    return historyCopy[document.documentElement.lang === "ja" ? "ja" : "en"];
  }
  function initializeHistoryControls() {
    const root = document.getElementById("monitoring-history-controls");
    if (!root) return;
    const previousStart = root.querySelector("#monitoring-start")?.value;
    const previousEnd = root.querySelector("#monitoring-end")?.value;
    const copy2 = historyText();
    const localInput = (time) => {
      const date = new Date(time);
      return new Date(time - date.getTimezoneOffset() * 6e4).toISOString().slice(0, 16);
    };
    root.innerHTML = `<h2>${copy2.title}</h2><form class='chart-history-controls'><label for='monitoring-range'>${copy2.range}<select id='monitoring-range'>${[["1h", copy2.hour], ["6h", copy2.six], ["24h", copy2.day], ["7d", copy2.week], ["custom", copy2.custom]].map(([value, label]) => `<option value='${value}'>${label}</option>`).join("")}</select></label><div class='chart-history-custom' hidden><label for='monitoring-start'>${copy2.start}<input id='monitoring-start' type='datetime-local' required></label><label for='monitoring-end'>${copy2.end}<input id='monitoring-end' type='datetime-local' required></label><button class='btn' type='submit'>${copy2.apply}</button></div></form>`;
    const select = root.querySelector("#monitoring-range");
    const start = root.querySelector("#monitoring-start");
    const end = root.querySelector("#monitoring-end");
    select.value = monitoringRange;
    start.value = previousStart ?? localInput(Date.now() - 864e5);
    end.value = previousEnd ?? localInput(Date.now());
    const toggle = () => {
      const custom = select.value === "custom";
      root.querySelector(".chart-history-custom").hidden = !custom;
      start.disabled = end.disabled = !custom;
    };
    toggle();
    const refresh = document.createElement("button");
    refresh.id = "monitoring-history-refresh";
    refresh.type = "button";
    refresh.className = "btn btn-secondary";
    refresh.textContent = copy2.refresh;
    refresh.onclick = () => {
      if (monitoringRange === "custom") root.querySelector("form").requestSubmit();
      else void loadMonitoringHistory(true);
    };
    root.querySelector("form").append(refresh);
    select.onchange = () => {
      monitoringRange = select.value;
      toggle();
      if (monitoringRange !== "custom") void loadMonitoringHistory(true);
    };
    root.querySelector("form").onsubmit = (event) => {
      event.preventDefault();
      const from = new Date(start.value), to = new Date(end.value);
      if (!Number.isFinite(from.getTime()) || !Number.isFinite(to.getTime()) || to <= from || to.getTime() - from.getTime() > 31 * 864e5) {
        document.getElementById("monitoring-history-error").textContent = historyText().invalid;
        return;
      }
      customHistoryStart = from.toISOString();
      customHistoryEnd = to.toISOString();
      void loadMonitoringHistory(true);
    };
  }
  function renderMonitoringCharts() {
    const selected = document.getElementById("monitoring-history-controls") ? monitoringHistory : lastDeviceDetail;
    renderBwChart(selected?.if_series || []);
    renderErrChart(selected?.if_series || []);
    renderSysChart(selected?.metrics || []);
  }
  async function loadMonitoringHistory(force = false) {
    const host = document.getElementById("monitoring-history-progress");
    if (!host || historyLoading && !force || monitoringRange === "custom" && (!customHistoryStart || !customHistoryEnd)) return;
    const generation = ++historyGeneration;
    historyController?.abort();
    stopHistoryProgress();
    const controller = new AbortController();
    historyController = controller;
    historyLoading = true;
    const error = document.getElementById("monitoring-history-error");
    error.textContent = "";
    const stop = showOperationProgress(host, historyText().title, (seconds) => `${historyText().loading}... ${document.documentElement.lang === "ja" ? `\uFF08\u7D4C\u904E ${seconds}\u79D2\uFF09` : `(${seconds}s elapsed)`}`);
    stopHistoryProgress = stop;
    const params = new URLSearchParams({ range: monitoringRange });
    if (monitoringRange === "custom") {
      params.set("start", customHistoryStart);
      params.set("end", customHistoryEnd);
    }
    try {
      const response = await fetch(`/api/device/${encodeURIComponent(window.DEVICE_IP || "")}/history?${params}`, { cache: "no-store", signal: controller.signal });
      const history = await response.json();
      if (!response.ok || history.error) throw new Error(history.error || `HTTP ${response.status}`);
      if (generation !== historyGeneration) return;
      monitoringHistory = history;
      document.getElementById("monitoring-history-period").textContent = `${fmtTime(history.start)} - ${fmtTime(history.end)}`;
      renderMonitoringCharts();
    } catch (reason) {
      if (!controller.signal.aborted && generation === historyGeneration) error.textContent = `${historyText().failed}: ${String(reason)}`;
    } finally {
      stop();
      if (generation === historyGeneration) historyLoading = false;
    }
  }
  function loadAnalyticsPollingMs() {
    try {
      const seconds = Number(localStorage.getItem("tracepulse-traffic-refresh-seconds"));
      return analyticsRefreshOptions.includes(seconds) ? seconds * 1e3 : 5e3;
    } catch {
      return 5e3;
    }
  }
  function saveAnalyticsPollingMs(value) {
    try {
      localStorage.setItem("tracepulse-traffic-refresh-seconds", String(value / 1e3));
    } catch {
    }
  }
  function t(key) {
    return typeof window.t === "function" ? window.t(key) : key;
  }
  function ifaceSelectionKey() {
    return `tracepulse-iface-selection:${window.DEVICE_IP || ""}`;
  }
  function saveSelectedInterfaces() {
    try {
      localStorage.setItem(ifaceSelectionKey(), JSON.stringify(Array.from(ifaceSelected)));
    } catch {
    }
  }
  function syncSelectedInterfaces(ifaces) {
    const known = new Set(ifaces.map((iface) => String(iface.if_index)));
    if (!ifaceSelectionReady) {
      let stored = null;
      try {
        stored = JSON.parse(localStorage.getItem(ifaceSelectionKey()) || "null");
      } catch {
        stored = null;
      }
      ifaceSelected = /* @__PURE__ */ new Set();
      if (Array.isArray(stored) && stored.length > 0) {
        stored.forEach((value) => {
          const key = String(value);
          if (known.has(key)) ifaceSelected.add(key);
        });
      }
      if (ifaceSelected.size === 0) {
        known.forEach((value) => ifaceSelected.add(value));
      }
      ifaceSelectionReady = true;
    } else {
      Array.from(ifaceSelected).forEach((value) => {
        if (!known.has(String(value))) ifaceSelected.delete(value);
      });
    }
    saveSelectedInterfaces();
  }
  function isInterfaceSelected(ifIndex) {
    return ifaceSelected.has(String(ifIndex));
  }
  function selectAllInterfaces(checked) {
    const ifaces = lastDeviceDetail?.interfaces || [];
    ifaceSelected = /* @__PURE__ */ new Set();
    if (checked) {
      ifaces.forEach((iface) => ifaceSelected.add(String(iface.if_index)));
    }
    saveSelectedInterfaces();
    if (lastDeviceDetail) renderDetail(lastDeviceDetail);
  }
  function toggleInterfaceSelection(ifIndex, checked) {
    const key = String(ifIndex);
    if (checked) {
      ifaceSelected.add(key);
    } else {
      ifaceSelected.delete(key);
    }
    saveSelectedInterfaces();
    if (lastDeviceDetail) renderDetail(lastDeviceDetail);
  }
  function renderInterfaceSelection(ifaces = []) {
    const box = document.getElementById("iface-filter");
    const note = document.getElementById("iface-filter-note");
    if (!box) return;
    box.innerHTML = ifaces.map((iface) => {
      const id = `iface-select-${iface.if_index}`;
      const label = iface.if_name || `if-${iface.if_index}`;
      const checked = isInterfaceSelected(iface.if_index) ? " checked" : "";
      return `<label class='iface-filter-item' for='${id}'><input id='${id}' type='checkbox' value='${iface.if_index}'${checked} onchange='toggleInterfaceSelection(this.value,this.checked)'><span>${esc(label)} (if-${iface.if_index})</span></label>`;
    }).join("");
    if (note) {
      note.textContent = ifaces.length ? `${Array.from(ifaceSelected).length} / ${ifaces.length} ${t("selected")}` : `0 / 0 ${t("selected")}`;
    }
  }
  function includeTrafficItem(item) {
    if (!item) return false;
    if (!Object.prototype.hasOwnProperty.call(item, "if_index")) return true;
    if (typeof window.isInterfaceSelected !== "function") return true;
    return window.isInterfaceSelected(item.if_index);
  }
  function trafficProtocolsEmptyHtml() {
    return `<p class='traffic-empty'>${t("traffic_protocols_empty")}</p>`;
  }
  function renderTrafficProtocols(data) {
    const box = document.getElementById("traffic-protocols");
    const section = document.getElementById("traffic-protocols-section");
    if (!box || !section) return;
    const allProtocols = Array.isArray(data?.protocols) ? data.protocols : [];
    const allTalkers = Array.isArray(data?.top_talkers) ? data.top_talkers : [];
    section.style.display = allProtocols.length || allTalkers.length ? "block" : "none";
    if (section.style.display === "none") return;
    const protocols = allProtocols.filter(includeTrafficItem);
    const talkers = allTalkers.filter(includeTrafficItem);
    if (protocols.length === 0 && talkers.length === 0) {
      box.innerHTML = trafficProtocolsEmptyHtml();
      return;
    }
    let cursor = 0;
    const stops = protocols.map((item, index) => {
      const start = cursor;
      cursor += Number(item.percentage || 0);
      return `${palette[index % palette.length]} ${start}% ${cursor}%`;
    });
    const legend = protocols.map((item, index) => {
      const color = palette[index % palette.length];
      const percent = Number(item.percentage || 0).toFixed(1);
      return `<div class='traffic-legend-item'><span class='traffic-legend-swatch' style='background:${color}'></span>${esc(item.protocol)} ${percent}% (${formatBps(item.bps)})</div>`;
    }).join("");
    const rows = talkers.map((item) => {
      const flags = (item.tcp_flags || []).join(", ");
      const ingress = item.ingress_if_index === null || item.ingress_if_index === void 0 ? "-" : item.ingress_if_index;
      const egress = item.egress_if_index === null || item.egress_if_index === void 0 ? "-" : item.egress_if_index;
      return `<tr><td>${esc(item.source_ip)}:${item.source_port || 0}</td><td>${esc(item.destination_ip)}:${item.destination_port || 0}</td><td>${esc(item.protocol)}</td><td>${esc(item.app_name || "-")}</td><td>${formatBps(item.bps)}</td><td>${formatBps(item.pps)}</td><td>${esc(flags || "-")}</td><td>${esc(item.ingress_if_name || `if-${ingress}`)}</td><td>${esc(item.egress_if_name || `if-${egress}`)}</td></tr>`;
    }).join("");
    const summary = data?.summary || {};
    const applications = (data?.applications || []).map((item) => {
      const percentage = Number(item.percentage || 0);
      return `<div class='traffic-share-item'><div class='traffic-share-label'><span>${esc(item.app_name)}</span><span>${percentage.toFixed(1)}% \xB7 ${formatBps(item.bps)}</span></div><div class='traffic-share-track'><span style='width:${Math.min(100, Math.max(0, percentage))}%'></span></div></div>`;
    }).join("");
    const endpoints = (items) => items.map((item) => `<div class='traffic-legend-item'><span>${esc(item.ip)}</span><span>${Number(item.percentage || 0).toFixed(1)}% \xB7 ${formatBps(item.bps)}</span></div>`).join("");
    const timeseries = (data?.timeseries || []).map((item) => `<tr><td>${esc(item.timestamp)}</td><td>${formatBps(item.udp_bps)}</td><td>${formatBps(item.tcp_bps)}</td><td>${formatBps(item.icmp_bps)}</td></tr>`).join("");
    box.innerHTML = `<section class='traffic-card traffic-summary-card'><div class='traffic-controls'><span class='traffic-card-label'>${t("traffic_protocols_window")}</span><button type='button' data-window='60s'>${t("traffic_protocols_live")}</button><button type='button' data-window='5m'>${t("traffic_protocols_last_5m")}</button><button type='button' data-window='1h'>${t("traffic_protocols_last_1h")}</button><button type='button' data-window='24h'>${t("traffic_protocols_last_24h")}</button><button type='button' id='traffic-pause'>${analyticsPaused ? t("traffic_protocols_resume") : t("traffic_protocols_pause")}</button><label class='traffic-card-label' for='traffic-refresh-interval'>${t("traffic_protocols_refresh_interval")}</label><select id='traffic-refresh-interval' class='traffic-refresh-select'>${analyticsRefreshOptions.map((seconds) => `<option value='${seconds}'${analyticsPollingMs === seconds * 1e3 ? " selected" : ""}>${seconds}s</option>`).join("")}</select><span class='traffic-card-label'>${t("traffic_protocols_auto_refresh").replace("{seconds}", String(analyticsPollingMs / 1e3))}</span></div><h3>${t("traffic_protocols_summary")} (${esc(analyticsWindow)})</h3><div class='traffic-kpi-row'><div><span>${t("traffic_protocols_total_bps")}</span><strong>${formatBps(summary.total_bps)}</strong></div><div><span>${t("traffic_protocols_packet_rate")}</span><strong>${formatBps(summary.total_pps)} pps</strong></div><div><span>${t("traffic_protocols_active_flows")}</span><strong>${fmtNum(summary.active_flows)}</strong></div><div><span>${t("traffic_protocols_top_protocol")}</span><strong>${esc(summary.top_protocol || "-")}</strong></div></div></section><section class='traffic-card traffic-timeseries-card'><h3>${t("traffic_protocols_timeseries")}</h3><div class='traffic-table-scroll traffic-timeseries-scroll'><table class='talker-table'><thead><tr><th>${t("traffic_protocols_time")}</th><th>UDP</th><th>TCP</th><th>ICMP</th></tr></thead><tbody>${timeseries}</tbody></table></div></section><section class='traffic-card traffic-protocol-card'><h3>${t("traffic_protocols_share")}</h3><div class='traffic-donut-wrap'><div class='traffic-donut' style='background:conic-gradient(${stops.join(",") || "#334155 0 100%"})'></div><div class='traffic-legend'>${legend}</div></div></section><section class='traffic-card traffic-applications-card'><h3>${t("traffic_protocols_applications")}</h3><div class='traffic-scroll-panel traffic-share-list'>${applications || trafficProtocolsEmptyHtml()}</div></section><section class='traffic-card traffic-endpoints-card'><h3>${t("traffic_protocols_top_sources")}</h3><div class='traffic-scroll-panel traffic-legend'>${endpoints(data?.top_sources || []) || trafficProtocolsEmptyHtml()}</div></section><section class='traffic-card traffic-endpoints-card'><h3>${t("traffic_protocols_top_destinations")}</h3><div class='traffic-scroll-panel traffic-legend'>${endpoints(data?.top_destinations || []) || trafficProtocolsEmptyHtml()}</div></section><section class='traffic-card traffic-talkers-card'><h3>${t("traffic_protocols_top_talkers")}</h3><div class='traffic-table-scroll'><table class='talker-table traffic-talkers-table'><thead><tr><th>${t("traffic_protocols_source")}</th><th>${t("traffic_protocols_destination")}</th><th>${t("traffic_protocols_protocol")}</th><th>${t("traffic_protocols_application")}</th><th>bps</th><th>pps</th><th>${t("traffic_protocols_tcp_flags")}</th><th>${t("traffic_protocols_in_if")}</th><th>${t("traffic_protocols_out_if")}</th></tr></thead><tbody>${rows}</tbody></table></div></section>`;
    box.querySelectorAll("[data-window]").forEach((button) => button.addEventListener("click", () => {
      analyticsWindow = button.dataset.window || "60s";
      refreshTrafficProtocols();
    }));
    box.querySelector("#traffic-pause")?.addEventListener("click", () => {
      analyticsPaused = !analyticsPaused;
      scheduleAnalyticsPolling();
      refreshTrafficProtocols();
    });
    box.querySelector("#traffic-refresh-interval")?.addEventListener("change", (event) => {
      const seconds = Number(event.target.value);
      if (!analyticsRefreshOptions.includes(seconds)) return;
      analyticsPollingMs = seconds * 1e3;
      saveAnalyticsPollingMs(analyticsPollingMs);
      scheduleAnalyticsPolling();
      renderTrafficProtocols(data);
    });
  }
  function refreshTrafficProtocols() {
    if (analyticsPaused || document.visibilityState === "hidden") return;
    fetch(`/api/flow/analytics?window=${encodeURIComponent(analyticsWindow)}&limit=10`, { cache: "no-store" }).then((response) => response.json()).then(renderTrafficProtocols).catch(() => renderTrafficProtocols(null));
  }
  function scheduleAnalyticsPolling() {
    if (analyticsTimer !== null) window.clearInterval(analyticsTimer);
    analyticsTimer = analyticsPaused ? null : window.setInterval(refreshTrafficProtocols, analyticsPollingMs);
  }
  function counterClass(delta, spikeThreshold) {
    if (delta >= spikeThreshold) return "crit";
    if (delta > 0) return "warn";
    return "none";
  }
  function formatCounter(total, delta) {
    const numericDelta = Number(delta || 0);
    const totalText = `<span class='counter-total'>${fmtNum(total)}</span>`;
    if (!numericDelta) return totalText;
    return `${totalText}<span class='counter-delta ${counterClass(numericDelta, window.TRACEPULSE_SPIKE_THRESHOLD || 10)}'> (+${fmtNum(numericDelta)})</span>`;
  }
  function formatLateCollisions(total, delta) {
    const numericDelta = Number(delta || 0);
    const totalText = `<span class='counter-total'>${fmtNum(total)}</span>`;
    if (!numericDelta) return totalText;
    return `${totalText}<span class='counter-delta duplex'> (+${fmtNum(numericDelta)})</span>`;
  }
  var activeErrorPopover;
  var dismissedErrorPopoverTrigger;
  var errorPopoverTimer;
  var errorPopoverSequence = 0;
  function closeErrorPopover() {
    window.clearTimeout(errorPopoverTimer);
    if (!activeErrorPopover) return;
    const { trigger, popup } = activeErrorPopover;
    if (typeof popup.hidePopover === "function" && popup.matches(":popover-open")) popup.hidePopover();
    popup.classList.remove("is-open");
    popup.hidden = true;
    trigger.setAttribute("aria-expanded", "false");
    if (trigger.isConnected) trigger.append(popup);
    else popup.remove();
    activeErrorPopover = void 0;
  }
  function positionErrorPopover() {
    if (!activeErrorPopover) return;
    const { trigger, popup } = activeErrorPopover;
    const bounds = trigger.getBoundingClientRect();
    const scrollRegion = trigger.closest(".table-scroll")?.getBoundingClientRect();
    if (!trigger.isConnected || bounds.bottom < 0 || bounds.top > window.innerHeight || scrollRegion && (bounds.bottom <= scrollRegion.top || bounds.top >= scrollRegion.bottom || bounds.right <= scrollRegion.left || bounds.left >= scrollRegion.right)) {
      closeErrorPopover();
      return;
    }
    const margin = 8, gap = 6;
    popup.style.maxHeight = `${Math.max(0, window.innerHeight - margin * 2)}px`;
    const naturalHeight = popup.getBoundingClientRect().height;
    const below = window.innerHeight - bounds.bottom - gap - margin;
    const above = bounds.top - gap - margin;
    const openBelow = naturalHeight <= below || below >= above;
    popup.style.maxHeight = `${Math.max(1, openBelow ? below : above)}px`;
    const size = popup.getBoundingClientRect();
    const top = openBelow ? bounds.bottom + gap : bounds.top - size.height - gap;
    popup.style.left = `${Math.max(margin, Math.min(bounds.left, window.innerWidth - size.width - margin))}px`;
    popup.style.top = `${Math.max(margin, Math.min(top, window.innerHeight - size.height - margin))}px`;
  }
  function bindErrorPopovers(container) {
    container.querySelectorAll(".err-cell").forEach((trigger) => {
      const popup = trigger.querySelector(".err-pop");
      popup.id = `interface-error-breakdown-${++errorPopoverSequence}`;
      trigger.setAttribute("aria-describedby", popup.id);
      const show = () => {
        window.clearTimeout(errorPopoverTimer);
        if (activeErrorPopover?.trigger === trigger) return;
        closeErrorPopover();
        document.body.append(popup);
        activeErrorPopover = { trigger, popup };
        popup.hidden = false;
        popup.classList.add("is-open");
        if (typeof popup.showPopover === "function") {
          popup.setAttribute("popover", "manual");
          popup.showPopover();
        }
        trigger.setAttribute("aria-expanded", "true");
        positionErrorPopover();
      };
      const leave = (event) => {
        const target = event.relatedTarget;
        if (target instanceof Node && (trigger.contains(target) || popup.contains(target))) return;
        errorPopoverTimer = window.setTimeout(() => {
          if (document.activeElement !== trigger && !trigger.matches(":hover") && !popup.matches(":hover")) closeErrorPopover();
        }, 120);
      };
      trigger.addEventListener("mouseenter", () => {
        if (dismissedErrorPopoverTrigger !== trigger) show();
      });
      trigger.addEventListener("mousemove", () => {
        if (dismissedErrorPopoverTrigger !== trigger) return;
        dismissedErrorPopoverTrigger = void 0;
        show();
      });
      trigger.addEventListener("mouseleave", leave);
      trigger.addEventListener("focus", show);
      trigger.addEventListener("blur", closeErrorPopover);
      trigger.addEventListener("click", show);
      trigger.addEventListener("keydown", (event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          show();
        }
      });
      popup.addEventListener("mouseenter", () => window.clearTimeout(errorPopoverTimer));
      popup.addEventListener("mouseleave", leave);
    });
  }
  window.addEventListener("scroll", (event) => {
    if (event.target instanceof Node && activeErrorPopover?.popup.contains(event.target)) return;
    positionErrorPopover();
  }, true);
  window.addEventListener("resize", closeErrorPopover);
  document.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;
    dismissedErrorPopoverTrigger = activeErrorPopover?.trigger;
    closeErrorPopover();
  });
  document.addEventListener("pointerdown", (event) => {
    if (event.target instanceof Node && activeErrorPopover && !activeErrorPopover.trigger.contains(event.target) && !activeErrorPopover.popup.contains(event.target)) closeErrorPopover();
  });
  function errorBreakdownPopoverHtml(errorBreakdown = {}, metrics = {}) {
    return `<div class='err-pop' role='tooltip' hidden><div class='err-pop-title'>${t("error_breakdown_hover_title")}</div><div class='err-pop-row'><span class='n'>${t("in_errors")}</span><span class='v'>+${fmtNum(metrics.in_errors_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("out_errors")}</span><span class='v'>+${fmtNum(metrics.out_errors_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("in_discards")}</span><span class='v'>+${fmtNum(metrics.in_discards_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("out_discards")}</span><span class='v'>+${fmtNum(metrics.out_discards_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("late_collisions")}</span><span class='v'>+${fmtNum(metrics.late_collisions_delta)}</span></div><div class='err-pop-divider'></div><div class='err-pop-row'><span class='n'>${t("error_breakdown_fcs_errors")}</span><span class='v'>+${fmtNum(errorBreakdown.fcs_errors_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("error_breakdown_alignment_errors")}</span><span class='v'>+${fmtNum(errorBreakdown.alignment_errors_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("error_breakdown_frame_too_longs")}</span><span class='v'>+${fmtNum(errorBreakdown.frame_too_longs_delta)}</span></div><div class='err-pop-row'><span class='n'>${t("error_breakdown_internal_mac_receive_errors")}</span><span class='v'>+${fmtNum(errorBreakdown.internal_mac_receive_errors_delta)}</span></div></div>`;
  }
  function formatCounterWithBreakdown(total, delta, errorBreakdown, metrics) {
    return `<span class='err-cell' tabindex='0' role='button' aria-expanded='false' aria-label='${esc(t("error_breakdown_hover_title"))}'>${formatCounter(total, delta)}${errorBreakdownPopoverHtml(errorBreakdown, metrics)}</span>`;
  }
  function diagnosticBadge(status) {
    const diagnostics = {
      healthy: { className: "healthy", translation: "diagnostic_healthy" },
      l1_error: { className: "l1_error", translation: "diagnostic_l1_error" },
      duplex_mismatch: { className: "duplex_mismatch", translation: "diagnostic_duplex_mismatch" },
      congestion: { className: "congestion", translation: "diagnostic_congestion" },
      down: { className: "down", translation: "diagnostic_down" }
    };
    const diagnostic = status === void 0 ? diagnostics.healthy : Object.prototype.hasOwnProperty.call(diagnostics, status) ? diagnostics[status] : diagnostics.down;
    return `<span class='diagnostic-badge ${diagnostic.className}'>${esc(t(diagnostic.translation))}</span>`;
  }
  function renderInterfaces(ifaces = []) {
    const tbody = document.getElementById("if-tbody");
    if (!tbody) return;
    closeErrorPopover();
    if (ifaces.length === 0) {
      tbody.innerHTML = `<tr><td colspan=13 class=no-data>${t("no_interface_data")}</td></tr>`;
      return;
    }
    const spikeThreshold = window.TRACEPULSE_SPIKE_THRESHOLD || 10;
    tbody.innerHTML = ifaces.map((iface) => {
      const metrics = iface.metrics || {};
      const linkClass = interfaceStatusClass(iface.link_status);
      const errorDelta = Math.max(metrics.in_errors_delta || 0, metrics.out_errors_delta || 0);
      const discardDelta = Math.max(metrics.in_discards_delta || 0, metrics.out_discards_delta || 0);
      const rowClass = errorDelta >= spikeThreshold || discardDelta >= spikeThreshold ? " row-crit" : errorDelta > 0 || discardDelta > 0 ? " row-warn" : "";
      const predictiveStatus = iface.predictive_status;
      const predictiveBadge = predictiveStatus && (predictiveStatus.dom_warning || predictiveStatus.trend_warning) ? "<span class='pred-badge'>[PRED]</span>" : "";
      return `<tr class='${rowClass.trim()}'><td>${iface.if_index}</td><td>${esc(iface.if_name)}${predictiveBadge}</td><td><span class=${linkClass}>${esc(iface.link_status)}</span></td><td>${diagnosticBadge(iface.health_status)}</td><td>${formatCounterWithBreakdown(metrics.in_errors, metrics.in_errors_delta, iface.error_breakdown, metrics)}</td><td>${formatCounterWithBreakdown(metrics.out_errors, metrics.out_errors_delta, iface.error_breakdown, metrics)}</td><td>${formatCounter(metrics.in_discards, metrics.in_discards_delta)}</td><td>${formatCounter(metrics.out_discards, metrics.out_discards_delta)}</td><td>${formatLateCollisions(metrics.late_collisions, metrics.late_collisions_delta)}</td><td>${Number(metrics.bandwidth_utilization || 0).toFixed(1)}%</td><td>${opticalPowerHtml(iface.tx_power_dbm)}</td><td>${opticalPowerHtml(iface.rx_power_dbm === void 0 ? predictiveStatus?.rx_optical_power_dbm : iface.rx_power_dbm)}</td><td>${fmtTime(iface.sampled_at)}</td></tr>`;
    }).join("");
    bindErrorPopovers(tbody);
  }
  function opticalPowerHtml(value) {
    if (typeof value !== "number" || !Number.isFinite(value)) return "N/A";
    return `<span class="${value <= -20 ? "status-warning" : ""}">${value.toFixed(1)} dBm</span>`;
  }
  function renderHardwareStatus(sensors = []) {
    const box = document.getElementById("hardware-status");
    if (!box) return;
    const visible = sensors.filter((sensor) => {
      const type = (sensor.sensor_type || "").toLowerCase();
      const hasValue = sensor.value !== null && sensor.value !== void 0 || sensor.status !== null && sensor.status !== void 0;
      return (type === "temperature" || type === "power" || type === "fan") && hasValue;
    });
    if (visible.length === 0) {
      box.innerHTML = "<span class=no-data>No Sensors Detected</span>";
      return;
    }
    box.innerHTML = visible.map((sensor) => {
      const cls = sensor.is_alarm || sensor.status_kind === "critical" ? "hardware-card crit" : sensor.status_kind === "warning" ? "hardware-card warn" : sensor.status_kind === "normal" ? "hardware-card" : sensor.status !== null && sensor.status !== void 0 && sensor.status !== 0 ? "hardware-card warn" : "hardware-card";
      const label = sensor.name || (sensor.source === "entity-physical" ? `component ${sensor.index}` : `sensor ${sensor.index}`);
      const displayValue = sensor.status_text ? sensor.status_text : sensor.value !== null && sensor.value !== void 0 ? `${sensor.value}${sensor.unit ? ` ${sensor.unit}` : ""}` : sensor.status !== null && sensor.status !== void 0 ? sensor.status : "N/A";
      const hasNumericValue = sensor.value !== null && sensor.value !== void 0;
      const stateClass = sensor.status_text || !hasNumericValue ? cls.endsWith("crit") ? "status-critical" : cls.endsWith("warn") ? "status-warning" : sensor.status_kind === "normal" ? "status-online" : "status-unknown" : "";
      return `<div class='${cls}'><div class='label'>${esc(label)}</div><div class='value ${stateClass}'>${esc(displayValue)}</div></div>`;
    }).join("");
  }
  function renderTemplateMetrics(metrics = [], error) {
    const box = document.getElementById("template-metrics");
    if (!box) return;
    if (error) {
      box.innerHTML = `<span class='no-data' role='alert'>${esc(error)}</span>`;
      return;
    }
    if (metrics.length === 0) {
      box.innerHTML = `<span class='no-data'>${t("template_metrics_empty")}</span>`;
      return;
    }
    box.innerHTML = metrics.map((metric) => {
      const value = metric.value === null ? "N/A" : `${metric.value_text ?? metric.value} ${metric.unit}`.trim();
      const note = metric.error ? `<div role='alert'>${esc(metric.error)}</div>` : "";
      return `<div class='hardware-card' data-metric-key='${esc(metric.key)}'><div class='label'>${esc(metric.label)}</div><div class='value'>${esc(value)}</div>${note}</div>`;
    }).join("");
  }
  function renderErrorBreakdownCard(ifaces = []) {
    const box = document.getElementById("error-breakdown");
    if (!box) return;
    if (ifaces.length === 0) {
      box.innerHTML = `<p class='no-data'>${t("error_breakdown_no_data")}</p>`;
      return;
    }
    if (errorBreakdownSelectedIf === null || !ifaces.some((iface) => iface.if_index === errorBreakdownSelectedIf)) {
      errorBreakdownSelectedIf = ifaces[0].if_index;
    }
    const options = ifaces.map((iface) => {
      const selected2 = iface.if_index === errorBreakdownSelectedIf ? " selected" : "";
      return `<option value='${iface.if_index}'${selected2}>${esc(iface.if_name || `if-${iface.if_index}`)} (if-${iface.if_index})</option>`;
    }).join("");
    const selected = ifaces.find((iface) => iface.if_index === errorBreakdownSelectedIf) || ifaces[0];
    const eb = selected.error_breakdown || {};
    const fcs = eb.fcs_errors_delta || 0;
    const align = eb.alignment_errors_delta || 0;
    const toolong = eb.frame_too_longs_delta || 0;
    const macrx = eb.internal_mac_receive_errors_delta || 0;
    const total = fcs + align + toolong + macrx;
    const pct = (value) => total > 0 ? value / total * 100 : 0;
    const bar = `<div class='breakdown-bar'><div class='breakdown-bar-seg fcs' style='width:${pct(fcs)}%'></div><div class='breakdown-bar-seg alignment' style='width:${pct(align)}%'></div><div class='breakdown-bar-seg frametoolong' style='width:${pct(toolong)}%'></div><div class='breakdown-bar-seg macreceive' style='width:${pct(macrx)}%'></div></div>`;
    const legend = `<div class='breakdown-legend'><span><span class='swatch' style='background:#f87171'></span>${t("error_breakdown_fcs_errors")}</span><span><span class='swatch' style='background:#fbbf24'></span>${t("error_breakdown_alignment_errors")}</span><span><span class='swatch' style='background:#a78bfa'></span>${t("error_breakdown_frame_too_longs")}</span><span><span class='swatch' style='background:#38bdf8'></span>${t("error_breakdown_internal_mac_receive_errors")}</span></div>`;
    const table = `<table class='breakdown-table'><thead><tr><th>${t("error_breakdown_fcs_errors")}</th><th>${t("error_breakdown_alignment_errors")}</th><th>${t("error_breakdown_frame_too_longs")}</th><th>${t("error_breakdown_internal_mac_receive_errors")}</th></tr></thead><tbody><tr><td>${fmtNum(eb.fcs_errors)} <span class='counter-delta warn'>(+${fmtNum(fcs)})</span></td><td>${fmtNum(eb.alignment_errors)} <span class='counter-delta warn'>(+${fmtNum(align)})</span></td><td>${fmtNum(eb.frame_too_longs)} <span class='counter-delta warn'>(+${fmtNum(toolong)})</span></td><td>${fmtNum(eb.internal_mac_receive_errors)} <span class='counter-delta warn'>(+${fmtNum(macrx)})</span></td></tr></tbody></table>`;
    box.innerHTML = `<div class='error-breakdown-select'><label for='error-breakdown-if'>${t("error_breakdown_select_hint")}</label><select id='error-breakdown-if' onchange='onErrorBreakdownIfaceChange(this.value)'>${options}</select></div>` + bar + legend + table;
  }
  function onErrorBreakdownIfaceChange(value) {
    errorBreakdownSelectedIf = parseInt(value, 10);
    if (window.LAST_DEVICE_DETAIL) renderErrorBreakdownCard(window.LAST_DEVICE_DETAIL.interfaces || []);
  }
  function renderSpikes(spikes = []) {
    const tbody = document.getElementById("spike-tbody");
    if (!tbody) return;
    if (spikes.length === 0) {
      tbody.innerHTML = `<tr><td colspan=8 class=no-data>${t("no_interface_spikes")}</td></tr>`;
      return;
    }
    tbody.innerHTML = spikes.map((spike) => {
      const linkClass = interfaceStatusClass(spike.link_status);
      return `<tr><td>${fmtTime(spike.latest_sampled_at)}</td><td>${esc(spike.if_name)} (if-${spike.if_index || 0})</td><td><span class=${linkClass}>${esc(spike.link_status)}</span></td><td>${fmtNum(spike.in_errors_delta)}</td><td>${fmtNum(spike.out_errors_delta)}</td><td>${fmtNum(spike.in_discards_delta)}</td><td>${fmtNum(spike.out_discards_delta)}</td><td>${fmtNum(spike.total_delta)}</td></tr>`;
    }).join("");
  }
  function renderAlerts(alerts = []) {
    const tbody = document.getElementById("alert-tbody");
    if (!tbody) return;
    if (alerts.length === 0) {
      tbody.innerHTML = `<tr><td colspan=5 class=no-data>${t("no_alerts")}</td></tr>`;
      return;
    }
    const severityClasses = { warning: "sev-warning", critical: "sev-critical", info: "sev-info" };
    tbody.innerHTML = alerts.map((alert) => {
      const severity = alert.severity || "";
      const severityClass = severityClasses[severity] || "";
      const iface = alert.interface || "-";
      return `<tr><td>${fmtTime(alert.at)}</td><td>${esc(iface)}</td><td>${esc(alert.type)}</td><td><span class=${severityClass}>${esc(severity)}</span></td><td>${esc(alert.details)}</td></tr>`;
    }).join("");
  }
  function formatAxisValue(value, yLabel) {
    if (yLabel === "bytes") {
      const units = ["B", "KB", "MB", "GB", "TB"];
      let displayValue = value;
      let index = 0;
      while (displayValue >= 1024 && index + 1 < units.length) {
        displayValue /= 1024;
        index++;
      }
      return `${index === 0 ? Math.round(displayValue) : displayValue.toFixed(1)} ${units[index]}`;
    }
    if (yLabel === "%") {
      if (value < 1) return value.toFixed(2);
      if (value < 10) return value.toFixed(1);
    }
    return Math.round(value).toString();
  }
  function sparkline(svgId, series, yLabel) {
    const svg = document.getElementById(svgId);
    if (!svg) return;
    const rect = svg.getBoundingClientRect();
    const width = Math.max(200, Math.round(rect.width) || 800);
    const height = Math.max(80, Math.round(rect.height) || 160);
    svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
    const fontSize = width < 420 ? 8 : 9;
    const paddingLeft = width < 420 ? 30 : 40;
    const paddingRight = 8;
    const paddingTop = 8;
    const paddingBottom = 20;
    const chartWidth = width - paddingLeft - paddingRight;
    const chartHeight = height - paddingTop - paddingBottom;
    const allPoints = series.flatMap((item) => (item.points || []).filter((point) => point && point.v !== null && point.v !== void 0 && !Number.isNaN(point.v)));
    if (allPoints.length === 0) {
      svg.classList.add("chart-empty");
      svg.innerHTML = `<text x='${width / 2}' y='${height / 2}' text-anchor='middle' fill='#64748b' font-size='13'>N/A</text>`;
      return;
    }
    svg.classList.remove("chart-empty");
    const times = allPoints.map((point) => parseTracePulseTime(point.t).getTime());
    const values = allPoints.map((point) => point.v);
    const historyBounds = monitoringHistory && ["bw-chart", "err-chart", "cpu-chart", "memory-chart"].includes(svgId) ? monitoringHistory : null;
    const timeMin = historyBounds ? new Date(historyBounds.start).getTime() : Math.min(...times);
    let timeMax = historyBounds ? new Date(historyBounds.end).getTime() : Math.max(...times);
    const valueMin = 0;
    const valueMax = yLabel === "%" ? 100 : Math.max(...values) * 1.15 || 1;
    if (timeMin === timeMax) timeMax = timeMin + 1;
    const tx = (time) => paddingLeft + (parseTracePulseTime(time).getTime() - timeMin) / (timeMax - timeMin) * chartWidth;
    const ty = (value) => paddingTop + chartHeight - (value - valueMin) / (valueMax - valueMin) * chartHeight;
    let output = "";
    for (let index = 0; index <= 4; index++) {
      const gridY = paddingTop + index * chartHeight / 4;
      const gridValue = (valueMax - valueMin) * (1 - index / 4) + valueMin;
      output += `<line x1='${paddingLeft}' y1='${gridY}' x2='${width - paddingRight}' y2='${gridY}' stroke='#1e293b' stroke-width='1'/>`;
      output += `<text x='${paddingLeft - 4}' y='${gridY + 4}' text-anchor='end' fill='#64748b' font-size='${fontSize}'>${formatAxisValue(gridValue, yLabel)}</text>`;
    }
    for (let index = 0; index <= 3; index++) {
      const labelTime = timeMin + (timeMax - timeMin) * index / 3;
      const x = paddingLeft + (labelTime - timeMin) / (timeMax - timeMin) * chartWidth;
      let anchor = "middle";
      let labelX = x;
      if (index === 0) {
        anchor = "start";
        labelX = paddingLeft + 2;
      } else if (index === 3) {
        anchor = "end";
        labelX = width - paddingRight - 2;
      }
      output += `<text x='${labelX}' y='${height - 4}' text-anchor='${anchor}' fill='#475569' font-size='${fontSize}'>${formatTracePulseClock(new Date(labelTime))}</text>`;
    }
    series.forEach((item) => {
      if (item.points.length === 0) return;
      const path = item.points.map((point, index) => `${index === 0 ? "M" : "L"}${tx(point.t).toFixed(1)} ${ty(point.v).toFixed(1)}`).join(" ");
      output += `<path d='${path}' fill='none' stroke='${item.color}' stroke-width='1.5' stroke-linejoin='round'/>`;
    });
    svg.innerHTML = output;
  }
  function interfaceLabel(ifIndex) {
    return window.IFACE_LABELS?.[String(ifIndex)] || `if-${ifIndex}`;
  }
  function renderBwChart(ifSeries = []) {
    const legend = document.getElementById("bw-legend");
    if (legend) legend.innerHTML = "";
    const ordered = [...ifSeries].sort((a, b) => a.if_index - b.if_index);
    const filtered = ordered.filter((series) => window.isInterfaceSelected?.(series.if_index));
    const chartSeries = filtered.map((series, index) => {
      const color = chartColors[index % chartColors.length];
      const label = interfaceLabel(series.if_index);
      if (legend) legend.innerHTML += `<span style='color:${color};margin-right:.5rem'>&#9632; ${esc(label)}</span>`;
      return { label, color, points: series.points.map((point) => ({ t: point.t, v: point.bw || 0 })) };
    });
    sparkline("bw-chart", chartSeries, "%");
  }
  function renderErrChart(ifSeries = []) {
    const legend = document.getElementById("err-legend");
    if (legend) legend.innerHTML = "";
    const ordered = [...ifSeries].sort((a, b) => a.if_index - b.if_index);
    const chartSeries = [];
    ordered.filter((series) => window.isInterfaceSelected?.(series.if_index)).forEach((series, index) => {
      const color = chartColors[index % chartColors.length];
      const label = interfaceLabel(series.if_index);
      if (legend && index < 4) legend.innerHTML += `<span style='color:${color};margin-right:.5rem'>&#9632; ${esc(label)}</span>`;
      chartSeries.push({
        label: `${label} in_err`,
        color,
        points: series.points.map((point) => ({ t: point.t, v: (point.in_err || 0) + (point.in_dis || 0) }))
      });
    });
    sparkline("err-chart", chartSeries, "count");
  }
  function renderCpuChart(metrics = []) {
    sparkline("cpu-chart", [{
      label: "CPU %",
      color: "#38bdf8",
      points: metrics.filter((metric) => metric.cpu !== null && metric.cpu !== void 0 && metric.cpu >= 0).map((metric) => ({ t: metric.t, v: metric.cpu || 0 }))
    }], "%");
  }
  function renderMemoryChart(metrics = []) {
    const memSeries = { label: "Memory Usage (%)", color: "#a78bfa", points: [] };
    metrics.forEach((metric) => {
      if (metric && metric.memory !== null && metric.memory !== void 0 && !Number.isNaN(metric.memory)) {
        memSeries.points.push({ t: metric.t, v: metric.memory });
      }
    });
    if (memSeries.points.length === 0 && metrics.length > 0) {
      const maxBytes = metrics.reduce((max, metric) => metric.memory_bytes && metric.memory_bytes > max ? metric.memory_bytes : max, 0);
      if (maxBytes > 0) {
        metrics.forEach((metric) => {
          if (metric.memory_bytes !== null && metric.memory_bytes !== void 0 && metric.memory_bytes > 0) {
            memSeries.points.push({ t: metric.t, v: Math.round(metric.memory_bytes / maxBytes * 100) });
          }
        });
      }
    }
    sparkline("memory-chart", [memSeries], "%");
  }
  function renderSysChart(metrics = []) {
    renderCpuChart(metrics);
    renderMemoryChart(metrics);
  }
  function renderHeader(device) {
    const title = document.getElementById("dev-title");
    if (title) {
      title.textContent = `${deviceLabel(device)} (${device.ip || ""})`;
      let metadata = document.getElementById("device-identity");
      if (!metadata) {
        metadata = document.createElement("div");
        metadata.id = "device-identity";
        title.parentElement.after(metadata);
      }
      metadata.innerHTML = `<div class="device-original">sysName: ${esc(device.name || "-")}</div>
            <div class="device-tags">${(device.tags || []).map((tag) => `<span class="device-tag">${esc(tag)}</span>`).join("")}</div>
            <button type="button" id="edit-device-metadata">${esc(metadataText("edit"))}</button>`;
      metadata.querySelector("button").onclick = () => {
        if (lastDeviceDetail) editDeviceMetadata(lastDeviceDetail, loadDeviceDetail);
      };
    }
    const status = document.getElementById("dev-status");
    if (status) {
      const classMap = { online: "status-online", offline: "status-offline", warning: "status-warning", critical: "status-critical" };
      status.className = classMap[device.status || ""] || "status-unknown";
      status.textContent = device.status || "unknown";
    }
    document.title = `TracePulse - ${deviceLabel(device) || "Device"}`;
  }
  function updateLastRefreshed() {
    const element = document.getElementById("device-refresh");
    if (element) {
      element.textContent = `${t("device_auto_refresh").replace("{seconds}", String(deviceRefreshSeconds))} \u2022 ${t("updated")}: ${formatTracePulseClock(/* @__PURE__ */ new Date())}`;
    }
  }
  function scheduleDeviceRefresh() {
    if (deviceRefreshTimer !== null) window.clearInterval(deviceRefreshTimer);
    deviceRefreshTimer = window.setInterval(loadDeviceDetail, deviceRefreshSeconds * 1e3);
  }
  function renderDetail(device) {
    lastDeviceDetail = device;
    window.LAST_DEVICE_DETAIL = device;
    renderHeader(device);
    const ifaces = device.interfaces || [];
    window.IFACE_LABELS = {};
    ifaces.forEach((iface) => {
      window.IFACE_LABELS[String(iface.if_index)] = iface.if_name || `if-${iface.if_index}`;
    });
    syncSelectedInterfaces(ifaces);
    renderInterfaceSelection(ifaces);
    renderInterfaces(ifaces);
    renderErrorBreakdownCard(ifaces);
    refreshTrafficProtocols();
    renderMonitoringCharts();
    renderHardwareStatus(device.hardware_sensors || []);
    renderTemplateMetrics(device.template_metrics || [], device.template_metrics_error);
    renderSpikes(device.spikes || []);
    renderAlerts(device.alerts || []);
    updateLastRefreshed();
  }
  var deviceDetailLoading = false;
  function loadDeviceDetail() {
    if (deviceDetailLoading) return;
    const refreshing = lastDeviceDetail !== null;
    deviceDetailLoading = true;
    const header = document.getElementById("device-refresh")?.parentElement;
    const stopProgress = header ? showOperationProgress(header, "Device details", (seconds) => {
      const japanese2 = document.documentElement.lang === "ja";
      return japanese2 ? `${lastDeviceDetail ? "\u30C7\u30D0\u30A4\u30B9\u60C5\u5831\u3092\u66F4\u65B0\u3057\u3066\u3044\u307E\u3059" : "\u30C7\u30D0\u30A4\u30B9\u60C5\u5831\u3092\u8AAD\u307F\u8FBC\u3093\u3067\u3044\u307E\u3059"}...\uFF08\u7D4C\u904E ${seconds}\u79D2\uFF09` : `${lastDeviceDetail ? "Updating device details" : "Loading device details"}... (${seconds}s elapsed)`;
    }) : () => {
    };
    Promise.all([
      fetch("/api/settings", { cache: "no-store" }).then((response) => response.json()).catch(() => null),
      fetch(`/api/device/${encodeURIComponent(window.DEVICE_IP || "")}`, { cache: "no-store" }).then((response) => {
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        return response.json();
      })
    ]).then(([settings, device]) => {
      const configuredInterval = Number(settings?.polling?.interval_seconds);
      if (Number.isFinite(configuredInterval) && configuredInterval >= 5 && configuredInterval <= 600) {
        deviceRefreshSeconds = configuredInterval;
        scheduleDeviceRefresh();
      }
      if (settings?.display?.timezone) {
        try {
          localStorage.setItem("tracepulse-timezone", settings.display.timezone);
        } catch {
        }
      }
      if (settings?.alert && typeof settings.alert.spike_threshold === "number") {
        window.TRACEPULSE_SPIKE_THRESHOLD = settings.alert.spike_threshold;
      }
      if (device.error) {
        const title = document.getElementById("dev-title");
        if (title) title.textContent = device.error;
        return;
      }
      renderDetail(device);
      if (refreshing || !monitoringHistory) void loadMonitoringHistory();
    }).catch((error) => {
      const title = document.getElementById("dev-title");
      if (title) title.textContent = `Error: ${String(error)}`;
    }).finally(() => {
      stopProgress();
      deviceDetailLoading = false;
    });
  }
  window.esc = esc;
  window.fmtNum = fmtNum;
  window.fmtTime = fmtTime;
  window.formatBps = formatBps;
  window.formatTracePulseClock = formatTracePulseClock;
  window.formatTracePulseTime = formatTracePulseTime;
  window.isInterfaceSelected = isInterfaceSelected;
  window.loadDeviceDetail = loadDeviceDetail;
  window.onErrorBreakdownIfaceChange = onErrorBreakdownIfaceChange;
  window.parseTracePulseTime = parseTracePulseTime;
  window.renderAlerts = renderAlerts;
  window.renderBwChart = renderBwChart;
  window.renderCpuChart = renderCpuChart;
  window.renderErrChart = renderErrChart;
  window.renderErrorBreakdownCard = renderErrorBreakdownCard;
  window.renderHardwareStatus = renderHardwareStatus;
  window.renderInterfaces = renderInterfaces;
  window.renderDetail = renderDetail;
  window.renderMemoryChart = renderMemoryChart;
  window.renderSpikes = renderSpikes;
  window.renderSysChart = renderSysChart;
  window.renderTrafficProtocols = renderTrafficProtocols;
  window.refreshTrafficProtocols = refreshTrafficProtocols;
  window.selectAllInterfaces = selectAllInterfaces;
  window.sparkline = sparkline;
  window.toggleInterfaceSelection = toggleInterfaceSelection;
  window.tracePulseTimeZone = tracePulseTimeZone;
  window.applyPageLanguage = () => {
    initializeHistoryControls();
    if (lastDeviceDetail) renderHeader(lastDeviceDetail);
    updateLastRefreshed();
    refreshTrafficProtocols();
  };
  initializeHistoryControls();
  void loadMonitoringHistory();
  loadDeviceDetail();
  scheduleDeviceRefresh();
  scheduleAnalyticsPolling();
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") {
      refreshTrafficProtocols();
      scheduleAnalyticsPolling();
    } else if (analyticsTimer !== null) {
      window.clearInterval(analyticsTimer);
      analyticsTimer = null;
    }
  });
  window.addEventListener("resize", () => {
    if (resizeTimer !== null) window.clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(() => {
      if (lastDeviceDetail) renderDetail(lastDeviceDetail);
    }, 200);
  });
  return __toCommonJS(device_detail_exports);
})();
