// The version lives in Cargo.toml. install.js derives the GitHub release
// download URL from package.json's version, so drift means a published package
// that downloads a tag that doesn't exist.
//
// This must run as its own process *before* `npm publish` — npm reads the
// manifest before it runs prepack, so a rewrite from inside prepack is packed
// with the stale version (verified: it published 0.2.1 with 0.2.2 in the
// tarball). `npm run release` chains them correctly.
//
// Pass an expected version to assert against, e.g. the release tag:
//   node sync-version.js 0.2.2
const fs = require('fs');
const path = require('path');

const cargo = fs.readFileSync(path.join(__dirname, '..', 'Cargo.toml'), 'utf8');
// First `version =` in the file is [package]'s — a dependency's must not match.
const want = cargo.match(/^version = "(.+)"/m)?.[1];
if (!want) {
  console.error('sync-version: no package version found in Cargo.toml');
  process.exit(1);
}

const expected = process.argv[2];
if (expected && expected !== want) {
  console.error(`sync-version: asked for ${expected} but Cargo.toml says ${want}`);
  process.exit(1);
}

const file = path.join(__dirname, 'package.json');
const pkg = JSON.parse(fs.readFileSync(file, 'utf8'));
if (pkg.version === want) {
  console.log(`sync-version: already ${want}`);
} else {
  console.log(`sync-version: ${pkg.version} -> ${want}`);
  pkg.version = want;
  fs.writeFileSync(file, `${JSON.stringify(pkg, null, 2)}\n`);
}
