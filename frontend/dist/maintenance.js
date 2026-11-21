"use strict";
(() => {
  // frontend/src/pages/maintenance.ts
  var words = {
    en: {
      title: "Alert maintenance",
      target: "Scope",
      global: "All devices",
      reason: "Reason",
      duration: "Duration",
      reasonPlaceholder: "e.g., Planned switch firmware upgrade",
      muted: "Muted",
      until: "Until",
      hour: "1 hour",
      day: "24 hours",
      indefinite: "Until manually unmuted",
      mute: "Mute",
      unmute: "Unmute",
      active: "Muted: collection and recording continue.",
      inactive: "Not muted",
      help: "Mute and Unmute apply immediately; you do not need to click Save Settings. After unmuting, continuing risks are summarized following a fresh observation."
    },
    ja: {
      title: "\u30A2\u30E9\u30FC\u30C8\u9759\u89B3",
      target: "\u5BFE\u8C61",
      global: "\u5168\u6A5F\u5668",
      reason: "\u7406\u7531",
      duration: "\u671F\u9593",
      reasonPlaceholder: "\u4F8B\uFF1A\u30B9\u30A4\u30C3\u30C1\u306E\u30D5\u30A1\u30FC\u30E0\u30A6\u30A7\u30A2\u66F4\u65B0",
      muted: "\u9759\u89B3\u4E2D",
      until: "\u89E3\u9664\u4E88\u5B9A",
      hour: "1\u6642\u9593",
      day: "24\u6642\u9593",
      indefinite: "\u624B\u52D5\u89E3\u9664\u307E\u3067",
      mute: "\u9759\u89B3\u3059\u308B",
      unmute: "\u89E3\u9664\u3059\u308B",
      active: "\u9759\u89B3\u4E2D\uFF1A\u53CE\u96C6\u30FB\u8A18\u9332\u306F\u7D99\u7D9A\u3057\u307E\u3059\u3002",
      inactive: "\u9759\u89B3\u306A\u3057",
      help: "\u9759\u89B3\u3068\u89E3\u9664\u306F\u5373\u6642\u306B\u9069\u7528\u3055\u308C\u308B\u305F\u3081\u3001\u300C\u8A2D\u5B9A\u3092\u4FDD\u5B58\u300D\u306F\u4E0D\u8981\u3067\u3059\u3002\u89E3\u9664\u5F8C\u306E\u65B0\u3057\u3044\u89B3\u6E2C\u3067\u7D99\u7D9A\u4E2D\u306E\u30EA\u30B9\u30AF\u3092\u307E\u3068\u3081\u3066\u901A\u77E5\u3057\u307E\u3059\u3002"
    }
  };
  var root = document.getElementById("core-maintenance");
  if (root) {
    let status;
    let devices = [];
    let selected = "";
    let busy = false;
    const text = (key) => words[document.documentElement.lang === "ja" ? "ja" : "en"][key];
    const element = (tag, value = "") => {
      const node = document.createElement(tag);
      node.textContent = value;
      return node;
    };
    const error = element("p");
    error.setAttribute("role", "alert");
    const request = async (method, body) => {
      const response = await fetch("/api/maintenance", {
        method,
        headers: { "Content-Type": "application/json", ...window.tracepulseRequestHeaders?.() },
        body: body === void 0 ? void 0 : JSON.stringify(body)
      });
      const value = await response.json();
      if (!response.ok || value.error) throw new Error(value.error || `HTTP ${response.status}`);
      return value;
    };
    const render = () => {
      root.replaceChildren();
      const own = selected ? status?.devices[selected] : status?.global;
      const active = !!own?.enabled && (own.muted_until === null || own.muted_until > (status?.server_time || 0));
      const header = element("div");
      header.className = "maintenance-header";
      header.append(element("h2", text("title")));
      const state = element("div");
      state.className = "maintenance-state";
      const badge = element("span", text(active ? "muted" : "inactive"));
      badge.className = `maintenance-status-badge ${active ? "is-muted" : "is-clear"}`;
      badge.setAttribute("role", "status");
      state.append(badge);
      if (active && own) {
        const details = element("div");
        details.className = "maintenance-status-details";
        details.append(element("span", text("active")));
        const until = own.muted_until === null ? text("indefinite") : new Intl.DateTimeFormat(
          document.documentElement.lang === "ja" ? "ja-JP" : "en-US",
          { dateStyle: "medium", timeStyle: "short" }
        ).format(new Date(own.muted_until * 1e3));
        details.append(element("span", `${text("until")}: ${until}`));
        if (own.reason) details.append(element("span", `${text("reason")}: ${own.reason}`));
        state.append(details);
      }
      header.append(state);
      root.append(header, element("p", text("help")));
      const form = element("div");
      form.className = "maintenance-form-grid";
      const scopeField = element("div");
      scopeField.className = "maintenance-field";
      const scopeLabel = element("label", text("target"));
      const target = element("select");
      target.id = "maintenance-scope";
      scopeLabel.htmlFor = target.id;
      const all = element("option", text("global"));
      all.value = "";
      target.append(all);
      for (const device of devices) {
        const option = element("option", `${device.name} (${device.ip})`);
        option.value = String(device.id);
        target.append(option);
      }
      target.value = selected;
      target.onchange = () => {
        selected = target.value;
        render();
      };
      scopeField.append(scopeLabel, target);
      const durationField = element("div");
      durationField.className = "maintenance-field";
      const durationLabel = element("label", text("duration"));
      const duration = element("select");
      duration.id = "maintenance-duration";
      durationLabel.htmlFor = duration.id;
      for (const [value, key] of [["3600", "hour"], ["86400", "day"], ["", "indefinite"]]) {
        const option = element("option", text(key));
        option.value = value;
        duration.append(option);
      }
      durationField.append(durationLabel, duration);
      const reasonField = element("div");
      reasonField.className = "maintenance-field maintenance-field-full";
      const reasonLabel = element("label", text("reason"));
      const reason = element("input");
      reason.id = "maintenance-reason";
      reasonLabel.htmlFor = reason.id;
      reason.maxLength = 500;
      reason.value = own?.reason || "";
      reason.placeholder = text("reasonPlaceholder");
      reasonField.append(reasonLabel, reason);
      form.append(scopeField, durationField, reasonField);
      root.append(form);
      const actions = element("div");
      actions.className = "maintenance-actions";
      for (const enabled of [true, false]) {
        const button = element("button", text(enabled ? "mute" : "unmute"));
        button.type = "button";
        button.className = enabled ? "btn btn-primary" : "btn btn-secondary";
        button.disabled = busy || !status || !enabled && !active;
        button.onclick = async () => {
          const submittedDuration = duration.value;
          const submittedReason = reason.value;
          busy = true;
          error.textContent = "";
          render();
          try {
            status = await request("POST", {
              device_id: selected ? Number(selected) : null,
              enabled,
              duration: enabled && submittedDuration ? Number(submittedDuration) : null,
              reason: submittedReason
            });
          } catch (failure) {
            error.textContent = String(failure);
          } finally {
            busy = false;
            render();
          }
        };
        actions.append(button);
      }
      root.append(actions, error);
    };
    const load = async () => {
      try {
        const [snapshot, response] = await Promise.all([request("GET"), fetch("/api/devices")]);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        devices = await response.json();
        status = snapshot;
      } catch (failure) {
        error.textContent = String(failure);
      }
      render();
    };
    window.addEventListener("tracepulse-language-change", render);
    void load();
  }
})();
