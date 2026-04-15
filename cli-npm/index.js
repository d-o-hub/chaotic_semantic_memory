#!/usr/bin/env node
/**
 * csm CLI binary runner
 * Detects platform and executes the appropriate binary
 */

import { spawn } from 'child_process';
import { join, dirname, resolve, normalize } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Platform detection
const platform = process.platform;
const arch = process.arch;

// Map Node.js platform/arch to binary names
const binaryMap = {
  'linux-x64': 'csm-linux-x64',
  'linux-arm64': 'csm-linux-arm64',
  'darwin-x64': 'csm-darwin-x64',
  'darwin-arm64': 'csm-darwin-arm64',
  'win32-x64': 'csm-win32-x64.exe',
};

const key = `${platform}-${arch}`;
const binaryName = binaryMap[key];

// Hard whitelist of allowed binary names for security
const allowedBinaries = Object.values(binaryMap);

if (!binaryName || !allowedBinaries.includes(binaryName)) {
  console.error(`Unsupported platform: ${platform}-${arch}`);
  console.error('Supported platforms: linux-x64, linux-arm64, darwin-x64, darwin-arm64, win32-x64');
  process.exit(1);
}

// Securely resolve the binary path
const binDir = resolve(__dirname, 'bin');
const binaryPath = normalize(join(binDir, binaryName));

// Safety check: ensure binaryPath is actually inside the package's bin directory
if (!binaryPath.startsWith(binDir)) {
  console.error('Invalid binary path detected.');
  process.exit(1);
}

// Spawn the binary with all arguments
const args = process.argv.slice(2);
const child = spawn(binaryPath, args, {
  stdio: 'inherit',
  env: process.env,
  shell: false, // Security: explicitly disable shell
});

child.on('error', (err) => {
  if (err.code === 'ENOENT') {
    console.error(`Binary not found at: ${binaryPath}`);
    console.error('Run npm install to download the platform binary.');
  } else {
    console.error(`Failed to execute binary: ${err.message}`);
  }
  process.exit(1);
});

child.on('exit', (code, signal) => {
  if (signal) {
    process.exit(1);
  } else {
    process.exit(code || 0);
  }
});
