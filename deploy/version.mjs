#!/usr/bin/env node
/**
 * Writes version.json – { app, version, git_sha, built_at, shared } – into an app's build output
 * (agents/PLATFORM.md "Versions / out-of-sync detection"). `shared` = the @scutoons/* packages the app bundles
 * (direct and transitive dependencies) with their versions.
 *
 *   node ../deploy/version.mjs [outDir]          from an app folder (npm run build does this; outDir default: build)
 *   node deploy/version.mjs --app-dir=qr --out=<dir>   for the PHP apps (composer.json "version"; deploy.mjs)
 *
 * git sha: GITHUB_SHA (CI) or `git rev-parse HEAD`, "dev" when neither is available.
 */
import { execSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');

const readJson = (file) => JSON.parse(readFileSync(file, 'utf8'));

/** name → { version, dependencies } of every shared package (shared/*) */
function sharedPackages() {
	const packages = {};
	const dir = join(repoRoot, 'shared');
	for (const entry of readdirSync(dir, { withFileTypes: true })) {
		const file = join(dir, entry.name, 'package.json');
		if (!entry.isDirectory() || !existsSync(file)) continue;
		const pkg = readJson(file);
		packages[pkg.name] = { version: pkg.version, dependencies: { ...pkg.dependencies, ...pkg.peerDependencies } };
	}
	return packages;
}

/** @scutoons/* packages reachable from these dependencies, sorted by name */
export function bundledShared(dependencies = {}) {
	const all = sharedPackages();
	const found = {};
	const visit = (deps) => {
		for (const name of Object.keys(deps ?? {})) {
			if (!all[name] || found[name]) continue;
			found[name] = all[name].version;
			visit(all[name].dependencies);
		}
	};
	visit(dependencies);
	return Object.fromEntries(Object.entries(found).sort(([a], [b]) => a.localeCompare(b)));
}

export function gitSha() {
	if (process.env.GITHUB_SHA) return process.env.GITHUB_SHA;
	try {
		return execSync('git rev-parse HEAD', { cwd: repoRoot, stdio: ['ignore', 'pipe', 'ignore'] }).toString().trim();
	} catch {
		return 'dev';
	}
}

/** version.json of the app in appDir (package.json or composer.json) */
export function versionInfo(appDir, appName) {
	const pkgFile = join(appDir, 'package.json');
	const composerFile = join(appDir, 'composer.json');
	let version = '0.0.0';
	let shared = {};
	if (existsSync(pkgFile)) {
		const pkg = readJson(pkgFile);
		version = pkg.version ?? version;
		shared = bundledShared({ ...pkg.dependencies, ...pkg.devDependencies });
	} else if (existsSync(composerFile)) {
		version = readJson(composerFile).version ?? version;
	}
	return {
		app: appName ?? basename(appDir),
		version,
		git_sha: gitSha(),
		built_at: new Date().toISOString(),
		shared
	};
}

export function writeVersion(appDir, outDir, appName) {
	const info = versionInfo(appDir, appName);
	mkdirSync(outDir, { recursive: true });
	writeFileSync(join(outDir, 'version.json'), JSON.stringify(info, null, 2) + '\n');
	return info;
}

// CLI
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const args = process.argv.slice(2);
	const opt = (name) => args.find((a) => a.startsWith(`--${name}=`))?.slice(name.length + 3);
	const appDir = resolve(opt('app-dir') ?? process.cwd());
	const outDir = resolve(appDir, opt('out') ?? args.find((a) => !a.startsWith('--')) ?? 'build');
	const info = writeVersion(appDir, outDir, opt('app'));
	console.log(`version.json → ${outDir}: ${info.app} ${info.version} (${info.git_sha.slice(0, 12)})`);
}
