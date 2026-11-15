const assert = require("node:assert/strict");
const { createServer } = require("node:http");
const { mkdtemp, readFile, writeFile, rm, mkdir } = require("node:fs/promises");
const { tmpdir } = require("node:os");
const { join } = require("node:path");
const { chromium } = require("playwright-core");
const { execFile } = require("node:child_process");
const { promisify } = require("node:util");

async function polish(view, state) {
    if (!process.env.JUI_POLISH_CHECK) return;
    const css = await readFile(join(__dirname, "../style.css"), "utf8");
    const image = await view
        .locator("canvas")
        .evaluate((canvas) => canvas.toDataURL());
    let html = await view.content();
    html = html.replace(
        '<link rel="stylesheet" href="/style.css">',
        `<style>${css}</style>`,
    );
    html = html.replace(
        /<canvas[\s\S]*?<\/canvas>/,
        `<img id="screen" src="${image}" alt="Native app">`,
    );
    html = html.replace(
        /<script type="module"[\s\S]*?<\/script>/,
        `<script>
    const stage = document.getElementById("stage");
    const surface = document.getElementById("surface");
    function fit() {
      const scale = Math.min((stage.clientWidth - 24) / 1280, (stage.clientHeight - 24) / 800);
      surface.style.width = 1280 * scale + "px";
      surface.style.height = 800 * scale + "px";
    }
    new ResizeObserver(fit).observe(stage);
    fit();
  </script>`,
    );
    const snapshot = createServer((_request, response) => {
        response.setHeader("Content-Type", "text/html");
        response.end(html);
    });
    await new Promise((accept) => snapshot.listen(0, "127.0.0.1", accept));
    try {
        const url = `http://127.0.0.1:${snapshot.address().port}/`;
        const result = await promisify(execFile)(process.execPath, [
            process.env.JUI_POLISH_CHECK,
            "--url",
            url,
            "--widths",
            "390,800,1440,1920",
            "--ignore",
            "#targets > div",
        ]);
        process.stdout.write(`${state} rendered snapshot: ${result.stdout}`);
        if (process.env.JUI_CAPTURE_DIR)
            await writeFile(
                join(process.env.JUI_CAPTURE_DIR, `polish-${state}.html`),
                html,
            );
    } finally {
        snapshot.closeAllConnections();
        await new Promise((accept) => snapshot.close(accept));
    }
}

