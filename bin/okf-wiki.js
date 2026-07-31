#!/usr/bin/env node
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawn } = require('child_process');
const { createRequire } = require('module');

const platforms = require('../npm/platforms.json');

class LauncherError extends Error {
  constructor(message, code, cause) {
    super(message);
    this.name = 'LauncherError';
    this.code = code;
    if (cause) {
      this.cause = cause;
    }
  }
}

function detectLinuxLibc(processLike) {
  const runtime = processLike || process;

  try {
    if (!runtime.report || typeof runtime.report.getReport !== 'function') {
      return 'gnu';
    }

    const report = runtime.report.getReport();
    const header = report && report.header ? report.header : {};
    if (header.glibcVersionRuntime || header.glibcVersionCompiler) {
      return 'gnu';
    }

    const sharedObjects = Array.isArray(report && report.sharedObjects)
      ? report.sharedObjects
      : [];
    const normalizedObjects = sharedObjects.map((entry) => String(entry).toLowerCase());

    if (normalizedObjects.some((entry) => entry.includes('musl'))) {
      return 'musl';
    }

    if (normalizedObjects.some((entry) => entry.includes('libc.so.6') || entry.includes('ld-linux'))) {
      return 'gnu';
    }
  } catch (_error) {
    return 'gnu';
  }

  return 'gnu';
}

function currentRuntime(processLike) {
  const runtime = processLike || process;
  const descriptor = {
    platform: runtime.platform,
    arch: runtime.arch,
  };

  if (descriptor.platform === 'linux') {
    descriptor.libc = detectLinuxLibc(runtime);
  }

  return descriptor;
}

function runtimeLabel(runtime) {
  return [runtime.platform, runtime.arch, runtime.libc].filter(Boolean).join('/');
}

function targetLabel(target) {
  return [target.platform, target.architecture, target.environment].filter(Boolean).join('/');
}

function selectTarget(targets, runtime) {
  const descriptor = runtime || currentRuntime();
  const selected = targets.find((target) => {
    if (target.platform !== descriptor.platform || target.architecture !== descriptor.arch) {
      return false;
    }

    if (descriptor.platform === 'linux') {
      return target.environment === descriptor.libc;
    }

    return true;
  });

  if (!selected) {
    const supported = targets.map(targetLabel).join(', ');
    throw new LauncherError(
      `Unsupported okf-wiki platform: ${runtimeLabel(descriptor)}. Supported platforms: ${supported}.`,
      'ERR_OKF_WIKI_UNSUPPORTED_PLATFORM'
    );
  }

  return selected;
}

function packageResolver(baseDir) {
  const anchor = path.join(path.resolve(baseDir || __dirname), '__okf_wiki_launcher__.js');
  return createRequire(anchor);
}

function resolvePackageRoot(packageName, options) {
  const baseDir = options && options.baseDir ? options.baseDir : __dirname;
  const resolver = options && options.require ? options.require : packageResolver(baseDir);

  try {
    return path.dirname(resolver.resolve(`${packageName}/package.json`));
  } catch (error) {
    throw new LauncherError(
      `The optional native package "${packageName}" is not installed. Reinstall okf-wiki with optional dependencies enabled for this platform (for example: npm install okf-wiki, or pnpm install without --no-optional).`,
      'ERR_OKF_WIKI_OPTIONAL_PACKAGE_MISSING',
      error
    );
  }
}

function resolveNativeBinary(target, options) {
  const packageRoot = resolvePackageRoot(target.package, options);
  const binaryPath = path.join(packageRoot, 'bin', target.binary);

  try {
    const stat = fs.statSync(binaryPath);
    if (!stat.isFile()) {
      throw new LauncherError(
        `The native okf-wiki binary for package "${target.package}" exists but is not a file: ${binaryPath}`,
        'ERR_OKF_WIKI_BINARY_NOT_FILE'
      );
    }
  } catch (error) {
    if (error instanceof LauncherError) {
      throw error;
    }

    throw new LauncherError(
      `The optional native package "${target.package}" is installed, but its binary was not found at ${binaryPath}. Reinstall okf-wiki and ensure optional package contents were not pruned.`,
      'ERR_OKF_WIKI_BINARY_MISSING',
      error
    );
  }

  return binaryPath;
}

function signalExitCode(signal) {
  const signalNumber = os.constants && os.constants.signals
    ? os.constants.signals[signal]
    : undefined;
  return typeof signalNumber === 'number' ? 128 + signalNumber : 1;
}

function forwardExit(code, signal, processLike) {
  const runtime = processLike || process;

  if (signal) {
    try {
      runtime.kill(runtime.pid, signal);
      const fallback = setTimeout(() => {
        runtime.exit(signalExitCode(signal));
      }, 500);
      if (typeof fallback.unref === 'function') {
        fallback.unref();
      }
    } catch (_error) {
      runtime.exit(signalExitCode(signal));
    }
    return;
  }

  runtime.exit(code === null || code === undefined ? 1 : code);
}

function run(argv, options) {
  const opts = options || {};
  const target = selectTarget(platforms.targets, opts.runtime || currentRuntime(opts.process));
  const binaryPath = resolveNativeBinary(target, opts);
  const spawnImpl = opts.spawn || spawn;
  const child = spawnImpl(binaryPath, argv || [], {
    stdio: 'inherit',
    windowsHide: false,
  });

  child.on('error', (error) => {
    const stderr = opts.stderr || process.stderr;
    stderr.write(`Failed to launch native okf-wiki binary at ${binaryPath}: ${error.message}\n`);
    (opts.process || process).exit(1);
  });

  child.on('exit', (code, signal) => {
    forwardExit(code, signal, opts.process || process);
  });

  return child;
}

function main() {
  try {
    run(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exit(1);
  }
}

module.exports = {
  LauncherError,
  currentRuntime,
  detectLinuxLibc,
  forwardExit,
  resolveNativeBinary,
  resolvePackageRoot,
  run,
  selectTarget,
  signalExitCode,
};

if (require.main === module) {
  main();
}
