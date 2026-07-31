'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { test } = require('node:test');

const { validateVersionSync } = require('../../scripts/check-version-sync.js');

const VERSION = '0.1.0';
const PLATFORM_TARGETS = [
  ['x86_64-unknown-linux-gnu', 'okf-wiki-linux-x64-gnu'],
  ['aarch64-unknown-linux-gnu', 'okf-wiki-linux-arm64-gnu'],
  ['x86_64-unknown-linux-musl', 'okf-wiki-linux-x64-musl'],
  ['aarch64-unknown-linux-musl', 'okf-wiki-linux-arm64-musl'],
  ['x86_64-apple-darwin', 'okf-wiki-darwin-x64'],
  ['aarch64-apple-darwin', 'okf-wiki-darwin-arm64'],
  ['x86_64-pc-windows-msvc', 'okf-wiki-win32-x64-msvc'],
];

function createFixture(overrides = {}) {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'okf-wiki-version-sync-'));
  const cargoVersion = overrides.cargoVersion || VERSION;
  const npmVersion = overrides.npmVersion || VERSION;
  const platformsVersion = overrides.platformsVersion || VERSION;
  const targets = overrides.targets || PLATFORM_TARGETS.map(([target, packageName]) => ({
    target,
    package: packageName,
  }));
  const optionalDependencies = {
    ...Object.fromEntries(PLATFORM_TARGETS.map(([, packageName]) => [packageName, npmVersion])),
    ...(overrides.optionalDependencies || {}),
  };
  for (const packageName of overrides.removeOptionalDependencies || []) {
    delete optionalDependencies[packageName];
  }

  fs.mkdirSync(path.join(rootDir, 'npm'));
  fs.writeFileSync(path.join(rootDir, 'Cargo.toml'), `[package]\nname = "okf-wiki"\nversion = "${cargoVersion}"\n\n[dependencies.some-dependency]\nversion = "9.9.9"\n`);
  fs.writeFileSync(path.join(rootDir, 'package.json'), `${JSON.stringify({
    name: 'okf-wiki',
    version: npmVersion,
    optionalDependencies,
  }, null, 2)}\n`);
  fs.writeFileSync(path.join(rootDir, 'npm', 'platforms.json'), `${JSON.stringify({
    version: platformsVersion,
    targets,
  }, null, 2)}\n`);

  return rootDir;
}

function withFixture(overrides, callback) {
  const rootDir = createFixture(overrides);
  try {
    return callback(rootDir);
  } finally {
    fs.rmSync(rootDir, { recursive: true, force: true });
  }
}

function assertValidationFails(overrides, expectedMessage, environment = {}) {
  withFixture(overrides, (rootDir) => {
    const result = validateVersionSync(rootDir, environment);
    assert.equal(result.valid, false);
    assert.match(result.errors.join('\n'), expectedMessage);
  });
}

test('Given matching distribution metadata, When the contract is validated, Then it passes', () => {
  withFixture({}, (rootDir) => {
    assert.deepEqual(validateVersionSync(rootDir, {}).valid, true);
  });
});

test('Given a dependency version in another Cargo section, When the contract is validated, Then only the package version is used', () => {
  withFixture({}, (rootDir) => {
    const result = validateVersionSync(rootDir, {});
    assert.equal(result.valid, true);
    assert.equal(result.version, VERSION);
  });
});

test('Given a mismatched Cargo package version, When the contract is validated, Then it fails', () => {
  assertValidationFails({ cargoVersion: '0.2.0' }, /does not match Cargo\.toml \[package\]\.version/);
});

test('Given a mismatched root npm version, When the contract is validated, Then it fails', () => {
  assertValidationFails({ npmVersion: '0.2.0' }, /package\.json version .* does not match/);
});

test('Given a mismatched platform metadata version, When the contract is validated, Then it fails', () => {
  assertValidationFails({ platformsVersion: '0.2.0' }, /npm\/platforms\.json version .* does not match/);
});

test('Given a missing optional dependency, When the contract is validated, Then it fails the exact package-set check', () => {
  assertValidationFails({ removeOptionalDependencies: ['okf-wiki-linux-x64-gnu'] }, /optionalDependencies package set .*missing okf-wiki-linux-x64-gnu/);
});

test('Given an unexpected optional dependency, When the contract is validated, Then it fails the exact package-set check', () => {
  assertValidationFails({ optionalDependencies: { 'okf-wiki-extra': VERSION } }, /optionalDependencies package set .*unexpected okf-wiki-extra/);
});

test('Given an optional dependency with a different exact version, When the contract is validated, Then it fails', () => {
  assertValidationFails({ optionalDependencies: { 'okf-wiki-linux-x64-gnu': '0.2.0' } }, /optionalDependency okf-wiki-linux-x64-gnu .* exact version 0\.1\.0/);
});

test('Given an optional dependency with a loose semver range, When the contract is validated, Then it fails', () => {
  assertValidationFails({ optionalDependencies: { 'okf-wiki-linux-x64-gnu': '^0.1.0' } }, /optionalDependency okf-wiki-linux-x64-gnu .* exact version 0\.1\.0/);
});

test('Given duplicate platform targets, When the contract is validated, Then it fails', () => {
  const targets = PLATFORM_TARGETS.map(([target, packageName]) => ({ target, package: packageName }));
  targets[1].target = targets[0].target;
  assertValidationFails({ targets }, /duplicate target x86_64-unknown-linux-gnu/);
});

test('Given duplicate platform packages, When the contract is validated, Then it fails', () => {
  const targets = PLATFORM_TARGETS.map(([target, packageName]) => ({ target, package: packageName }));
  targets[1].package = targets[0].package;
  assertValidationFails({ targets }, /duplicate package okf-wiki-linux-x64-gnu/);
});

test('Given a v tag for another version, When the contract is validated, Then it fails', () => {
  assertValidationFails({}, /GITHUB_REF_NAME v0\.2\.0 does not match v0\.1\.0/, { GITHUB_REF_NAME: 'v0.2.0' });
});

test('Given workflow_dispatch without a tag, When the contract is validated, Then it remains valid', () => {
  withFixture({}, (rootDir) => {
    const result = validateVersionSync(rootDir, { GITHUB_EVENT_NAME: 'workflow_dispatch' });
    assert.equal(result.valid, true);
  });
});

test('Given a non-v ref name, When the contract is validated, Then tag validation is skipped', () => {
  withFixture({}, (rootDir) => {
    const result = validateVersionSync(rootDir, { GITHUB_REF_NAME: 'release-0.1.0' });
    assert.equal(result.valid, true);
  });
});
