let active = 0;
let bar: HTMLElement | null = null;
let finishTimer: number | undefined;
let removeTimer: number | undefined;
let status: HTMLElement | null = null;

export function initializeRefreshIndicator(anchor: HTMLElement | null): HTMLElement | null {
    if (!anchor) return null;
    if (!document.getElementById("refresh-indicator-styles")) {
        const style = document.createElement("style");
        style.id = "refresh-indicator-styles";
        style.textContent = `
            .refresh-anchor{position:relative;display:inline-block;padding-left:7em}
            .refresh-status{position:absolute;left:0;top:50%;transform:translateY(-50%);display:inline-flex;align-items:center;gap:.35em;white-space:nowrap;color:#5eead4;visibility:hidden}
            .refresh-status.active{visibility:visible}
            html[data-theme='light'] .refresh-status{color:#087d70}
            .refresh-spinner{box-sizing:border-box;width:12px;height:12px;border:1.5px solid currentColor;border-right-color:transparent;border-radius:50%;animation:refresh-spin .8s linear infinite}
            .refresh-top{position:fixed;top:0;left:0;right:0;height:2.5px;z-index:1200;pointer-events:none;overflow:hidden;opacity:1;transition:opacity 200ms ease}
            .refresh-top::before{content:"";display:block;width:100%;height:100%;background:linear-gradient(90deg,#00D2FF,#10B981);transform:scaleX(.3);transform-origin:left;animation:refresh-slide 1.5s ease-in-out infinite alternate}
            .refresh-top.complete::before{animation:none;transform:scaleX(1);transition:transform 150ms ease}
            .refresh-top.fade{opacity:0}
            @keyframes refresh-spin{to{transform:rotate(360deg)}}
            @keyframes refresh-slide{from{transform:translateX(-20%) scaleX(.3)}to{transform:translateX(60%) scaleX(.4)}}
            @media(prefers-reduced-motion:reduce){.refresh-spinner,.refresh-top::before{animation:none}.refresh-top,.refresh-top.complete::before{transition:none}}
        `;
        document.head.append(style);
    }
    const text = document.createElement("span");
    text.textContent = anchor.textContent;
    status = document.createElement("span");
    status.className = "refresh-status";
    status.setAttribute("role", "status");
    status.setAttribute("aria-live", "polite");
    anchor.classList.add("refresh-anchor");
    anchor.replaceChildren(status, text);
    const updateLanguage = (): void => {
        if (active && status) status.lastChild!.textContent = document.documentElement.lang === "ja" ? "更新中…" : "Updating…";
    };
    window.addEventListener("tracepulse-language-change", updateLanguage);
    return text;
}

export function showRefreshIndicator(): () => void {
    if (active++ === 0) {
        window.clearTimeout(finishTimer);
        window.clearTimeout(removeTimer);
        bar?.remove();
        bar = document.createElement("div");
        bar.className = "refresh-top";
        bar.setAttribute("aria-hidden", "true");
        document.body.append(bar);
        if (status) {
            const spinner = document.createElement("span");
            spinner.className = "refresh-spinner";
            spinner.setAttribute("aria-hidden", "true");
            status.replaceChildren(spinner, document.createTextNode(document.documentElement.lang === "ja" ? "更新中…" : "Updating…"));
            status.classList.add("active");
        }
    }
    let stopped = false;
    return () => {
        if (stopped) return;
        stopped = true;
        if (--active !== 0) return;
        status?.classList.remove("active");
        if (status) status.textContent = "";
        const completedBar = bar;
        if (!completedBar) return;
        if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
            completedBar.remove();
            bar = null;
            return;
        }
        completedBar.classList.add("complete");
        finishTimer = window.setTimeout(() => {
            completedBar.classList.add("fade");
            removeTimer = window.setTimeout(() => {
                completedBar.remove();
                if (bar === completedBar) bar = null;
            }, 200);
        }, 150);
    };
}
