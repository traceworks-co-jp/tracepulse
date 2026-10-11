import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { chromium } from "@playwright/test";
import pngToIco from "png-to-ico";

const assets = new URL("../assets/", import.meta.url);
const browser = await chromium.launch();
try {
    const page = await browser.newPage();
    async function extractSymbol(filename) {
        await page.setContent(await readFile(new URL(filename, assets), "utf8"));
        return page.evaluate(() => {
            const root = document.querySelector("svg");
            const paths = [...root.querySelectorAll("path")];
            const silhouette = paths.map(path => path.getBBox())
                .reduce((largest, box) => box.width * box.height > largest.width * largest.height ? box : largest);
            const selected = paths.filter(path => {
                const box = path.getBBox();
                return box.x >= silhouette.x - 1 && box.y >= silhouette.y - 1
                    && box.x + box.width <= silhouette.x + silhouette.width + 1
                    && box.y + box.height <= silhouette.y + silhouette.height + 1;
            });
            if (!selected.length || selected.length === paths.length) throw new Error("Cannot separate symbol from wordmark");
            const boxes = selected.map(path => path.getBBox());
            const left = Math.min(...boxes.map(box => box.x));
            const top = Math.min(...boxes.map(box => box.y));
            const right = Math.max(...boxes.map(box => box.x + box.width));
            const bottom = Math.max(...boxes.map(box => box.y + box.height));
            const side = Math.max(right - left, bottom - top) * 1.1;
            const x = (left + right - side) / 2;
            const y = (top + bottom - side) / 2;
            const serializer = new XMLSerializer();
            return { viewBox: `${x} ${y} ${side} ${side}`, x, y, side,
                paths: selected.map(path => serializer.serializeToString(path)).join("") };
        });
    }

    function svgIcon(symbol, size, background = false) {
        const backdrop = background ? `<rect x="${symbol.x}" y="${symbol.y}" width="${symbol.side}" height="${symbol.side}" fill="white"/>` : "";
        return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="${symbol.viewBox}">${backdrop}${symbol.paths}</svg>`;
    }

    async function rasterize(symbol, size, background = false) {
        await page.setViewportSize({ width: size, height: size });
        await page.setContent(`<style>html,body{margin:0;padding:0}svg{display:block}</style>${svgIcon(symbol, size, background)}`);
        const png = await page.screenshot({ type: "png", omitBackground: true });
        const visiblePixels = await page.evaluate(async base64 => {
            const image = new Image();
            image.src = `data:image/png;base64,${base64}`;
            await image.decode();
            const canvas = document.createElement("canvas");
            canvas.width = image.width;
            canvas.height = image.height;
            const context = canvas.getContext("2d");
            context.drawImage(image, 0, 0);
            const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
            let count = 0;
            for (let index = 3; index < pixels.length; index += 4) if (pixels[index] > 0) count++;
            return count;
        }, png.toString("base64"));
        assert(visiblePixels > 0, `Blank icon at ${size}px`);
        assert.equal(png.readUInt32BE(16), size);
        assert.equal(png.readUInt32BE(20), size);
        return png;
    }

    const light = await extractSymbol("TracePulse-logo-light_Community.svg");
    const dark = await extractSymbol("TracePulse-logo-dark_Community.svg");
    const adaptive = `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><style>.logo-dark{display:none}@media(prefers-color-scheme: dark){.logo-light{display:none}.logo-dark{display:inline}}</style><g class="logo-light">${svgIcon(light, 64)}</g><g class="logo-dark">${svgIcon(dark, 64)}</g></svg>\n`;
    await writeFile(new URL("favicon.svg", assets), adaptive);
    for (const [filename, symbol] of [["favicon.ico", light], ["favicon-dark.ico", dark]]) {
        const sizes = [16, 32, 48];
        const images = [];
        for (const size of sizes) images.push(await rasterize(symbol, size));
        const ico = await pngToIco(images);
        assert.equal(ico.readUInt16LE(2), 1);
        assert.equal(ico.readUInt16LE(4), sizes.length);
        sizes.forEach((size, index) => assert.equal(ico.readUInt8(6 + index * 16), size));
        await writeFile(new URL(filename, assets), ico);
    }
    await writeFile(new URL("apple-touch-icon.png", assets), await rasterize(light, 180, true));
    console.log("Generated adaptive SVG, 16/32/48px light and dark ICOs, and 180px Apple Touch Icon.");
} finally {
    await browser.close();
}