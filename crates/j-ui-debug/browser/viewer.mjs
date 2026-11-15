const screen = document.getElementById("screen");
const surface = document.getElementById("surface");
const stage = document.getElementById("stage");
const keyboard = document.getElementById("keyboard");
const status = document.getElementById("status");
const message = document.getElementById("message");
const root = document.getElementById("targets");
const proxies = new Map();
const pointers = new Set();
let current = null;
let stopped = false;
let composing = false;
let chain = Promise.resolve();
let epoch = 0;
let queued = 0;
let lastMove = 0;
let lastHeartbeat = 0;

async function request(path, body) {
    const options = { signal: AbortSignal.timeout(2000) };
    if (body !== undefined) {
        options.method = "POST";
        options.headers = { "Content-Type": "application/json" };
        options.body = JSON.stringify(body);
    }
    const response = await fetch(path, options);
    if (!response.ok) {
        const error = new Error("Native app unavailable.");
        error.status = response.status;
        throw error;
    }
    return response.json();
}

function modifiers(event) {
    return {
        alt: event.altKey,
        ctrl: event.ctrlKey,
        shift: event.shiftKey,
        meta: event.metaKey,
    };
}

function position(event) {
    const rect = screen.getBoundingClientRect();
    const x = Math.max(
        0,
        Math.min(1, (event.clientX - rect.left) / rect.width),
    );
    const y = Math.max(
        0,
        Math.min(1, (event.clientY - rect.top) / rect.height),
    );
    return { x, y };
}

function send(events) {
    if (
        !current ||
        queued >= 32 ||
        (document.hidden && events[0].kind !== "reset")
    )
        return;
    const { session, frame, generation } = current;
    const version = epoch;
    queued += 1;
    chain = chain
        .then(async () => {
            if (
                !current ||
                version !== epoch ||
                current.session !== session ||
                current.generation !== generation
            )
                return;
            await request("/input", { session, frame, generation, events });
        })
        .catch((error) => {
            epoch += 1;
            status.textContent = "Reconnecting";
            current = null;
            if (error.status === 409) return;
            surface.hidden = true;
            message.hidden = false;
        })
        .finally(() => {
            queued -= 1;
        });
}

function reset() {
    pointers.clear();
    epoch += 1;
    send([{ kind: "reset" }]);
}

function fit() {
    if (!current) return;
    const scale = Math.min(
        Math.max(1, stage.clientWidth - 24) / current.width,
        Math.max(1, stage.clientHeight - 24) / current.height,
    );
    surface.style.width = `${current.width * scale}px`;
    surface.style.height = `${current.height * scale}px`;
}

const roles = {
    Button: "button",
    CheckBox: "checkbox",
    Switch: "switch",
    Slider: "slider",
    TextInput: "textbox",
    MultilineTextInput: "textbox",
    ComboBox: "combobox",
    Tab: "tab",
    Link: "link",
    RadioButton: "radio",
    Label: "group",
    Heading: "heading",
};

function render(frame, image) {
    screen.width = frame.width;
    screen.height = frame.height;
    screen.getContext("2d").drawImage(image, 0, 0);
    screen.dataset.session = frame.session;
    screen.dataset.frame = frame.frame;
    const ids = new Set();
    for (const target of frame.targets) {
        ids.add(target.id);
        let node = proxies.get(target.id);
        if (!node) {
            node = document.createElement("div");
            node.dataset.nativeId = target.id;
            root.append(node);
            proxies.set(target.id, node);
        }
        const details = [target.label, target.path, target.source]
            .filter(Boolean)
            .join(" — ");
        node.id = target.path;
        node.setAttribute("role", roles[target.role] || "group");
        node.setAttribute("aria-label", details);
        node.setAttribute("aria-disabled", String(!target.enabled));
        node.dataset.nativeRole = target.role;
        node.dataset.source = target.source || "";
        node.textContent = details;
        const [left, top, width, height] = target.rect;
        node.style.left = `${left * 100}%`;
        node.style.top = `${top * 100}%`;
        node.style.width = `${width * 100}%`;
        node.style.height = `${height * 100}%`;
        node.style.zIndex = target.z_order;
    }
    for (const [id, node] of proxies) {
        if (ids.has(id)) continue;
        node.remove();
        proxies.delete(id);
    }
    surface.hidden = false;
    message.hidden = true;
    fit();
}

