// What is live where: each target's `status` URL (deploy/targets.json) answers with the commit it was built from
// (`git_sha` in version.json / the API's /health/). Shared by deploy/changed.mjs (what to deploy) and
// deploy/status.mjs (npm run deploy:status).
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';

export const config = JSON.parse(readFileSync(new URL('./targets.json', import.meta.url), 'utf8'));

/** Deployable targets in deploy order (held-back ones like embassy are not in `order`). */
export const ordered = () => config.order.map((name) => ({ name, ...config.targets[name] }));

/** The commit a target is serving, or null (not reachable, no git_sha). */
export async function liveSha(target) {
	if (!target.status) return null;
	try {
		const res = await fetch(target.status, { headers: { 'Cache-Control': 'no-cache' }, signal: AbortSignal.timeout(15000) });
		if (!res.ok) return null;
		const json = await res.json();
		return typeof json.git_sha === 'string' && /^[0-9a-f]{7,40}$/.test(json.git_sha) ? json.git_sha : null;
	} catch {
		return null;
	}
}

export const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim();

/** Is the commit in this clone (a full fetch is needed for diffs against older deploys)? */
export function knownCommit(sha) {
	try {
		git('cat-file', '-e', `${sha}^{commit}`);
		return true;
	} catch {
		return false;
	}
}

// ── Which changed files matter to a target (issue 73: only what reaches its build) ──────────────────────────────────
// A target's watch list = its `watch` patterns + `shared/<pkg>/**` for every @scutoons package its `watchPackages` dirs
// depend on (package.json dependencies / devDependencies / peerDependencies, transitively) – derived, never listed by
// hand. Then the global `watchIgnore` drops what never ships (tests, docs, seed scripts) unless `watchKeep` holds it
// (markdown under a src/ dir is imported, e.g. admin's ?raw templates). deploy/targets.json itself counts for a target
// only when its own block (or a shared top-level key like `protocols`) changed. When in doubt: deploy.

export const TARGETS_FILE = 'deploy/targets.json';

const readJson = (path) => {
	try {
		return JSON.parse(readFileSync(new URL(`../${path}`, import.meta.url), 'utf8'));
	} catch {
		return null;
	}
};
const sharedDirs = () =>
	readdirSync(new URL('../shared/', import.meta.url), { withFileTypes: true })
		.filter((d) => d.isDirectory())
		.map((d) => `shared/${d.name}`);

// a glob ("dir/**", "**" + "/x/**", "*.md" …) → RegExp over a repo path: `**` any depth, `*` within one segment
export function globToRegExp(glob) {
	let re = '';
	for (let i = 0; i < glob.length; i++) {
		const c = glob[i];
		if (c === '*' && glob[i + 1] === '*') {
			if (glob[i + 2] === '/') (re += '(?:.*/)?'), (i += 2);
			else (re += '.*'), i++;
		} else if (c === '*') re += '[^/]*';
		else re += c.replace(/[.+?^${}()|[\]\\]/g, '\\$&');
	}
	return new RegExp(`^${re}$`);
}
const regexps = new Map();
const matches = (glob, path) => {
	if (!regexps.has(glob)) regexps.set(glob, globToRegExp(glob));
	return regexps.get(glob).test(path);
};

/**
 * The shared package dirs ("shared/ui" …) the given package dirs depend on, transitively. A package.json that can't be
 * read or an @scutoons package not found under shared/ → every shared dir (err on the side of deploying).
 */
export function packageClosure(roots, readPkg = readJson, shared = sharedDirs()) {
	const byName = new Map();
	for (const dir of shared) {
		const name = readPkg(`${dir}/package.json`)?.name;
		if (name) byName.set(name, dir);
	}
	const found = new Set();
	const todo = [...roots];
	while (todo.length) {
		const pkg = readPkg(`${todo.pop()}/package.json`);
		if (!pkg) return [...shared].sort();
		for (const name of Object.keys({ ...pkg.dependencies, ...pkg.devDependencies, ...pkg.peerDependencies })) {
			if (!name.startsWith('@scutoons/')) continue;
			const dir = byName.get(name);
			if (!dir) return [...shared].sort();
			if (!found.has(dir)) found.add(dir), todo.push(dir);
		}
	}
	return [...found].sort();
}

/** The target's full watch list: its `watch` + `shared/<pkg>/**` of its `watchPackages` closure. */
export function watchPatterns(target, readPkg = readJson, shared) {
	const derived = target.watchPackages?.length ? packageClosure(target.watchPackages, readPkg, shared).map((d) => `${d}/**`) : [];
	return [...(target.watch ?? []), ...derived];
}

/** Does a changed path matter to the target: under its watch list ("!…" excludes), not dropped by watchIgnore? */
export function watched(target, path, { cfg = config, patterns = watchPatterns(target) } = {}) {
	if (!patterns.some((p) => !p.startsWith('!') && matches(p, path))) return false;
	if (patterns.some((p) => p.startsWith('!') && matches(p.slice(1), path))) return false;
	return !((cfg.watchIgnore ?? []).some((p) => matches(p, path)) && !(cfg.watchKeep ?? []).some((p) => matches(p, path)));
}

/** Did the target's part of deploy/targets.json change: its block or a top-level key other than `targets` ("$…" keys don't count)? Unreadable old text → yes. */
export function targetConfigChanged(name, oldText, newCfg = config) {
	let old;
	try {
		old = JSON.parse(oldText);
	} catch {
		return true;
	}
	const plain = (value) => JSON.stringify(value ?? null, (key, v) => (key.startsWith('$') ? undefined : v));
	const top = (cfg) => plain(Object.fromEntries(Object.entries(cfg).filter(([key]) => key !== 'targets')));
	return plain(old?.targets?.[name]) !== plain(newCfg.targets[name]) || top(old ?? {}) !== top(newCfg);
}

/**
 * The changed files that need a redeploy of the target. `oldConfig` = deploy/targets.json as it was at the live commit
 * (text; null = unknown → counts as changed).
 */
export function relevantFiles(target, files, { oldConfig = null, cfg = config, patterns = watchPatterns(target) } = {}) {
	return files.filter((file) => (file === TARGETS_FILE ? targetConfigChanged(target.name, oldConfig, cfg) : watched(target, file, { cfg, patterns })));
}

/** deploy/targets.json at a commit (text), or null. */
export function configAt(sha) {
	try {
		return git('show', `${sha}:${TARGETS_FILE}`);
	} catch {
		return null;
	}
}
