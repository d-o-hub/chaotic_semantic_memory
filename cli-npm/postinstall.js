#!/usr/bin/env node
/**
 * csm CLI postinstall script
 * Downloads platform-specific binary from GitHub Releases
 */

import { createWriteStream, mkdirSync, existsSync, chmodSync, readFileSync } from 'fs';
import { join, dirname, resolve, normalize } from 'path';
import { fileURLToPath } from 'url';
import { pipeline } from 'stream/promises';
import { Readable } from 'stream';
import { spawn } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Platform detection
const platform = process.platform;
const arch = process.arch;

// Map Node.js platform/arch to binary tarball names
const tarballMap = {
  'linux-x64': 'csm-linux-x64.tar.gz',
  'linux-arm64': 'csm-linux-arm64.tar.gz',
  'darwin-x64': 'csm-darwin-x64.tar.gz',
  'darwin-arm64': 'csm-darwin-arm64.tar.gz',
  'win32-x64': 'csm-win32-x64.tar.gz',
};

const key = `${platform}-${arch}`;
const tarballName = tarballMap[key];

// Hard whitelist of allowed tarball names for security
const allowedTarballs = Object.values(tarballMap);

if (!tarballName || !allowedTarballs.includes(tarballName)) {
  console.log(`Skipping binary download: unsupported platform ${platform}-${arch}`);
  console.log('Supported platforms: linux-x64, linux-arm64, darwin-x64, darwin-arm64, win32-x64');
  process.exit(0);
}

// Read package version
const pkgPath = resolve(__dirname, 'package.json');
const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
const version = pkg.version;

// Ensure version matches semver to prevent URL injection
if (!/^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?$/.test(version)) {
  console.error(`Invalid package version: ${version}`);
  process.exit(1);
}

// GitHub Release URL construction
const repo = 'd-o-hub/chaotic_semantic_memory';
const downloadUrl = `https://github.com/${repo}/releases/download/v${version}/${tarballName}`;

const binDir = resolve(__dirname, 'bin');
const binaryName = tarballName.replace('.tar.gz', '');
const binaryPath = normalize(join(binDir, platform === 'win32' ? `${binaryName}.exe` : binaryName));

// Safety check: ensure binaryPath is actually inside the package's bin directory
if (!binaryPath.startsWith(binDir)) {
  console.error('Invalid binary path detected.');
  process.exit(1);
}

// Ensure bin directory exists
if (!existsSync(binDir)) {
  mkdirSync(binDir, { recursive: true });
}

// Check if binary already exists (skip re-download)
if (existsSync(binaryPath)) {
  console.log(`Binary already exists at ${binaryPath}, skipping download`);
  process.exit(0);
}

console.log(`Downloading csm v${version} for ${platform}-${arch}...`);
console.log(`URL: ${downloadUrl}`);

try {
  const response = await fetch(downloadUrl);

  if (!response.ok) {
    if (response.status === 404) {
      console.error(`Binary not found for version v${version}`);
      console.error('This version may not have been released yet.');
      console.error('Check available releases at: https://github.com/d-o-hub/chaotic_semantic_memory/releases');
    } else {
      console.error(`Download failed: HTTP ${response.status}`);
    }
    process.exit(1);
  }

  // Convert web ReadableStream to Node.js Readable stream
  const nodeStream = Readable.fromWeb(response.body);

  // Use tar command to extract. We use -xz directly.
  // Security: shell: false prevents shell injection, and we use a whitelisted binDir.
  const tar = spawn('tar', ['-xz', '-f', '-', '-C', binDir], { shell: false });

  await pipeline(
    nodeStream,
    tar.stdin
  );

  // Wait for tar to finish
  await new Promise((resolve, reject) => {
    tar.on('close', (code) => {
      if (code === 0) resolve();
      else reject(new Error(`tar exited with code ${code}`));
    });
    tar.on('error', reject);
  });

  // Make binary executable (Unix only)
  if (platform !== 'win32') {
    chmodSync(binaryPath, 0o755);
  }

  console.log(`Successfully installed csm v${version}`);
  console.log(`Binary location: ${binaryPath}`);
} catch (err) {
  console.error(`Failed to download binary: ${err.message}`);
  console.error('');
  console.error('Alternative installation methods:');
  console.error('  1. Install from crates.io: cargo install chaotic_semantic_memory --bin csm');
  console.error('  2. Download manually from: https://github.com/d-o-hub/chaotic_semantic_memory/releases');
  process.exit(1);
}
