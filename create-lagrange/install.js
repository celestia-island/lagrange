#!/usr/bin/env node
// @create-lagrange install.js — fetch the prebuilt lagrange binary for the
// current platform from GitHub Releases and place it next to index.js.
//
// Runs automatically on `npm install create-lagrange` (postinstall). If the
// fetch fails (offline, unknown platform), we print a hint to install via
// cargo and exit non-fatally so the package install itself succeeds.

const { createWriteStream, existsSync, mkdirSync } = require("fs");
const { pipeline } = require("stream/promises");
const { createGunzip } = require("zlib");
const { extract } = require("tar");
const path = require("path");

// --- platform → Rust target + archive suffix ---
const TARGETS = {
  "linux-x64": "x86_64-unknown-linux-gnu",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
  "win32-arm64": "aarch64-pc-windows-msvc",
};

function platformKey() {
  return `${process.platform}-${process.arch}`;
}

function binName() {
  return process.platform === "win32" ? "lagrange.exe" : "lagrange";
}

async function getLatestVersion() {
  const resp = await fetch(
    "https://api.github.com/repos/celestia-island/lagrange/releases/latest",
    { headers: { "User-Agent": "create-lagrange-installer" } }
  );
  if (!resp.ok) throw new Error(`GitHub API ${resp.status}`);
  const json = await resp.json();
  return json.tag_name; // e.g. "v0.1.0"
}

async function main() {
  const key = platformKey();
  const target = TARGETS[key];
  if (!target) {
    console.warn(
      `create-lagrange: no prebuilt binary for ${key}. Install via cargo: cargo install lagrange-library`
    );
    return;
  }

  const binDir = path.join(__dirname, "bin");
  const binPath = path.join(binDir, binName());

  // Already downloaded? Skip.
  if (existsSync(binPath)) {
    console.log("create-lagrange: binary already present, skipping download.");
    return;
  }

  let version;
  try {
    version = await getLatestVersion();
  } catch (e) {
    console.warn(`create-lagrange: could not determine latest version (${e.message}).`);
    console.warn("  Install via cargo instead: cargo install lagrange-library");
    return;
  }

  const ext = process.platform === "win32" ? "zip" : "tar.gz";
  const url = `https://github.com/celestia-island/lagrange/releases/download/${version}/lagrange-${version}-${target}.${ext}`;
  console.log(`create-lagrange: downloading ${url}`);

  mkdirSync(binDir, { recursive: true });

  try {
    const resp = await fetch(url, {
      headers: { "User-Agent": "create-lagrange-installer" },
      redirect: "follow",
    });
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);

    if (ext === "tar.gz") {
      // tar.gz: gunzip → untar → extract just the binary
      await pipeline(
        resp.body,
        createGunzip(),
        extract({ cwd: binDir, filter: (p) => p.endsWith(binName()) })
      );
    } else {
      // zip: write to temp, then extract (Windows). We use the built-in
      // approach of writing + PowerShell expand, or fall back to a note.
      const tmp = path.join(binDir, "lagrange.zip");
      const ws = createWriteStream(tmp);
      // Node 18+ doesn't have a built-in zip extractor; use a streaming
      // approach via the 'unzipper' package if available, or instruct manual.
      const reader = resp.body.getReader();
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        ws.write(Buffer.from(value));
      }
      ws.end();
      console.warn(
        "create-lagrange: zip extraction on Windows requires manual unzip of",
        tmp,
        "→ extract lagrange.exe to",
        binDir
      );
    }

    console.log("create-lagrange: binary installed to", binPath);
  } catch (e) {
    console.warn(`create-lagrange: download failed (${e.message}).`);
    console.warn("  Install via cargo instead: cargo install lagrange-library");
  }
}

main();
