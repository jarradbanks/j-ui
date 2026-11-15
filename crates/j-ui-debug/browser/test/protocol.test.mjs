import test from "node:test";
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createViewer } from "../server.mjs";
import { frame } from "../transport.mjs";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { fileURLToPath } from "node:url";

test("CLI reports only its URL, fails occupied ports, and stops cleanly", async () => {
    const directory = await mkdtemp(join(tmpdir(), "j-ui-debug-cli-"));
    const script = fileURLToPath(new URL("../server.mjs", import.meta.url));
    const args = [
        script,
        "--connection",
        join(directory, "absent.json"),
        "--port",
    ];
    const child = spawn(process.execPath, [...args, "0"]);
    try {
        const [data] = await once(child.stdout, "data");
        const startup = JSON.parse(data.toString());
        assert.deepEqual(Object.keys(startup), ["url"]);
        assert.equal((await fetch(`${startup.url}health`)).status, 503);
        const collision = spawn(process.execPath, [
            ...args,
            new URL(startup.url).port,
        ]);
        let diagnostic = "";
        collision.stderr.on("data", (data) => {
            diagnostic += data;
        });
        const [code] = await once(collision, "exit");
        assert.equal(code, 1);
        assert.match(diagnostic, /EADDRINUSE/);
    } finally {
        const ended = once(child, "exit");
        child.kill("SIGTERM");
        const [code] = await ended;
        assert.equal(code, 0);
        await rm(directory, { recursive: true });
    }
});

async function listen(server) {
    await new Promise((accept) => server.listen(0, "127.0.0.1", accept));
    return `http://127.0.0.1:${server.address().port}`;
}

test("viewer guards requests and recovers from absent connections and native restarts", async () => {
    const directory = await mkdtemp(join(tmpdir(), "j-ui-debug-test-"));
    const connection = join(directory, "connection.json");
    let session = "1234567812345678";
    let badFrame = false;
    let received;
    const token = "f".repeat(64);
    const native = createServer(async (request, response) => {
        assert.equal(request.headers.authorization, `Bearer ${token}`);
        if (request.url === "/input") {
            let text = "";
            for await (const chunk of request) text += chunk;
            received = JSON.parse(text);
            response.end("{}");
            return;
        }
        const metadata = Buffer.from(
            JSON.stringify({
                session,
                frame: 1,
                generation: 1,
                width: 1,
                height: 1,
                ppp: 1,
                targets: [],
            }),
        );
        const length = Buffer.alloc(4);
        length.writeUInt32LE(metadata.length);
        const png = Buffer.from(
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j6xkAAAAASUVORK5CYII=",
            "base64",
        );
        if (badFrame) png.writeUInt32BE(2, 16);
        response.end(Buffer.concat([length, metadata, png]));
    });
    const viewer = createViewer(connection);
    try {
        const upstream = await listen(native);
        const url = await listen(viewer);
        assert.equal((await fetch(`${url}/health`)).status, 503);
        await writeFile(connection, JSON.stringify({ url: upstream, token }));
        assert.deepEqual(await (await fetch(`${url}/health`)).json(), {
            ready: true,
        });
        assert.equal((await fetch(`${url}/snapshot`)).status, 403);
        assert.equal(
            (await fetch(url, { headers: { Origin: "https://example.com" } }))
                .status,
            403,
        );
        assert.equal(
            (await fetch(url, { headers: { "Sec-Fetch-Site": "cross-site" } }))
                .status,
            403,
        );
        const root = await fetch(url);
        const cookie = root.headers.get("set-cookie").split(";")[0];
        assert.match(
            root.headers.get("set-cookie"),
            /HttpOnly; SameSite=Strict/,
        );
        const options = { headers: { Cookie: cookie } };
        assert.equal(
            (await (await fetch(`${url}/snapshot`, options)).json()).session,
            session,
        );
        session = "abcdefabcdefabcd";
        assert.equal(
            (await (await fetch(`${url}/snapshot`, options)).json()).session,
            session,
        );
        assert.equal(
            (
                await fetch(`${url}/input`, {
                    method: "POST",
                    headers: {
                        Cookie: cookie,
                        "Content-Type": "application/json",
                    },
                    body: "{}",
                })
            ).status,
            403,
        );
        const body = {
            session,
            frame: 1,
            generation: 1,
            events: [{ kind: "reset" }],
        };
        assert.equal(
            (
                await fetch(`${url}/input`, {
                    method: "POST",
                    headers: {
                        Cookie: cookie,
                        Origin: url,
                        "Content-Type": "application/json",
                    },
                    body: JSON.stringify(body),
                })
            ).status,
            200,
        );
        assert.deepEqual(received, body);
        badFrame = true;
        assert.equal((await fetch(`${url}/health`)).status, 503);
        await assert.rejects(frame(connection), /Invalid native frame/);
        await writeFile(
            connection,
            JSON.stringify({ url: "https://example.com", token }),
        );
        await assert.rejects(frame(connection), /Invalid native connection/);
    } finally {
        viewer.closeAllConnections();
        native.closeAllConnections();
        await Promise.all([
            new Promise((accept) => viewer.close(accept)),
            new Promise((accept) => native.close(accept)),
        ]);
        await rm(directory, { recursive: true });
    }
});
