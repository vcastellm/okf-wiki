'use strict';

const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const test = require('node:test');

const launcher = require('../../bin/okf-wiki.js');
const platforms = require('../../npm/platforms.json');

function makeTempProject() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'okf-wiki-launcher-'));
}

function writeFakeBundledBinary(root, target, script) {
  const binDir = path.join(root, 'bin', 'native', target.target);
  fs.mkdirSync(binDir, { recursive: true });

  const binaryPath = path.join(binDir, target.binary);
  fs.writeFileSync(binaryPath, script);
  if (process.platform !== 'win32') {
    fs.chmodSync(binaryPath, 0o755);
  }

  return binaryPath;
}

test('Given linux libc reports When detecting libc Then glibc and musl are distinguished', () => {
  const glibcRuntime = {
    report: {
      getReport() {
        return { header: { glibcVersionRuntime: '2.39' }, sharedObjects: [] };
      },
    },
  };
  const muslRuntime = {
    report: {
      getReport() {
        return { header: {}, sharedObjects: ['/lib/ld-musl-x86_64.so.1'] };
      },
    },
  };

  assert.equal(launcher.detectLinuxLibc(glibcRuntime), 'gnu');
  assert.equal(launcher.detectLinuxLibc(muslRuntime), 'musl');
});

test('Given supported runtime descriptors When selecting targets Then the expected Rust target is returned', () => {
  const targets = platforms.targets;

  assert.equal(
    launcher.selectTarget(targets, { platform: 'linux', arch: 'x64', libc: 'gnu' }).target,
    'x86_64-unknown-linux-gnu'
  );
  assert.equal(
    launcher.selectTarget(targets, { platform: 'linux', arch: 'x64', libc: 'musl' }).target,
    'x86_64-unknown-linux-musl'
  );
  assert.equal(
    launcher.selectTarget(targets, { platform: 'linux', arch: 'arm64', libc: 'gnu' }).target,
    'aarch64-unknown-linux-gnu'
  );
  assert.equal(
    launcher.selectTarget(targets, { platform: 'linux', arch: 'arm64', libc: 'musl' }).target,
    'aarch64-unknown-linux-musl'
  );
  assert.equal(
    launcher.selectTarget(targets, { platform: 'darwin', arch: 'arm64' }).target,
    'aarch64-apple-darwin'
  );
  assert.equal(
    launcher.selectTarget(targets, { platform: 'win32', arch: 'x64' }).target,
    'x86_64-pc-windows-msvc'
  );
});

test('Given an unsupported runtime When selecting a target Then an actionable error is thrown', () => {
  assert.throws(
    () => launcher.selectTarget(platforms.targets, { platform: 'freebsd', arch: 'x64' }),
    (error) => {
      assert.equal(error.code, 'ERR_OKF_WIKI_UNSUPPORTED_PLATFORM');
      assert.match(error.message, /Unsupported okf-wiki platform: freebsd\/x64/);
      assert.match(error.message, /Supported platforms:/);
      return true;
    }
  );
});

test('Given a bundled native binary When resolving the native binary Then its deterministic local path is returned', () => {
  const root = makeTempProject();
  const target = platforms.targets.find((entry) => entry.target === 'x86_64-unknown-linux-gnu');
  assert.ok(target);
  const expectedBinary = writeFakeBundledBinary(root, target, '#!/usr/bin/env node\n');

  assert.equal(launcher.resolveNativeBinary(target, { baseDir: root }), expectedBinary);
});

test('Given the selected bundled binary is absent When resolving the binary Then an actionable error is thrown', () => {
  const root = makeTempProject();
  const target = platforms.targets.find((entry) => entry.target === 'x86_64-unknown-linux-gnu');
  assert.ok(target);

  assert.throws(
    () => launcher.resolveNativeBinary(target, { baseDir: root }),
    (error) => {
      assert.equal(error.code, 'ERR_OKF_WIKI_BINARY_MISSING');
      assert.match(error.message, new RegExp(target.target));
      assert.ok(error.message.includes(path.join('bin', 'native', target.target, target.binary)));
      return true;
    }
  );
});

test('Given a fake native binary When running the launcher Then argv stdin stdout stderr and exit code are forwarded', { skip: process.platform === 'win32' ? 'POSIX shebang executable required for fake native binary' : false }, () => {
  if (/\s/.test(process.execPath)) {
    return;
  }

  const runtime = launcher.currentRuntime();
  let target;

  try {
    target = launcher.selectTarget(platforms.targets, runtime);
  } catch (_error) {
    return;
  }

  const root = makeTempProject();
  fs.mkdirSync(path.join(root, 'bin'), { recursive: true });
  fs.mkdirSync(path.join(root, 'npm'), { recursive: true });
  fs.copyFileSync(path.join(__dirname, '..', '..', 'bin', 'okf-wiki.js'), path.join(root, 'bin', 'okf-wiki.js'));
  fs.copyFileSync(path.join(__dirname, '..', '..', 'npm', 'platforms.json'), path.join(root, 'npm', 'platforms.json'));

  writeFakeBundledBinary(root, target, `#!${process.execPath}
const fs = require('fs');
const input = fs.readFileSync(0, 'utf8');
process.stdout.write(JSON.stringify({ argv: process.argv.slice(2), input }) + '\\n');
process.stderr.write('fake stderr\\n');
process.exit(Number(process.env.OKF_WIKI_FAKE_EXIT || '0'));
`);

  const result = spawnSync(process.execPath, [path.join(root, 'bin', 'okf-wiki.js'), 'alpha', 'two words'], {
    cwd: root,
    input: 'stdin payload',
    encoding: 'utf8',
    env: { ...process.env, OKF_WIKI_FAKE_EXIT: '17' },
  });

  assert.equal(result.error, undefined);
  assert.equal(result.status, 17);
  assert.equal(result.signal, null);
  assert.match(result.stderr, /fake stderr/);
  assert.deepEqual(JSON.parse(result.stdout.trim()), {
    argv: ['alpha', 'two words'],
    input: 'stdin payload',
  });
});
