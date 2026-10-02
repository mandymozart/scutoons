// What is live where: each target's `status` URL (deploy/targets.json) answers with the commit it was built from
// (`git_sha` in version.json / the API's /health/). Shared by deploy/changed.mjs (what to deploy) and
// deploy/status.mjs (npm run deploy:status).
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

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

/** Does a changed path fall under the target's `watch` patterns ("dir/**" or an exact file; "!…" excludes)? */
export function watched(target, path) {
	const match = (pattern) => (pattern.endsWith('/**') ? path.startsWith(pattern.slice(0, -2)) : path === pattern);
	const patterns = target.watch ?? [];
	return patterns.some((p) => !p.startsWith('!') && match(p)) && !patterns.some((p) => p.startsWith('!') && match(p.slice(1)));
}
