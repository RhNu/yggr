#!/usr/bin/env node
import { spawn } from "node:child_process";
import { createConnection } from "node:net";

const BACKEND_HOST = "127.0.0.1";
const BACKEND_PORT = 18080;
const POLL_INTERVAL = 500;

const COLORS = {
  backend: "\x1b[36m",
  frontend: "\x1b[35m",
  dev: "\x1b[33m",
};
const RESET = "\x1b[0m";

function log(tag, msg) {
  const c = COLORS[tag] ?? "";
  console.log(`${c}[${tag}]${RESET} ${msg}`);
}

function isPortOpen(host, port) {
  return new Promise((resolve) => {
    const socket = createConnection({ host, port });
    socket.setTimeout(1000);
    socket.once("connect", () => {
      socket.destroy();
      resolve(true);
    });
    socket.once("error", () => {
      socket.destroy();
      resolve(false);
    });
    socket.once("timeout", () => {
      socket.destroy();
      resolve(false);
    });
  });
}

async function waitForBackend(child) {
  while (true) {
    if (child.exitCode !== null) return false;
    if (await isPortOpen(BACKEND_HOST, BACKEND_PORT)) return true;
    await new Promise((r) => setTimeout(r, POLL_INTERVAL));
  }
}

function spawnCmd(command, args, tag) {
  const color = COLORS[tag] ?? "";
  const isWin = process.platform === "win32";
  const child = isWin
    ? spawn("cmd", ["/c", command, ...args])
    : spawn(command, args);

  function pipe(stream) {
    let buf = "";
    stream.on("data", (data) => {
      buf += data.toString();
      const lines = buf.split("\n");
      buf = lines.pop() ?? "";
      for (const line of lines) {
        if (line.trim()) {
          process.stdout.write(`${color}[${tag}]${RESET} ${line}\n`);
        }
      }
    });
  }

  pipe(child.stdout);
  pipe(child.stderr);

  return child;
}

function killTree(child, tag) {
  if (!child || child.exitCode !== null) return;
  log(tag, "stopping...");
  try {
    if (process.platform === "win32") {
      spawn("taskkill", ["/pid", String(child.pid), "/T", "/F"]);
    } else {
      child.kill("SIGTERM");
    }
  } catch {
    // ignore
  }
}

async function main() {
  let backend = null;
  let frontend = null;
  let exiting = false;

  const cleanup = (code) => {
    if (exiting) return;
    exiting = true;
    log("dev", "Shutting down...");
    killTree(frontend, "frontend");
    killTree(backend, "backend");
    process.exit(code ?? 0);
  };

  process.on("SIGINT", () => cleanup(0));
  process.on("SIGTERM", () => cleanup(0));

  log("dev", "Starting backend (cargo run)...");
  backend = spawnCmd("cargo", ["run"], "backend");

  const ready = await waitForBackend(backend);
  if (!ready) {
    log("dev", "Backend failed to start");
    cleanup(1);
    return;
  }

  log("dev", "Backend is ready. Starting frontend...");
  frontend = spawnCmd("pnpm", ["--filter", "frontend", "dev"], "frontend");

  backend.on("exit", (code) => {
    log("dev", `Backend exited (code ${code})`);
    cleanup(code ?? 1);
  });

  frontend.on("exit", (code) => {
    log("dev", `Frontend exited (code ${code})`);
    cleanup(code ?? 0);
  });
}

main();
