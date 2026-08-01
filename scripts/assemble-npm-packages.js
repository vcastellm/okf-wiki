#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');

const REPOSITORY_ROOT = path.resolve(__dirname, '..');
const DEFAULT_BIN_DIR = path.join(REPOSITORY_ROOT, 'target', 'npm-binaries');
const DEFAULT_OUT_DIR = path.join(REPOSITORY_ROOT, 'dist');
const VERSION_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

const AssemblyError = Error;

function usage() {
  return [
    'Usage: assemble-npm-packages.js [--version <version>] [--bin-dir <dir>] [--out-dir <dir>]',
    '',
    'Expected native binaries: <bin-dir>/<target>/<binary> from npm/platforms.json.',
  ].join('\n');
}

function parseArgs(argv) {
  const options = {};

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--help' || arg === '-h') {
      options.help = true;
      continue;
    }

    if (arg !== '--version' && arg !== '--bin-dir' && arg !== '--out-dir') {
      throw new AssemblyError(`Unknown argument ${JSON.stringify(arg)}.\n${usage()}`);
    }

    const value = argv[index + 1];
    if (!value || value.startsWith('--')) {
      throw new AssemblyError(`Missing value for ${arg}.\n${usage()}`);
    }
    index += 1;

    if (arg === '--version') {
      options.version = value;
    } else if (arg === '--bin-dir') {
      options.binDir = value;
    } else if (arg === '--out-dir') {
      options.outDir = value;
    }
  }

  return options;
}

function readJsonFile(filePath, label) {
  assertFile(filePath, label);
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch (error) {
    throw new AssemblyError(`${label} is not valid JSON: ${error.message}`);
  }
}

function writeJsonFile(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function assertDirectory(dirPath, label) {
  let stat;
  try {
    stat = fs.statSync(dirPath);
  } catch (error) {
    throw new AssemblyError(`${label} does not exist: ${dirPath}`);
  }

  if (!stat.isDirectory()) {
    throw new AssemblyError(`${label} is not a directory: ${dirPath}`);
  }
}

function assertFile(filePath, label) {
  let stat;
  try {
    stat = fs.statSync(filePath);
  } catch (error) {
    throw new AssemblyError(`${label} does not exist: ${filePath}`);
  }

  if (!stat.isFile()) {
    throw new AssemblyError(`${label} is not a file: ${filePath}`);
  }
}

function copyRequiredFile(sourcePath, destinationPath, label) {
  assertFile(sourcePath, label);
  fs.mkdirSync(path.dirname(destinationPath), { recursive: true });
  fs.copyFileSync(sourcePath, destinationPath);
}

function validateVersion(version) {
  if (typeof version !== 'string' || !VERSION_PATTERN.test(version)) {
    throw new AssemblyError(`--version must be an exact semantic version; received ${JSON.stringify(version)}.`);
  }
}

function validateTarget(target, index) {
  if (!target || typeof target !== 'object' || Array.isArray(target)) {
    throw new AssemblyError(`npm/platforms.json targets[${index}] must be an object.`);
  }

  for (const field of ['target', 'platform', 'architecture', 'binary']) {
    if (typeof target[field] !== 'string' || target[field].length === 0) {
      throw new AssemblyError(`npm/platforms.json targets[${index}].${field} must be a non-empty string.`);
    }
  }

  if (target.platform === 'linux' && target.environment !== 'gnu' && target.environment !== 'musl') {
    throw new AssemblyError(`npm/platforms.json targets[${index}].environment must be gnu or musl for Linux targets.`);
  }
}

function validatePlatforms(platforms) {
  if (!platforms || typeof platforms !== 'object' || !Array.isArray(platforms.targets)) {
    throw new AssemblyError('npm/platforms.json targets must be an array.');
  }

  const seenTargets = new Set();

  platforms.targets.forEach((target, index) => {
    validateTarget(target, index);
    if (seenTargets.has(target.target)) {
      throw new AssemblyError(`npm/platforms.json contains duplicate target ${target.target}.`);
    }
    seenTargets.add(target.target);
  });
}

function rootManifest(rootPackage, version) {
  return {
    name: rootPackage.name,
    version,
    description: rootPackage.description,
    license: rootPackage.license,
    repository: rootPackage.repository,
    homepage: rootPackage.homepage,
    bugs: rootPackage.bugs,
    engines: rootPackage.engines,
    files: ['bin/okf-wiki.js', 'bin/native/', 'npm/platforms.json', 'README.md', 'LICENSE'],
    bin: rootPackage.bin,
    publishConfig: {
      access: 'public',
      provenance: true,
    },
  };
}

function assembleNpmPackages(options = {}) {
  const rootDir = path.resolve(options.rootDir || REPOSITORY_ROOT);
  const packagePath = path.join(rootDir, 'package.json');
  const platformsPath = path.join(rootDir, 'npm', 'platforms.json');
  const rootPackage = readJsonFile(packagePath, 'package.json');
  const platforms = readJsonFile(platformsPath, 'npm/platforms.json');
  const version = options.version || rootPackage.version;
  const binDir = path.resolve(options.binDir || DEFAULT_BIN_DIR);
  const outDir = path.resolve(options.outDir || DEFAULT_OUT_DIR);

  validateVersion(version);
  validatePlatforms(platforms);
  assertDirectory(binDir, 'Native binary directory');
  if (fs.existsSync(outDir)) {
    throw new AssemblyError(`Output directory already exists: ${outDir}`);
  }
  fs.mkdirSync(outDir, { recursive: true });

  const rootPackageDir = path.join(outDir, rootPackage.name);
  fs.mkdirSync(rootPackageDir, { recursive: true });
  writeJsonFile(path.join(rootPackageDir, 'package.json'), rootManifest(rootPackage, version));
  copyRequiredFile(path.join(rootDir, 'bin', 'okf-wiki.js'), path.join(rootPackageDir, 'bin', 'okf-wiki.js'), 'launcher');
  fs.chmodSync(path.join(rootPackageDir, 'bin', 'okf-wiki.js'), 0o755);
  copyRequiredFile(platformsPath, path.join(rootPackageDir, 'npm', 'platforms.json'), 'npm/platforms.json');
  copyRequiredFile(path.join(rootDir, 'README.md'), path.join(rootPackageDir, 'README.md'), 'README.md');
  copyRequiredFile(path.join(rootDir, 'LICENSE'), path.join(rootPackageDir, 'LICENSE'), 'LICENSE');

  for (const target of platforms.targets) {
    const sourceBinary = path.join(binDir, target.target, target.binary);
    const destinationBinary = path.join(rootPackageDir, 'bin', 'native', target.target, target.binary);

    if (!fs.existsSync(sourceBinary)) {
      throw new AssemblyError(`Missing native binary for ${target.target}: ${sourceBinary}`);
    }

    copyRequiredFile(sourceBinary, destinationBinary, `native binary for ${target.target}`);
    if (target.platform !== 'win32') {
      fs.chmodSync(destinationBinary, 0o755);
    }
  }

  return {
    rootPackage: rootPackage.name,
    outDir,
    version,
  };
}

function main(argv = process.argv.slice(2)) {
  const options = parseArgs(argv);
  if (options.help) {
    process.stdout.write(`${usage()}\n`);
    return;
  }

  const result = assembleNpmPackages(options);
  process.stdout.write(`Assembled npm package ${result.rootPackage} in ${result.outDir} for ${result.version}.\n`);
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}

module.exports = {
  AssemblyError,
  assembleNpmPackages,
  parseArgs,
};
