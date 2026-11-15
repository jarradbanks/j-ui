import { readFile } from "node:fs/promises";

async function request(connection, path, body) {
    const config = JSON.parse(await readFile(connection, "utf8"));
    const url = new URL(config.url);
    const valid =
        url.protocol === "http:" &&
        url.hostname === "127.0.0.1" &&
        url.port &&
        url.pathname === "/" &&
        !url.search &&
        !url.hash &&
        !url.username &&
        !url.password;
    if (!valid || !/^[a-f0-9]{64}$/.test(config.token))
        throw new Error("Invalid native connection.");
    const options = {
        headers: { Authorization: `Bearer ${config.token}` },
        signal: AbortSignal.timeout(1500),
        redirect: "error",
    };
    if (body !== undefined) {
        options.method = "POST";
        options.headers["Content-Type"] = "application/json";
        options.body = JSON.stringify(body);
    }
    const response = await fetch(`${config.url}${path}`, options);
    if (!response.ok) {
        const error = new Error("Native app unavailable.");
        error.status = response.status;
        throw error;
    }
    return response;
}

function validTargets(targets) {
    if (!Array.isArray(targets) || targets.length > 4096) return false;
    const ids = new Set();
    for (const target of targets) {
        if (
            !target ||
            !["id", "path", "label", "role"].every(
                (key) =>
                    typeof target[key] === "string" &&
                    target[key].length <= 4096,
            )
        )
            return false;
        if (ids.has(target.id)) return false;
        ids.add(target.id);
        if (target.source !== null && typeof target.source !== "string")
            return false;
        if (
            !Array.isArray(target.rect) ||
            target.rect.length !== 4 ||
            !target.rect.every(Number.isFinite)
        )
            return false;
        const [left, top, width, height] = target.rect;
        if (
            left < 0 ||
            top < 0 ||
            width <= 0 ||
            height <= 0 ||
            left + width > 1.0001 ||
            top + height > 1.0001
        )
            return false;
        if (
            !Number.isSafeInteger(target.z_order) ||
            target.z_order < 0 ||
            typeof target.enabled !== "boolean"
        )
            return false;
    }
    return true;
}

export async function frame(connection) {
    const response = await request(connection, "/snapshot");
    const chunks = [];
    let size = 0;
    for await (const chunk of response.body) {
        size += chunk.length;
        if (size > 32 * 1024 * 1024)
            throw new Error("Native snapshot too large.");
        chunks.push(chunk);
    }
    const body = Buffer.concat(chunks);
    if (body.length < 4) throw new Error("Invalid native snapshot.");
    const length = body.readUInt32LE(0);
    if (length > 8 * 1024 * 1024 || length + 4 > body.length)
        throw new Error("Invalid native metadata.");
    const metadata = JSON.parse(body.subarray(4, length + 4).toString("utf8"));
    const bytes = body.subarray(length + 4);
    const numbers = ["frame", "generation", "width", "height"];
    if (
        !/^[a-f0-9]{16}$/.test(metadata.session) ||
        !numbers.every(
            (key) => Number.isSafeInteger(metadata[key]) && metadata[key] > 0,
        )
    )
        throw new Error("Invalid native identity.");
    if (
        !Number.isFinite(metadata.ppp) ||
        metadata.ppp <= 0 ||
        metadata.width > 8192 ||
        metadata.height > 8192
    )
        throw new Error("Invalid native size.");
    if (
        bytes.length < 24 ||
        bytes.subarray(0, 8).toString("hex") !== "89504e470d0a1a0a" ||
        bytes.readUInt32BE(16) !== metadata.width ||
        bytes.readUInt32BE(20) !== metadata.height ||
        !validTargets(metadata.targets)
    )
        throw new Error("Invalid native frame.");
    return { ...metadata, png: bytes.toString("base64") };
}

export async function input(connection, message) {
    if (
        !message ||
        !Array.isArray(message.events) ||
        message.events.length === 0 ||
        message.events.length > 64 ||
        Buffer.byteLength(JSON.stringify(message)) > 16384
    )
        throw new Error("Invalid native input.");
    await request(connection, "/input", message);
    return { accepted: true };
}
