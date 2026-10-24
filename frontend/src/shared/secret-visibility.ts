const eye = `<svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true" focusable="false"><path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12Z"/><circle cx="12" cy="12" r="3"/><path class="secret-eye-slash" d="m3 3 18 18"/></svg>`;

export function addSecretVisibility(input: HTMLInputElement): void {
    if (input.parentElement?.classList.contains("secret-input")) return;
    if (!document.getElementById("secret-visibility-style")) {
        const style = document.createElement("style");
        style.id = "secret-visibility-style";
        style.textContent = `
            .secret-input{position:relative;display:block;min-width:0}
            .secret-input input{padding-right:44px!important}
            .secret-input .secret-toggle{position:absolute;right:2px;top:2px;bottom:2px;width:40px;padding:0;display:flex;align-items:center;justify-content:center;border:0;border-radius:4px;background:transparent;color:inherit;cursor:pointer}
            .secret-toggle:focus-visible{outline:2px solid #3b82f6;outline-offset:-2px}
            .secret-toggle:hover{background:rgb(127 127 127 / .15)}
            .secret-toggle .secret-eye-slash{display:none}
            .secret-toggle[aria-pressed=true] .secret-eye-slash{display:block}
        `;
        document.head.append(style);
    }
    const wrapper = document.createElement("span");
    wrapper.className = "secret-input";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "secret-toggle";
    button.setAttribute("aria-controls", input.id);
    button.innerHTML = eye;
    const update = (): void => {
        const visible = input.type === "text";
        const key = visible ? "hide_community" : "show_community";
        const fallback = visible ? "Hide community" : "Show community";
        const label = window.t?.(key) || fallback;
        button.setAttribute("aria-label", label);
        button.setAttribute("aria-pressed", String(visible));
        button.title = label;
    };
    button.addEventListener("click", () => {
        input.type = input.type === "password" ? "text" : "password";
        update();
    });
    input.before(wrapper);
    wrapper.append(input, button);
    update();
}

window.addEventListener("tracepulse-language-change", () => {
    document.querySelectorAll<HTMLButtonElement>(".secret-toggle").forEach((button) => {
        const key = button.getAttribute("aria-pressed") === "true" ? "hide_community" : "show_community";
        const label = window.t?.(key);
        if (label) { button.setAttribute("aria-label", label); button.title = label; }
    });
});
