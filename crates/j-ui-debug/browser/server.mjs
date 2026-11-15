import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { randomBytes } from "node:crypto";
import { isAbsolute, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { frame, input } from "./transport.mjs";

const assets = new Map([
    ["/", ["index.html", "text/html"]],
    ["/viewer.mjs", ["viewer.mjs", "text/javascript"]],
    ["/style.css", ["style.css", "text/css"]],
]);

export function createViewer(connection) {
    const token = randomBytes(32).toString("hex");
    const server = createServer(async (request, response) => {
        const origin = `http://127.0.0.1:${server.address().port}`;
        function reply(status, body, type = "application/json") {
            response.writeHead(status, {
                "Content-Type": type,
                "Cache-Control": "no-store",
                "X-Frame-Options": "DENY",
                "X-Content-Type-Options": "nosniff",
                "Content-Security-Policy":
                    "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'",
            });
            response.end(body);
        }
        const crossOrigin =
            request.headers.origin && request.headers.origin !== origin;
        if (
            request.headers.host !== new URL(origin).host ||
            request.headers["sec-fetch-site"] === "cross-site" ||
            crossOrigin
        )
            return reply(403, "{}");
        if (request.method === "GET" && request.url === "/health") {
            try {
                await frame(connection);
                return reply(200, '{"ready":true}');
            } catch {
                return reply(503, '{"ready":false}');
            }
        }
        if (request.url === "/" && request.method === "GET")
            response.setHeader(
                "Set-Cookie",
                `jui_view=${token}; HttpOnly; SameSite=Strict; Path=/`,
            );
        else if (
            !request.headers.cookie?.split("; ").includes(`jui_view=${token}`)
        )
            return reply(403, "{}");
        try {
            if (request.method === "GET" && assets.has(request.url)) {
                const [file, type] = assets.get(request.url);
                return reply(
                    200,
                    await readFile(new URL(file, import.meta.url)),
                    type,
                );
            }
            if (request.method === "GET" && request.url === "/snapshot")
                return reply(200, JSON.stringify(await frame(connection)));
            if (request.method !== "POST" || request.url !== "/input")
                return reply(404, "{}");
            if (
                request.headers.origin !== origin ||
                request.headers["content-type"] !== "application/json"
            )
                return reply(403, "{}");
            const chunks = [];
            let length = 0;
            for await (const chunk of request) {
                length += chunk.length;
                if (length > 16384) return reply(413, "{}");
                chunks.push(chunk);
            }
            const result = await input(
                connection,
                JSON.parse(Buffer.concat(chunks).toString("utf8")),
            );
            return reply(200, JSON.stringify(result));
        } catch (error) {
            const status = error.status === 409 ? 409 : 503;
            return reply(status, '{"error":"Native app unavailable."}');
        }
    });
    server.requestTimeout = 3000;
    server.headersTimeout = 3000;
    return server;
}

async function main() {
    const args = process.argv.slice(2);
    let connection;
    let port = 0;
    while (args.length) {
        const name = args.shift();
        if (name === "--connection") connection = args.shift();
        else if (name === "--port") port = Number(args.shift());
        else throw new Error(`Unknown argument: ${name}`);
    }
    if (
        !connection ||
        !isAbsolute(connection) ||
        !Number.isSafeInteger(port) ||
        port < 0 ||
        port > 65535
    )
        throw new Error(
            "Usage: node server.mjs --connection <absolute file> --port <0..65535>",
        );
    const server = createViewer(connection);
    await new Promise((accept, reject) => {
        server.once("error", reject);
        server.listen(port, "127.0.0.1", accept);
    });
    process.stdout.write(
        `${JSON.stringify({ url: `http://127.0.0.1:${server.address().port}/` })}\n`,
    );
    for (const signal of ["SIGINT", "SIGTERM"])
        process.once(signal, () => {
            server.close();
            server.closeAllConnections();
        });
}

if (
    process.argv[1] &&
    resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
    main().catch((error) => {
        process.stderr.write(`${error.message}\n`);
        process.exitCode = 1;
    });
}