async function poll() {
    if (stopped) return;
    if (!document.hidden) {
        try {
            const next = await request("/snapshot");
            const changed =
                !current ||
                current.session !== next.session ||
                current.generation !== next.generation;
            if (changed) {
                epoch += 1;
                try {
                    await request("/input", {
                        session: next.session,
                        frame: next.frame,
                        generation: next.generation,
                        events: [{ kind: "reset" }],
                    });
                } catch (error) {
                    if (error.status !== 409) throw error;
                }
            }
            if (
                !current ||
                next.session !== current.session ||
                next.frame !== current.frame
            ) {
                const image = new Image();
                image.src = `data:image/png;base64,${next.png}`;
                await image.decode();
                current = next;
                render(next, image);
            }
            status.textContent = "Live";
            if (
                document.hasFocus() &&
                document.activeElement === keyboard &&
                performance.now() - lastHeartbeat > 500
            ) {
                lastHeartbeat = performance.now();
                send([{ kind: "keep_alive" }]);
            }
        } catch {
            current = null;
            epoch += 1;
            surface.hidden = true;
            message.hidden = false;
            message.textContent =
                "Start the native app to connect. The view will reconnect automatically.";
            status.textContent = "Offline";
        }
    }
    setTimeout(poll, 150);
}

surface.addEventListener("pointerdown", (event) => {
    if (!current || event.button > 2) return;
    event.preventDefault();
    keyboard.focus({ preventScroll: true });
    pointers.add(event.pointerId);
    screen.setPointerCapture(event.pointerId);
    send([
        {
            kind: "button",
            ...position(event),
            button: event.button,
            pressed: true,
            modifiers: modifiers(event),
        },
    ]);
});
surface.addEventListener("pointerup", (event) => {
    if (event.button > 2) return;
    pointers.delete(event.pointerId);
    send([
        {
            kind: "button",
            ...position(event),
            button: event.button,
            pressed: false,
            modifiers: modifiers(event),
        },
    ]);
    if (screen.hasPointerCapture(event.pointerId))
        screen.releasePointerCapture(event.pointerId);
});
surface.addEventListener("pointermove", (event) => {
    if (performance.now() - lastMove < 50) return;
    lastMove = performance.now();
    send([{ kind: "move", ...position(event) }]);
});
surface.addEventListener("pointercancel", reset);
surface.addEventListener("lostpointercapture", (event) => {
    if (pointers.delete(event.pointerId)) reset();
});
surface.addEventListener("contextmenu", (event) => event.preventDefault());
surface.addEventListener(
    "wheel",
    (event) => {
        event.preventDefault();
        send([
            {
                kind: "wheel",
                dx: Math.max(-2000, Math.min(2000, event.deltaX)),
                dy: Math.max(-2000, Math.min(2000, event.deltaY)),
            },
        ]);
    },
    { passive: false },
);

function key(event, pressed) {
    if (pressed && event.key === "Escape" && event.ctrlKey && event.altKey) {
        event.preventDefault();
        reset();
        keyboard.blur();
        return;
    }
    if (
        event.isComposing ||
        composing ||
        ["Process", "Dead", "Unidentified"].includes(event.key)
    )
        return;
    if (["Shift", "Control", "Alt", "Meta"].includes(event.key)) {
        send([{ kind: "modifiers", modifiers: modifiers(event) }]);
        return;
    }
    if (event.key.length === 1 && !event.ctrlKey && !event.metaKey) return;
    event.preventDefault();
    const key = event.key.length === 1 ? event.key.toUpperCase() : event.key;
    send([
        {
            kind: "key",
            key,
            pressed,
            repeat: event.repeat,
            modifiers: modifiers(event),
        },
    ]);
}
keyboard.addEventListener("keydown", (event) => key(event, true));
keyboard.addEventListener("keyup", (event) => key(event, false));
keyboard.addEventListener("beforeinput", (event) => {
    if (event.isComposing || composing) return;
    event.preventDefault();
    if (event.data) send([{ kind: "text", text: event.data.slice(0, 2048) }]);
});
keyboard.addEventListener("paste", (event) => {
    event.preventDefault();
    const text = event.clipboardData.getData("text/plain").slice(0, 2048);
    if (text) send([{ kind: "text", text }]);
});
keyboard.addEventListener("compositionstart", () => {
    composing = true;
});
keyboard.addEventListener("compositionend", (event) => {
    composing = false;
    if (event.data) send([{ kind: "text", text: event.data.slice(0, 2048) }]);
    keyboard.value = "";
});
window.addEventListener("blur", reset);
document.addEventListener("visibilitychange", () => {
    if (document.hidden) reset();
});
window.addEventListener("pagehide", () => {
    reset();
    stopped = true;
});
screen.addEventListener("focus", () => keyboard.focus({ preventScroll: true }));
new ResizeObserver(fit).observe(stage);
void poll();
