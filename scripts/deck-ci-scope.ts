// deck:ci's changed tier (#604, CI-SPEED.md proposal 5): what a change touches. Files since the last green run map to
// their workspace crates, plus everything that depends on them; a hub crate or a shared file means the whole default tier.
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { WATCH, watched } from './deck-ci-stamps.ts';

export interface Crate {
	name: string;
	/** repo-relative, forward slashes, e.g. deck/core */
	dir: string;
	/** workspace crates it depends on (normal, dev, build) */
	deps: string[];
}

/** crates most of the deck builds on, that the wasm builds and the browser compile, or whose binary live-deck's tests spawn (relay): a change runs everything */
export const HUB = new Set([
	'deck-core', 'deck-catalog', 'protocol', 'deck-space', 'deck-engine', 'live-core', 'deck-outlet', 'deck-ui', 'deck-kit',
	'deck-events', 'deck-module', 'deck-module-host', 'deck-module-macros', 'deck-show', 'deck-engine-wasm', 'space-web', 'relay-web', 'relay'
]);

/** files under the watched tree that are no crate's own: the workspace, the toolchain, the scripts, the shared packages */
const HUB_PATHS = ['deck/Cargo.toml', 'deck/Cargo.lock', 'deck/.cargo', 'deck/rust-toolchain*', 'deck/clippy.toml', 'deck/rustfmt.toml'];

export type Scope =
	| { kind: 'none' }
	| { kind: 'all'; why: string }
	| { kind: 'crates'; touched: string[]; closure: string[] };

/** the crate a file belongs to: the deepest crate dir it lies under */
export function owner(file: string, crates: readonly Crate[]): Crate | undefined {
	let best: Crate | undefined;
	for (const c of crates) if (file.startsWith(`${c.dir}/`) && (!best || c.dir.length > best.dir.length)) best = c;
	return best;
}

/** the touched crates and every workspace crate that depends on them, transitively */
export function closure(touched: Iterable<string>, crates: readonly Crate[]): string[] {
	const users = new Map<string, string[]>();
	for (const c of crates) for (const d of c.deps) users.set(d, [...(users.get(d) ?? []), c.name]);
	const seen = new Set(touched);
	const todo = [...seen];
	while (todo.length) for (const u of users.get(todo.pop()!) ?? []) if (!seen.has(u)) (seen.add(u), todo.push(u));
	return [...seen].sort();
}

export function scope(files: readonly string[], crates: readonly Crate[]): Scope {
	const mine = files.filter((f) => watched(f, WATCH));
	if (!mine.length) return { kind: 'none' };
	const touched = new Set<string>();
	for (const f of mine) {
		if (watched(f, HUB_PATHS)) return { kind: 'all', why: f };
		const c = f.startsWith('deck/') ? owner(f, crates) : undefined;
		if (!c) return { kind: 'all', why: f };
		if (HUB.has(c.name)) return { kind: 'all', why: `${f} (${c.name})` };
		touched.add(c.name);
	}
	const all = closure(touched, crates);
	const hub = all.find((n) => HUB.has(n));
	if (hub) return { kind: 'all', why: `${hub} depends on ${[...touched].join(', ')}` };
	return { kind: 'crates', touched: [...touched].sort(), closure: all };
}

/** the workspace's crates from cargo metadata */
export function workspace(deckDir: string): Crate[] {
	const m = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--offline'], { cwd: deckDir, encoding: 'utf8', maxBuffer: 256 << 20 }));
	const members = new Set<string>(m.workspace_members);
	const names = new Set<string>(m.packages.filter((p: { id: string }) => members.has(p.id)).map((p: { name: string }) => p.name));
	const root = String(m.workspace_root).replaceAll('\\', '/');
	return m.packages
		.filter((p: { id: string }) => members.has(p.id))
		.map((p: { name: string; manifest_path: string; dependencies: { name: string }[] }) => ({
			name: p.name,
			dir: `deck/${p.manifest_path.replaceAll('\\', '/').slice(root.length + 1).replace(/\/?Cargo\.toml$/, '')}`.replace(/\/$/, ''),
			deps: [...new Set(p.dependencies.map((d) => d.name).filter((n) => names.has(n)))]
		}));
}

const LAST = 'last-green.json';

/** the commit of the last green deck:ci run (any tier but quick), when it is an ancestor of HEAD */
export function lastGreen(dir: string, root: string): string | undefined {
	try {
		const { head } = JSON.parse(readFileSync(join(dir, LAST), 'utf8')) as { head: string };
		execFileSync('git', ['merge-base', '--is-ancestor', head, 'HEAD'], { cwd: root, stdio: 'ignore' });
		return head;
	} catch {
		return undefined;
	}
}

export function markGreen(dir: string, head: string, tier: string): void {
	mkdirSync(dir, { recursive: true });
	writeFileSync(join(dir, LAST), JSON.stringify({ head, tier, date: new Date().toISOString() }));
}

/** the files changed from `base` to HEAD, plus the uncommitted ones */
export function changedSince(root: string, base: string, dirty: readonly string[]): string[] {
	const out = execFileSync('git', ['diff', '--name-only', `${base}..HEAD`], { cwd: root, encoding: 'utf8' });
	return [...new Set([...out.split('\n').filter(Boolean), ...dirty])];
}
