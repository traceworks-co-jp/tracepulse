export function showOperationProgress(container: HTMLElement, label: string, message: (seconds: number) => string): () => void {
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
    const update = (): void => { status.textContent = message(Math.floor((performance.now() - started) / 1000)); };
    update();
    const timer = window.setInterval(update, 1000);
    return () => { window.clearInterval(timer); element.remove(); };
}