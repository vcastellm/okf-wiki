'use strict';

const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const test = require('node:test');

const platforms = require('../../npm/platforms.json');

const REPOSITORY_ROOT = path.resolve(__dirname, '..', '..');
const SCRIPT_PATH = path.join(REPOSITORY_ROOT, 'scripts', 'assemble-npm-packages.js');
const VERSION = '9.8.7-test.1';

function makeTempDir(label) {
  return fs.mkdtempSync(path.join(os.tmpdir(), `okf-wiki-${label}-`));
}

function writeInputBinaries(binDir, skippedTarget) {
  for (const target of platforms.targets) {
    if (target.target === skippedTarget) {
      continue;
    }

    const targetDir = path.join(binDir, target.target);
    fs.mkdirSync(targetDir, { recursive: true });
    fs.writeFileSync(path.join(targetDir, target.binary), `native binary for ${target.target}\n`);
  }
}

function runAssembler(binDir, outDir) {
  return spawnSync(process.execPath, [
    SCRIPT_PATH,
    '--version', VERSION,
    '--bin-dir', binDir,
    '--out-dir', outDir,
  ], {
    cwd: REPOSITORY_ROOT,
    encoding: 'utf8',
  });
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function listTree(rootDir) {
  const entries = [];

  function visit(relativeDir) {
    const absoluteDir = path.join(rootDir, relativeDir);
    for (const entry of fs.readdirSync(absoluteDir, { withFileTypes: true })) {
      const relativePath = path.join(relativeDir, entry.name).split(path.sep).join('/');
      entries.push(entry.isDirectory() ? `${relativePath}/` : relativePath);
      if (entry.isDirectory()) {
        visit(relativePath);
      }
    }
  }

  visit('');
  return entries.sort();
}

function expectedTree() {
  return [
    'okf-wiki/',
    'okf-wiki/LICENSE',
    'okf-wiki/README.md',
    'okf-wiki/bin/',
    'okf-wiki/bin/okf-wiki.js',
    'okf-wiki/bin/native/',
    'okf-wiki/npm/',
    'okf-wiki/npm/platforms.json',
    'okf-wiki/package.json',
    ...platforms.targets.flatMap((target) => [
      `okf-wiki/bin/native/${target.target}/`,
      `okf-wiki/bin/native/${target.target}/${target.binary}`,
    ]),
  ].sort();
}

test('Given prebuilt native binaries When the npm package is assembled Then one package contains deterministic bundled native binaries', () => {
  const workspace = makeTempDir('assemble-packages');
  const binDir = path.join(workspace, 'prebuilt');
  const outDir = path.join(workspace, 'dist');
  writeInputBinaries(binDir);

  const result = runAssembler(binDir, outDir);

  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stderr, '');
  assert.deepEqual(listTree(outDir), expectedTree());

  const rootManifest = readJson(path.join(outDir, 'okf-wiki', 'package.json'));
  assert.equal(rootManifest.name, 'okf-wiki');
  assert.equal(rootManifest.version, VERSION);
  assert.deepEqual(rootManifest.files, ['bin/okf-wiki.js', 'bin/native/', 'npm/platforms.json', 'README.md', 'LICENSE']);
  assert.deepEqual(rootManifest.bin, { 'okf-wiki': 'bin/okf-wiki.js' });
  assert.equal(rootManifest.optionalDependencies, undefined);
  assert.equal(rootManifest.scripts, undefined);
  assert.equal(rootManifest.dependencies, undefined);
  assert.equal(rootManifest.devDependencies, undefined);

  assert.equal(
    fs.readFileSync(path.join(outDir, 'okf-wiki', 'bin', 'okf-wiki.js'), 'utf8'),
    fs.readFileSync(path.join(REPOSITORY_ROOT, 'bin', 'okf-wiki.js'), 'utf8')
  );
  assert.equal(
    fs.readFileSync(path.join(outDir, 'okf-wiki', 'npm', 'platforms.json'), 'utf8'),
    fs.readFileSync(path.join(REPOSITORY_ROOT, 'npm', 'platforms.json'), 'utf8')
  );

  for (const target of platforms.targets) {
    const binaryPath = path.join(outDir, 'okf-wiki', 'bin', 'native', target.target, target.binary);
    assert.equal(fs.readFileSync(binaryPath, 'utf8'), `native binary for ${target.target}\n`);

    if (target.platform !== 'win32') {
      assert.notEqual(fs.statSync(binaryPath).mode & 0o111, 0, `${target.target} binary should be executable`);
    } else {
      assert.ok(binaryPath.endsWith('okf-wiki.exe'));
    }
  }
});

test('Given a missing native binary When npm packages are assembled Then the failure names the missing input', () => {
  const workspace = makeTempDir('assemble-missing-binary');
  const binDir = path.join(workspace, 'prebuilt');
  const outDir = path.join(workspace, 'dist');
  const missingTarget = platforms.targets.find((target) => target.target === 'x86_64-pc-windows-msvc');
  assert.ok(missingTarget);
  writeInputBinaries(binDir, missingTarget.target);

  const result = runAssembler(binDir, outDir);

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Missing native binary/);
  assert.match(result.stderr, new RegExp(missingTarget.target));
  assert.match(result.stderr, /okf-wiki\.exe/);
});

test('Given an existing output directory When npm packages are assembled Then existing files are preserved', () => {
  const workspace = makeTempDir('assemble-existing-output');
  const binDir = path.join(workspace, 'prebuilt');
  const outDir = path.join(workspace, 'dist');
  const sentinel = path.join(outDir, 'keep.txt');
  writeInputBinaries(binDir);
  fs.mkdirSync(outDir);
  fs.writeFileSync(sentinel, 'keep me\n');

  const result = runAssembler(binDir, outDir);

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Output directory already exists/);
  assert.equal(fs.readFileSync(sentinel, 'utf8'), 'keep me\n');
});
