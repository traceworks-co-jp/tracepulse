"use strict";
(() => {
  // frontend/src/pages/maintenance.ts
  var words = {
    en: {
      title: "Alert maintenance",
      target: "Scope",
      global: "All devices",
      reason: "Reason",
      hour: "1 hour",
      day: "24 hours",
      indefinite: "Until manually unmuted",
      mute: "Mute",
      unmute: "Unmute",
      active: "Muted: collection and recording continue.",
      inactive: "Not muted",
      help: "After unmuting, continuing risks are summarized following a fresh observation."
    },
    ja: {
      title: "\u30A2\u30E9\u30FC\u30C8\u9759\u89B3",
      target: "\u5BFE\u8C61",
      global: "\u5168\u6A5F\u5668",
      reason: "\u7406\u7531",
      hour: "1\u6642\u9593",
      day: "24\u6642\u9593",
      indefinite: "\u624B\u52D5\u89E3\u9664\u307E\u3067",
      mute: "\u9759\u89B3\u3059\u308B",
      unmute: "\u89E3\u9664\u3059\u308B",
      active: "\u9759\u89B3\u4E2D\uFF1A\u53CE\u96C6\u30FB\u8A18\u9332\u306F\u7D99\u7D9A\u3057\u307E\u3059\u3002",
      inactive: "\u9759\u89B3\u306A\u3057",
      help: "\u89E3\u9664\u5F8C\u306E\u65B0\u3057\u3044\u89B3\u6E2C\u3067\u7D99\u7D9A\u4E2D\u306E\u30EA\u30B9\u30AF\u3092\u307E\u3068\u3081\u3066\u901A\u77E5\u3057\u307E\u3059\u3002"
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
      root.replaceChildren(element("h2", text("title")), element("p", text("help")));
      const label = element("label", text("target")), target = element("select");
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
      label.append(target);
      root.append(label);
      const own = selected ? status?.devices[selected] : status?.global;
      const active = !!own?.enabled && (own.muted_until === null || own.muted_until > (status?.server_time || 0));
      root.append(element("p", text(active ? "active" : "inactive")));
      const reasonLabel = element("label", text("reason")), reason = element("input");
      reason.maxLength = 500;
      reason.value = own?.reason || "";
      reasonLabel.append(reason);
      root.append(reasonLabel);
      const duration = element("select");
      for (const [value, key] of [["3600", "hour"], ["86400", "day"], ["", "indefinite"]]) {
        const option = element("option", text(key));
        option.value = value;
        duration.append(option);
      }
      root.append(duration);
      for (const enabled of [true, false]) {
        const button = element("button", text(enabled ? "mute" : "unmute"));
        button.type = "button";
        button.className = "btn";
        button.disabled = busy || !status || !enabled && !active;
        button.onclick = async () => {
          busy = true;
          error.textContent = "";
          render();
          try {
            status = await request("POST", {
              device_id: selected ? Number(selected) : null,
              enabled,
              duration: enabled && duration.value ? Number(duration.value) : null,
              reason: reason.value
            });
          } catch (failure) {
            error.textContent = String(failure);
          } finally {
            busy = false;
            render();
          }
        };
        root.append(button);
      }
      root.append(error);
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
