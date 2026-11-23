// deck:ci's green stamps (#524, agents/research/CI-SPEED.md proposal 1): after a green step, scripts/deck-ci.mjs stores a
// stamp keyed by the CONTENT the step saw – the blob hashes of every watched path at HEAD, the step, the tier and the
// toolchain – in the repo's common git dir, so every worktree, the orchestrator's run and the pre-push hook see it. A later
// run with the same key skips that step and names the stamp's date. Never stamped and never skipped: a step whose
// watched paths have uncommitted or untracked changes (then the working tree is not what HEAD says), and every step when
// any WATCH path is dirty. `--fresh` / DECK_CI_FRESH=1 ignores the stamps (and writes new ones).
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, unlinkSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

export type Tier = 'quick' | 'default' | 'full';

/** what deck:ci watches: a change here runs it from the pre-push hook (`--if-changed`) and is part of every step's key.
 * A path is a file or a folder; a trailing `*` matches by prefix. */
export const WATCH: readonly string[] = [
	'deck',
	'shared/live',
	'shared/live-core',
	'shared/live-deck',
	'.github/workflows/deck.yml',
	'scripts/deck-ci.mjs',
	'scripts/deck-ci-stamps.*',
	'scripts/git-hooks/pre-push',
	'scripts/check-deck-deps.*',
	'scripts/deck-deps.*',
	'scripts/native-logic.*',
	// what deck's tests and builds read outside deck/ (include_str!, fixtures): a change there is a change of their result
	'server/tests/fixtures',
	'server/schema/tables/locale-order.vectors.json',
	'server/schema/api/spore-network.vectors.json',
	'server/src/Domain/Modules/ModuleSource.php',
	'shared/content/test/fixtures',
	'creator/static/SCUTOONS-logo-animated.svg'
];

/** a repo-relative path (forward slashes) under one of `paths` */
export function watched(file: string, paths: readonly string[]): boolean {
	return paths.some((p) => (p.endsWith('*') ? file.startsWith(p.slice(0, -1)) : file === p || file.startsWith(`${p}/`)));
}

/** `git ls-tree -r -z --full-tree HEAD` → path → "mode sha" */
export function parseLsTree(z: string): Map<string, string> {
	const files = new Map<string, string>();
	for (const entry of z.split('\0')) {
		const tab = entry.indexOf('\t');
		if (tab < 0) continue;
		const [mode, , sha] = entry.slice(0, tab).split(' ');
		files.set(entry.slice(tab + 1), `${mode} ${sha}`);
	}
	return files;
}

/** `git status --porcelain=v1 -z --untracked-files=all` → every path that differs from HEAD (both sides of a rename) */
export function parseStatus(z: string): string[] {
	const parts = z.split('\0');
	const dirty: string[] = [];
	for (let i = 0; i < parts.length; i++) {
		const e = parts[i];
		if (e.length < 4) continue;
		dirty.push(e.slice(3));
		// a rename / copy carries its source as the next entry
		if (e[0] === 'R' || e[0] === 'C') dirty.push(parts[++i] ?? '');
	}
	return dirty.filter(Boolean);
}

export interface KeyInput {
	step: string;
	tier: Tier;
	/** `rustc -V`, `node -v`, RUSTFLAGS – what builds the same tree differently */
	toolchain: string;
	/** HEAD's files (parseLsTree) */
	tree: Map<string, string>;
	/** the step's watched paths */
	paths: readonly string[];
}

/** the content key of one step: equal only when every watched file at HEAD, the step, the tier and the toolchain are */
export function contentKey({ step, tier, toolchain, tree, paths }: KeyInput): string {
	const h = createHash('sha256');
	h.update(`deck-ci stamp v1\n${step}\n${tier}\n${toolchain}\n`);
	for (const f of [...tree.keys()].filter((f) => watched(f, paths)).sort()) h.update(`${tree.get(f)} ${f}\n`);
	return h.digest('hex');
}

export interface Stamp {
	step: string;
	tier: Tier;
	/** ISO date of the green run */
	date: string;
	seconds: number;
	head: string;
	toolchain: string;
}

export type Plan = { action: 'skip'; stamp: Stamp } | { action: 'run'; stamp: boolean; why: string };

export interface DecideInput {
	/** parseStatus of the working tree */
	dirty: readonly string[];
	/** the step's watched paths (WATCH + its own) */
	paths: readonly string[];
	/** false for a step whose result depends on more than the tree (the core-only check reads main's baseline) */
	stampable: boolean;
	fresh: boolean;
	/** the stamp under the step's key, when there is one */
	found?: Stamp;
}

/** run or skip one step, and whether a green run may be stamped */
export function decide({ dirty, paths, stampable, fresh, found }: DecideInput): Plan {
	if (!stampable) return { action: 'run', stamp: false, why: 'never stamped' };
	const changed = dirty.find((f) => watched(f, WATCH)) ?? dirty.find((f) => watched(f, paths));
	if (changed) return { action: 'run', stamp: false, why: `uncommitted change (${changed}) – no stamps` };
	if (fresh) return { action: 'run', stamp: true, why: 'fresh' };
	if (found) return { action: 'skip', stamp: found };
	return { action: 'run', stamp: true, why: 'no stamp for this tree' };
}

/** the stamps of one repo: one JSON file per key in a folder every worktree sees */
export class Stamps {
	constructor(readonly dir: string) {}

	find(key: string): Stamp | undefined {
		try {
			return JSON.parse(readFileSync(join(this.dir, `${key}.json`), 'utf8')) as Stamp;
		} catch {
			return undefined;
		}
	}

	write(key: string, stamp: Stamp): void {
		mkdirSync(this.dir, { recursive: true });
		writeFileSync(join(this.dir, `${key}.json`), `${JSON.stringify(stamp, null, '\t')}\n`);
	}

	/** drop stamps older than `days` (a tree that old is not run again) */
	prune(days = 30, now = Date.now()): number {
		if (!existsSync(this.dir)) return 0;
		let n = 0;
		for (const f of readdirSync(this.dir)) {
			const p = join(this.dir, f);
			if (f.endsWith('.json') && now - statSync(p).mtimeMs > days * 864e5) {
				unlinkSync(p);
				n++;
			}
		}
		return n;
	}
}

const git = (root: string, args: string[]) => execFileSync('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 64 << 20 });

/** HEAD, its files and the working tree's changes */
export function gitState(root: string): { head: string; tree: Map<string, string>; dirty: string[] } {
	return {
		head: git(root, ['rev-parse', 'HEAD']).trim(),
		tree: parseLsTree(git(root, ['ls-tree', '-r', '-z', '--full-tree', 'HEAD'])),
		dirty: parseStatus(git(root, ['status', '--porcelain=v1', '-z', '--untracked-files=all']))
	};
}

/** the stamps folder in the repo's common git dir (shared by every worktree, never in the working tree) */
export function stampsDir(root: string): string {
	return join(resolve(root, git(root, ['rev-parse', '--git-common-dir']).trim()), 'deck-ci-stamps');
}

/** what builds the same tree differently */
export function toolchain(deckDir: string): string {
	let rustc = 'rustc ?';
	try {
		rustc = execFileSync('rustc', ['-V'], { cwd: deckDir, encoding: 'utf8' }).trim();
	} catch {
		// no rustc: the cargo steps fail on their own
	}
	return `${rustc} · node ${process.version} · RUSTFLAGS=${process.env.RUSTFLAGS ?? ''}`;
}
