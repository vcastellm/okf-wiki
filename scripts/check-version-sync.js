'use strict';

const fs = require('node:fs');
const path = require('node:path');

const REPOSITORY_ROOT = path.resolve(__dirname, '..');
const EXACT_VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

function parseCargoPackageVersion(contents) {
  let section = '';

  for (const line of contents.split(/\r?\n/)) {
    const sectionMatch = line.match(/^\s*\[([^\]]+)\]\s*(?:#.*)?$/);
    if (sectionMatch) {
      section = sectionMatch[1].trim();
      continue;
    }

    if (section !== 'package') {
      continue;
    }

    const versionMatch = line.match(/^\s*version\s*=\s*(["'])([^"']+)\1\s*(?:#.*)?$/);
    if (versionMatch) {
      return versionMatch[2];
    }
  }

  return undefined;
}

function readJsonFile(filePath, label, errors) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch (error) {
    errors.push(`${label} could not be read: ${error.message}`);
    return undefined;
  }
}

function validateVersionSync(rootDir = REPOSITORY_ROOT, environment = process.env) {
  const errors = [];
  const cargoPath = path.join(rootDir, 'Cargo.toml');
  const packagePath = path.join(rootDir, 'package.json');
  const platformsPath = path.join(rootDir, 'npm', 'platforms.json');

  let cargoVersion;
  try {
    cargoVersion = parseCargoPackageVersion(fs.readFileSync(cargoPath, 'utf8'));
  } catch (error) {
    errors.push(`Cargo.toml could not be read: ${error.message}`);
  }
  if (cargoVersion === undefined) {
    errors.push('Cargo.toml [package].version is missing or is not a quoted value.');
  }

  const packageJson = readJsonFile(packagePath, 'package.json', errors);
  const npmVersion = packageJson && typeof packageJson.version === 'string'
    ? packageJson.version
    : undefined;
  if (npmVersion === undefined) {
    errors.push('package.json version is missing or is not a string.');
  }

  const platforms = readJsonFile(platformsPath, 'npm/platforms.json', errors);
  const platformsVersion = platforms && typeof platforms.version === 'string'
    ? platforms.version
    : undefined;
  if (platformsVersion === undefined) {
    errors.push('npm/platforms.json version is missing or is not a string.');
  }

  const metadataVersions = [
    ['Cargo.toml [package].version', cargoVersion],
    ['package.json version', npmVersion],
    ['npm/platforms.json version', platformsVersion],
  ].filter(([, version]) => version !== undefined);

  for (const [label, version] of metadataVersions) {
    if (!EXACT_VERSION.test(version)) {
      errors.push(`${label} must be an exact semantic version; received ${JSON.stringify(version)}.`);
    }
  }

  const canonicalVersion = metadataVersions.length > 0 ? metadataVersions[0][1] : undefined;
  if (canonicalVersion !== undefined) {
    for (const [label, version] of metadataVersions.slice(1)) {
      if (version !== canonicalVersion) {
        errors.push(`${label} (${version}) does not match ${metadataVersions[0][0]} (${canonicalVersion}).`);
      }
    }
  }

  const targets = platforms && Array.isArray(platforms.targets) ? platforms.targets : undefined;
  if (targets === undefined) {
    errors.push('npm/platforms.json targets must be an array.');
  }

  const platformPackages = [];
  if (targets !== undefined) {
    const seenTargets = new Set();
    const seenPackages = new Set();

    targets.forEach((entry, index) => {
      if (!entry || typeof entry !== 'object') {
        errors.push(`npm/platforms.json targets[${index}] must be an object.`);
        return;
      }

      if (typeof entry.target !== 'string' || entry.target.length === 0) {
        errors.push(`npm/platforms.json targets[${index}].target must be a non-empty string.`);
      } else if (seenTargets.has(entry.target)) {
        errors.push(`npm/platforms.json contains duplicate target ${entry.target}.`);
      } else {
        seenTargets.add(entry.target);
      }

      if (typeof entry.package !== 'string' || entry.package.length === 0) {
        errors.push(`npm/platforms.json targets[${index}].package must be a non-empty string.`);
      } else if (seenPackages.has(entry.package)) {
        errors.push(`npm/platforms.json contains duplicate package ${entry.package}.`);
      } else {
        seenPackages.add(entry.package);
        platformPackages.push(entry.package);
      }
    });
  }

  const optionalDependencies = packageJson && packageJson.optionalDependencies;
  if (!optionalDependencies || typeof optionalDependencies !== 'object' || Array.isArray(optionalDependencies)) {
    errors.push('package.json optionalDependencies must be an object.');
  } else {
    const expectedPackages = new Set(platformPackages);
    const actualPackages = new Set(Object.keys(optionalDependencies));
    const missingPackages = platformPackages.filter((name) => !actualPackages.has(name));
    const unexpectedPackages = Object.keys(optionalDependencies).filter((name) => !expectedPackages.has(name));

    if (missingPackages.length > 0 || unexpectedPackages.length > 0) {
      const details = [];
      if (missingPackages.length > 0) {
        details.push(`missing ${missingPackages.join(', ')}`);
      }
      if (unexpectedPackages.length > 0) {
        details.push(`unexpected ${unexpectedPackages.join(', ')}`);
      }
      errors.push(`package.json optionalDependencies package set does not match npm/platforms.json targets (${details.join('; ')}).`);
    }

    const expectedVersion = npmVersion || canonicalVersion;
    if (expectedVersion !== undefined) {
      for (const [name, version] of Object.entries(optionalDependencies)) {
        if (version !== expectedVersion) {
          errors.push(`optionalDependency ${name} must use exact version ${expectedVersion}; received ${JSON.stringify(version)}.`);
        }
      }
    }
  }

  const refName = environment && typeof environment.GITHUB_REF_NAME === 'string'
    ? environment.GITHUB_REF_NAME
    : undefined;
  if (refName && refName.startsWith('v')) {
    if (canonicalVersion === undefined || refName !== `v${canonicalVersion}`) {
      errors.push(`GITHUB_REF_NAME ${refName} does not match v${canonicalVersion || '<version>'}.`);
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    version: canonicalVersion,
  };
}

function main() {
  const result = validateVersionSync();
  if (!result.valid) {
    process.stderr.write(`Version sync check failed:\n${result.errors.map((error) => `- ${error}`).join('\n')}\n`);
    process.exitCode = 1;
    return;
  }

  process.stdout.write(`Version sync check passed for ${result.version}.\n`);
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    process.stderr.write(`Version sync check failed unexpectedly: ${error.message}\n`);
    process.exitCode = 1;
  }
}

module.exports = {
  parseCargoPackageVersion,
  validateVersionSync,
};
