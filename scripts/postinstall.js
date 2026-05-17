#!/usr/bin/env node
// Downloads the correct neboai binary for the current platform during npm install
const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");

const REPO = "NeboLoop/publisher";
const BINARY = "neboai";

function getPlatform() {
  const os = process.platform;
  const arch = process.arch;

  const osMap = { darwin: "darwin", linux: "linux", win32: "windows" };
  const archMap = { arm64: "arm64", x64: "amd64" };

  const platformOs = osMap[os];
  const platformArch = archMap[arch];

  if (!platformOs || !platformArch) {
    console.error(`Unsupported platform: ${os}-${arch}`);
    process.exit(1);
  }

  return `${platformOs}-${platformArch}`;
}

function getLatestRelease() {
  return new Promise((resolve, reject) => {
    const url = `https://api.github.com/repos/${REPO}/releases/latest`;
    https.get(url, { headers: { "User-Agent": "neboai-installer" } }, (res) => {
      let data = "";
      res.on("data", (chunk) => (data += chunk));
      res.on("end", () => {
        try {
          const json = JSON.parse(data);
          resolve(json.tag_name);
        } catch (e) {
          reject(new Error("Failed to parse release info"));
        }
      });
    }).on("error", reject);
  });
}

function downloadBinary(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, { headers: { "User-Agent": "neboai-installer" } }, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        // Follow redirect
        https.get(res.headers.location, { headers: { "User-Agent": "neboai-installer" } }, (res2) => {
          res2.pipe(file);
          file.on("finish", () => { file.close(); resolve(); });
        }).on("error", reject);
      } else if (res.statusCode === 200) {
        res.pipe(file);
        file.on("finish", () => { file.close(); resolve(); });
      } else {
        reject(new Error(`Download failed: HTTP ${res.statusCode}`));
      }
    }).on("error", reject);
  });
}

async function main() {
  const platform = getPlatform();
  const binDir = path.join(__dirname, "..", "bin");

  // Skip if binary already exists (e.g., prepacked)
  const binPath = path.join(binDir, process.platform === "win32" ? `${BINARY}.exe` : BINARY);
  if (fs.existsSync(binPath)) {
    console.log(`neboai binary already exists at ${binPath}`);
    return;
  }

  console.log(`Installing neboai for ${platform}...`);

  try {
    const version = await getLatestRelease();
    const ext = process.platform === "win32" ? ".exe" : "";
    const url = `https://github.com/${REPO}/releases/download/${version}/${BINARY}-${platform}${ext}`;

    fs.mkdirSync(binDir, { recursive: true });
    await downloadBinary(url, binPath);
    fs.chmodSync(binPath, 0o755);

    console.log(`Installed neboai ${version}`);
  } catch (err) {
    console.error(`Failed to install neboai: ${err.message}`);
    console.error("You can install manually: cargo install --path cli/");
    // Don't fail npm install — the skill still works for building artifacts
    process.exit(0);
  }
}

main();
