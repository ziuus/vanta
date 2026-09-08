// Downloads the prebuilt vanta binary for this platform from the matching
// GitHub release. ponytail: linux-x64 only — vanta reads /proc and /sys and
// links libdbus, so there is nothing to ship for other platforms. Add targets
// here and in .github/workflows/release.yml when someone asks for them.
const fs = require('fs');
const os = require('os');
const path = require('path');
const https = require('https');
const { execFileSync } = require('child_process');

const { version } = require('./package.json');
const TARGET = 'x86_64-unknown-linux-gnu';
const ASSET = `vanta-${TARGET}.tar.gz`;
const URL = `https://github.com/ziuus/vanta/releases/download/v${version}/${ASSET}`;
const FROM_SOURCE = 'cargo install --git https://github.com/ziuus/vanta';

if (process.platform !== 'linux' || process.arch !== 'x64') {
  console.error(
    `vanta: no prebuilt binary for ${process.platform}-${process.arch} (linux-x64 only).`
  );
  console.error(`vanta: build it yourself with:  ${FROM_SOURCE}`);
  process.exit(1);
}

// GitHub redirects release downloads to a CDN host, so follow Location.
function download(url, hops = 0) {
  return new Promise((resolve, reject) => {
    if (hops > 5) return reject(new Error('too many redirects'));
    https
      .get(url, { headers: { 'user-agent': `vanta-tui/${version}` } }, (res) => {
        const { statusCode, headers } = res;
        if (statusCode >= 300 && statusCode < 400 && headers.location) {
          res.resume();
          return download(headers.location, hops + 1).then(resolve, reject);
        }
        if (statusCode !== 200) {
          res.resume();
          return reject(new Error(`HTTP ${statusCode} for ${url}`));
        }
        const chunks = [];
        res.on('data', (c) => chunks.push(c));
        res.on('end', () => resolve(Buffer.concat(chunks)));
        res.on('error', reject);
      })
      .on('error', reject);
  });
}

(async () => {
  const binDir = path.join(__dirname, 'bin');
  const dest = path.join(binDir, 'vanta-bin');
  const tgz = path.join(os.tmpdir(), `vanta-${version}-${process.pid}.tar.gz`);

  fs.mkdirSync(binDir, { recursive: true });
  process.stderr.write(`vanta: fetching ${ASSET}\n`);
  fs.writeFileSync(tgz, await download(URL));

  try {
    execFileSync('tar', ['-xzf', tgz, '-C', binDir, 'vanta']);
    fs.renameSync(path.join(binDir, 'vanta'), dest);
    fs.chmodSync(dest, 0o755);
  } finally {
    fs.rmSync(tgz, { force: true });
  }

  // Fail the install rather than leave a bin shim pointing at nothing.
  execFileSync(dest, ['--version'], { stdio: 'ignore' });
  process.stderr.write('vanta: installed — run `vanta`\n');
})().catch((err) => {
  console.error(`vanta: install failed — ${err.message}`);
  console.error(`vanta: build from source instead:  ${FROM_SOURCE}`);
  process.exit(1);
});