async function check() {
    const { createViewer } = await import("../server.mjs");
    const directory = await mkdtemp(join(tmpdir(), "j-ui-browser-test-"));
    const connection = join(directory, "connection.json");
    const browser = await chromium.launch({
        headless: true,
        executablePath: process.env.CHROMIUM_PATH,
    });
    const token = "f".repeat(64);
    let session = "1234567812345678";
    let online = true;
    let dynamic = true;
    let generation = 1;
    let inputDelay = 0;
    const inputs = [];
    const page = await browser.newPage();
    const png = Buffer.from(
        await page.evaluate(() => {
            const canvas = document.createElement("canvas");
            canvas.width = 1280;
            canvas.height = 800;
            const context = canvas.getContext("2d");
            context.fillStyle = "#121416";
            context.fillRect(0, 0, 1280, 800);
            context.fillStyle = "#edeef1";
            context.fillText("Native Snapshot Fixture", 24, 40);
            return canvas.toDataURL().split(",")[1];
        }),
        "base64",
    );
    await page.close();
    const targets = [
        {
            id: "level",
            path: "demo/mixer/master/level",
            label: "Master Level",
            role: "Slider",
            source: "examples/demo.rs:141",
            rect: [0.1, 0.2, 0.3, 0.05],
            z_order: 0,
            enabled: true,
        },
        {
            id: "apply",
            path: "egui/apply",
            label: "Apply",
            role: "Button",
            source: null,
            rect: [0.1, 0.4, 0.2, 0.05],
            z_order: 1,
            enabled: true,
        },
        {
            id: "overlay",
            path: "egui/overlay",
            label: "Overlay",
            role: "Button",
            source: null,
            rect: [0.15, 0.4, 0.04, 0.05],
            z_order: 2,
            enabled: true,
        },
    ];
    const native = createServer(async (request, response) => {
        if (!online) {
            response.writeHead(503);
            response.end();
            return;
        }
        assert.equal(request.headers.authorization, `Bearer ${token}`);
        if (request.url === "/input") {
            let body = "";
            for await (const chunk of request) body += chunk;
            const input = JSON.parse(body);
            if (input.generation !== generation) {
                response.writeHead(409);
                response.end("{}");
                return;
            }
            inputs.push(input);
            await new Promise((accept) => setTimeout(accept, inputDelay));
            response.end("{}");
            return;
        }
        const metadata = Buffer.from(
            JSON.stringify({
                session,
                frame: 1,
                generation,
                width: 1280,
                height: 800,
                ppp: 1,
                targets: dynamic ? targets : targets.slice(0, 2),
            }),
        );
        const length = Buffer.alloc(4);
        length.writeUInt32LE(metadata.length);
        response.end(Buffer.concat([length, metadata, png]));
    });
    const viewer = createViewer(connection);
    try {
        await new Promise((accept) => native.listen(0, "127.0.0.1", accept));
        await new Promise((accept) => viewer.listen(0, "127.0.0.1", accept));
        const url = `http://127.0.0.1:${viewer.address().port}/`;
        for (const dpi of [1, 1.5]) {
            const context = await browser.newContext({
                deviceScaleFactor: dpi,
            });
            const view = await context.newPage();
            const errors = [];
            view.on("pageerror", (error) => errors.push(error.message));
            await view.goto(url);
            if (dpi === 1) {
                await view
                    .getByRole("status")
                    .filter({ hasText: "Offline" })
                    .waitFor();
                await writeFile(
                    connection,
                    JSON.stringify({
                        url: `http://127.0.0.1:${native.address().port}`,
                        token,
                    }),
                );
            }
            await view
                .getByRole("status")
                .filter({ hasText: "Live" })
                .waitFor();
            if (dpi === 1) await polish(view, "Live");
            assert.equal(
                await view
                    .locator("#feedback, #nodes, #draw, #annotate")
                    .count(),
                0,
            );
            for (const width of [390, 800, 1440, 1920]) {
                await view.setViewportSize({ width, height: 900 });
                await view.waitForTimeout(100);
                const geometry = await view.evaluate((targets) => {
                    const image = document
                        .getElementById("screen")
                        .getBoundingClientRect();
                    return targets.map((target) => {
                        const node = document.getElementById(target.path);
                        const rect = node.getBoundingClientRect();
                        return {
                            id: target.id,
                            actual: [rect.x, rect.y, rect.width, rect.height],
                            expected: [
                                image.x + target.rect[0] * image.width,
                                image.y + target.rect[1] * image.height,
                                target.rect[2] * image.width,
                                target.rect[3] * image.height,
                            ],
                            hit: document.elementFromPoint(
                                rect.x + rect.width / 2,
                                rect.y + rect.height / 2,
                            ).dataset.nativeId,
                        };
                    });
                }, targets);
                for (const target of geometry) {
                    target.actual.forEach((coordinate, index) =>
                        assert.ok(
                            Math.abs(coordinate - target.expected[index]) < 1,
                        ),
                    );
                    assert.equal(target.hit, target.id);
                }
            }
            await view.locator("#keyboard").focus();
            inputs.length = 0;
            await view.keyboard.press("Space");
            await view.keyboard.press("a");
            await view.keyboard.insertText("é");
            await view
                .locator("#keyboard")
                .evaluate((node) =>
                    node.dispatchEvent(
                        new CompositionEvent("compositionstart"),
                    ),
                );
            await view.locator("#keyboard").dispatchEvent("keydown", {
                key: "Process",
                isComposing: true,
            });
            await view.locator("#keyboard").dispatchEvent("beforeinput", {
                data: "漢",
                isComposing: true,
            });
            await view
                .locator("#keyboard")
                .evaluate((node) =>
                    node.dispatchEvent(
                        new CompositionEvent("compositionend", { data: "漢" }),
                    ),
                );
            await view.waitForTimeout(200);
            const keyboardEvents = inputs.flatMap((batch) => batch.events);
            for (const key of ["Space", "A"]) {
                assert.ok(
                    keyboardEvents.some(
                        (event) =>
                            event.kind === "key" &&
                            event.key === key &&
                            event.pressed,
                    ),
                );
                assert.ok(
                    keyboardEvents.some(
                        (event) =>
                            event.kind === "key" &&
                            event.key === key &&
                            !event.pressed,
                    ),
                );
            }
            for (const text of [" ", "a", "é", "漢"]) {
                assert.equal(
                    keyboardEvents.filter(
                        (event) => event.kind === "text" && event.text === text,
                    ).length,
                    1,
                    `text ${JSON.stringify(text)}: ${JSON.stringify(keyboardEvents)}`,
                );
            }
            inputDelay = 80;
            inputs.length = 0;
            await view.keyboard.type("Z".repeat(60), { delay: 0 });
            await view.waitForTimeout(1000);
            assert.equal(
                inputs
                    .flatMap((batch) => batch.events)
                    .filter((event) => event.kind === "text")
                    .map((event) => event.text)
                    .join(""),
                "Z".repeat(60),
                "bounded batching must preserve a fast text burst behind a delayed request",
            );
            assert.ok(inputs.every((batch) => batch.events.length <= 64));
            inputs.length = 0;
            for (let count = 0; count < 4; count += 1)
                await view.keyboard.insertText("漢".repeat(2048));
            await view.waitForTimeout(600);
            assert.equal(
                inputs
                    .flatMap((batch) => batch.events)
                    .filter((event) => event.kind === "text")
                    .map((event) => event.text)
                    .join(""),
                "漢".repeat(8192),
            );
            assert.ok(
                inputs.every(
                    (batch) =>
                        Buffer.byteLength(JSON.stringify(batch)) <= 16384,
                ),
            );
            inputs.length = 0;
            inputDelay = 350;
            const stalled = view.waitForRequest(
                (request) =>
                    request.url().endsWith("/input") &&
                    request
                        .postDataJSON()
                        .events.some(
                            (event) =>
                                event.kind === "key" &&
                                event.key === "ArrowRight",
                        ),
            );
            await view.keyboard.down("ArrowRight");
            await stalled;
            await view.keyboard.type("obsolete", { delay: 0 });
            await view.keyboard.up("ArrowRight");
            await view.waitForTimeout(1000);
            const stale = inputs.flatMap((batch) => batch.events);
            assert.ok(stale.some((event) => event.kind === "reset"));
            assert.ok(!stale.some((event) => event.kind === "text"));
            inputDelay = 80;
            await view.keyboard.press("ArrowDown");
            generation += 1;
            dynamic = false;
            await view
                .locator('[data-native-id="overlay"]')
                .waitFor({ state: "detached" });
            inputs.length = 0;
            for (let count = 0; count < 7; count += 1)
                await view.keyboard.press("ArrowUp");
            await view
                .getByRole("button", { name: /^Apply —/ })
                .click({ delay: 150 });
            await view.waitForTimeout(600);
            const resumed = inputs.flatMap((batch) => batch.events);
            assert.equal(
                resumed.filter(
                    (event) =>
                        event.kind === "key" &&
                        event.key === "ArrowUp" &&
                        event.pressed,
                ).length,
                7,
            );
            assert.ok(
                resumed.some(
                    (event) => event.kind === "button" && event.pressed,
                ),
            );
            assert.ok(
                resumed.some(
                    (event) => event.kind === "button" && !event.pressed,
                ),
            );
            assert.ok(inputs.every((batch) => batch.generation === generation));
            inputDelay = 0;
            await view
                .getByRole("button", { name: /^Apply —/ })
                .click({ delay: 100 });
            await view.waitForTimeout(150);
            assert.ok(
                inputs.some((batch) =>
                    batch.events.some(
                        (event) => event.kind === "button" && event.pressed,
                    ),
                ),
            );
            assert.ok(
                inputs.some((batch) =>
                    batch.events.some(
                        (event) => event.kind === "button" && !event.pressed,
                    ),
                ),
            );
            await view.locator("#keyboard").dispatchEvent("keydown", {
                key: "Escape",
                ctrlKey: true,
                altKey: true,
            });
            await view.waitForTimeout(100);
            assert.ok(
                inputs.some((batch) =>
                    batch.events.some((event) => event.kind === "reset"),
                ),
            );
            dynamic = false;
            session = "abcdefabcdefabcd";
            await view.waitForFunction(
                () =>
                    document.getElementById("screen").dataset.session ===
                    "abcdefabcdefabcd",
            );
            assert.equal(
                await view.locator('[data-native-id="overlay"]').count(),
                0,
            );
            online = false;
            await view
                .getByRole("status")
                .filter({ hasText: "Offline" })
                .waitFor();
            assert.ok(await view.locator("#surface").isHidden());
            if (dpi === 1) await polish(view, "Offline");
            online = true;
            dynamic = true;
            session = "1234567812345678";
            await view
                .getByRole("status")
                .filter({ hasText: "Live" })
                .waitFor();
            assert.deepEqual(errors, []);
            if (process.env.JUI_CAPTURE_DIR) {
                await mkdir(process.env.JUI_CAPTURE_DIR, { recursive: true });
                await view.screenshot({
                    path: join(
                        process.env.JUI_CAPTURE_DIR,
                        `viewer-${dpi}.png`,
                    ),
                });
            }
            await context.close();
        }
        console.log(
            "PASSED browser geometry/input/reconnect: 390/800/1440/1920, DPI 1.0/1.5, native path/source proxies, z-order hit testing, dynamic removal, keyboard reset, no duplicate annotation UI or page errors. Fixture transport; actual Orca Annotate/Draw acceptance is separate.",
        );
    } finally {
        viewer.closeAllConnections();
        native.closeAllConnections();
        await Promise.all([
            new Promise((accept) => viewer.close(accept)),
            new Promise((accept) => native.close(accept)),
        ]);
        await browser.close();
        await rm(directory, { recursive: true });
    }
}

check().catch((error) => {
    console.error(error);
    process.exitCode = 1;
});
