#!/usr/bin/env node
// Fetches the neboai binary that matches this package's version from the
// GitHub release (v<version>) into vendor/. Runs as postinstall, and again from
// the `neboai` launcher on first use when the package manager skipped
// postinstall (pnpm does by default for global installs).
"use strict";
const fs = require("fs");
const path = require("path");
const https = require("https");

const REPO = "NeboLoop/publisher";
const VERSION = require("../package.json").version;
const VENDOR = path.join(__dirname, "..", "vendor");
const BIN = path.join(VENDOR, process.platform === "win32" ? "neboai.exe" : "neboai");

function platform() {
  const os = { darwin: "darwin", linux: "linux", win32: "windows" }[process.platform];
  const arch = { arm64: "arm64", x64: "amd64" }[process.arch];
  if (!os || !arch) throw new Error(`unsupported platform ${process.platform}-${process.arch}`);
  // Windows on Arm runs the x64 build.
  return os === "windows" ? "windows-amd64" : `${os}-${arch}`;
}

function download(url, dest, redirects = 5) {
  return new Promise((resolve, reject) => {
    https
      .get(url, { headers: { "User-Agent": "neboai-installer" } }, (res) => {
        if ([301, 302, 303, 307, 308].includes(res.statusCode) && res.headers.location && redirects > 0) {
          res.resume();
          return download(res.headers.location, dest, redirects - 1).then(resolve, reject);
        }
        if (res.statusCode !== 200) {
          res.resume();
          return reject(new Error(`download failed: HTTP ${res.statusCode} for ${url}`));
        }
        const file = fs.createWriteStream(dest);
        res.pipe(file);
        file.on("finish", () => file.close(resolve));
        file.on("error", reject);
      })
      .on("error", reject);
  });
}

async function ensureBinary() {
  if (fs.existsSync(BIN)) return BIN;
  const ext = process.platform === "win32" ? ".exe" : "";
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/neboai-${platform()}${ext}`;
  fs.mkdirSync(VENDOR, { recursive: true });
  const tmp = `${BIN}.download`;
  await download(url, tmp);
  fs.chmodSync(tmp, 0o755);
  fs.renameSync(tmp, BIN);
  return BIN;
}

module.exports = { ensureBinary };

if (require.main === module) {
  ensureBinary()
    .then(() => console.log(`neboai ${VERSION} installed`))
    .catch((err) => {
      // Never fail the package install: the launcher retries on first use.
      console.warn(`neboai: could not fetch the binary now (${err.message}); it will be fetched on first run.`);
    });
}
