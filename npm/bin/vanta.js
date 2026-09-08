#!/usr/bin/env node
// Thin shim: exec the native binary that install.js downloaded next to us.
const path = require('path');
const fs = require('fs');
const { spawnSync } = require('child_process');

const bin = path.join(__dirname, 'vanta-bin');

if (!fs.existsSync(bin)) {
  console.error('vanta: binary missing — reinstall with `npm install -g vanta`');
  console.error('vanta: or build from source:  cargo install --git https://github.com/ziuus/vanta');
  process.exit(1);
}

const { status, signal } = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' });
process.exit(signal ? 1 : (status ?? 1));
