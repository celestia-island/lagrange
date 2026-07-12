#!/usr/bin/env node
// @create-lagrange entry point — spawns the downloaded lagrange binary with
// `init`, forwarding all CLI args so `npx create-lagrange my-docs` becomes
// `lagrange init --dir my-docs ...`.

const { spawn } = require("child_process");
const { existsSync } = require("fs");
const path = require("path");

const binName = process.platform === "win32" ? "lagrange.exe" : "lagrange";
const binPath = path.join(__dirname, "bin", binName);

if (!existsSync(binPath)) {
  console.error("create-lagrange: binary not found at", binPath);
  console.error("  Re-run: npm install create-lagrange");
  console.error("  Or install via cargo: cargo install lagrange-library");
  process.exit(1);
}

// The first arg after `npx create-lagrange` is the target directory (like
// create-vite / create-next-app). Map it to `lagrange init --dir <arg>`.
const userArgs = process.argv.slice(2);
const child = spawn(binPath, ["init", ...userArgs], {
  stdio: "inherit",
});

child.on("exit", (code) => process.exit(code ?? 1));
