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

function writeFakePackage(root, target, script) {
  const packageRoot = path.join(root, 'node_modules', target.package);
  const binDir = path.join(packageRoot, 'bin');
  fs.mkdirSync(binDir, { recursive: true });
  fs.writeFileSync(
    path.join(packageRoot, 'package.json'),
    JSON.stringify({ name: target.package, version: '0.0.0' })
  );

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

test('Given supported runtime descriptors When selecting targets Then the expected platform package is returned', () => {
  const targets = platforms.targets;

  assert.equal(
    launcher.selectTarget(targets, { platform: 'linux', arch: 'x64', libc: 'gnu' }).package,
    'okf-wiki-linux-x64-gnu'
  );
  assert.equal(
    launcher.selectTarget(targets, { platform: 'linux', arch: 'x64', libc: 'musl' }).package,
    'okf-wiki-linux-x64-musl'
  );
  assert.equal(
    launcher.selectTarget(targets, { platform: 'linux', arch: 'arm64', libc: 'gnu' }).package,
    'okf-wiki-linux-arm64-gnu'
  );
  assert.equal(
    launcher.selectTarget(targets, { platform: 'linux', arch: 'arm64', libc: 'musl' }).package,
    'okf-wiki-linux-arm64-musl'
  );
  assert.equal(
    launcher.selectTarget(targets, { platform: 'darwin', arch: 'arm64' }).package,
    'okf-wiki-darwin-arm64'
  );
  assert.equal(
    launcher.selectTarget(targets, { platform: 'win32', arch: 'x64' }).package,
    'okf-wiki-win32-x64-msvc'
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

test('Given a fake optional package When resolving the native binary Then package-root bin path is returned', () => {
  const root = makeTempProject();
  const target = platforms.targets.find((entry) => entry.package === 'okf-wiki-linux-x64-gnu');
  assert.ok(target);
  const expectedBinary = writeFakePackage(root, target, '#!/usr/bin/env node\n');

  assert.equal(launcher.resolvePackageRoot(target.package, { baseDir: root }), path.dirname(path.dirname(expectedBinary)));
  assert.equal(launcher.resolveNativeBinary(target, { baseDir: root }), expectedBinary);
});

test('Given the selected optional package is omitted When resolving the binary Then an actionable error is thrown', () => {
  const root = makeTempProject();
  const target = platforms.targets.find((entry) => entry.package === 'okf-wiki-linux-x64-gnu');
  assert.ok(target);

  assert.throws(
    () => launcher.resolveNativeBinary(target, { baseDir: root }),
    (error) => {
      assert.equal(error.code, 'ERR_OKF_WIKI_OPTIONAL_PACKAGE_MISSING');
      assert.match(error.message, new RegExp(target.package));
      assert.match(error.message, /optional dependencies enabled/);
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

  writeFakePackage(root, target, `#!${process.execPath}
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
