// The deck's build output in one place (#526, agents/research/CI-SPEED.md "Design (#526)"): every worktree's cargo target
// dir lives in `<git common dir>/deck-target/<worktree id>/`, the registry crates' compiles in `…/deck-target/sccache/`
// (sccache, when installed). One target dir shared by all worktrees is wrong, not only slow: cargo keys a workspace crate by
// its path relative to the workspace, so a second worktree reuses the first one's artifacts by mtime and tests its code.
//
//   npm run deck:target                         sizes: the root per worktree (this / alive / stale), sccache, the old
//                                               deck/target of every worktree, the main checkout's deck/target, free disk
//   npm run deck:target -- --clear              delete the stale worktree dirs (their worktree is gone)
//   npm run deck:target -- --clear --all        delete the whole root: every worktree's build + sccache (no session building)
//   npm run deck:target -- --clear --legacy     also delete the worktrees' old deck/target (before #526)
//   npm run deck:target -- --clear --main       also delete the main checkout's deck/target – only this flag touches it
//   npm run deck:target -- --ensure             write this worktree's .cargo/config.toml (npm's postinstall, deck:ci);
//                                               nothing in the main checkout or a plain clone; never fails
//
// How a worktree finds it: the untracked, git-ignored `<worktree>/.cargo/config.toml` written here (build.target-dir,
// build.rustc-wrapper = sccache, [env] SCCACHE_DIR) – cargo reads it above deck/.cargo/config.toml; CARGO_TARGET_DIR
// still overrides. Scripts and tests ask cargo where the target is (deckTarget() in shared/live-core/scripts/wasm.ts,
// env!("CARGO_TARGET_TMPDIR") in the Rust tests), never `deck/target`.
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statfsSync, writeFileSync } from 'node:fs';
import { lstat, readdir } from 'node:fs/promises';
import { basename, delimiter, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** the root's folder name in the common git dir, and sccache's folder inside it */
export const ROOT = 'deck-target';
export const SCCACHE = 'sccache';
/** sccache's cap (LRU beyond it) and how long its server stays after the last compile (no stray server) */
export const SCCACHE_SIZE = '20G';
export const SCCACHE_IDLE_SECONDS = 300;
const HEADER = '# Written by scripts/deck-target.ts (#526)';

const slash = (p: string) => p.replaceAll('\\', '/');
const same = (a: string, b: string) => slash(resolve(a)).toLowerCase() === slash(resolve(b)).toLowerCase();

/** a linked worktree's id (`<common>/worktrees/<id>`); null for the main checkout and a plain clone (git dir = common dir) */
export function worktreeId(gitDir: string, commonDir: string): string | null {
	return same(gitDir, commonDir) ? null : basename(resolve(gitDir));
}

/** a TOML basic string (paths with forward slashes) */
const str = (s: string) => JSON.stringify(slash(s));

/** the worktree's `.cargo/config.toml` */
export function configToml({ target, wrapper, sccacheDir }: { target: string; wrapper: string | null; sccacheDir: string }): string {
	const lines = [
		`${HEADER}: this worktree's deck builds go to the one shared location`,
		'# (agents/research/CI-SPEED.md "Design (#526)"). Untracked; rewritten by npm install and deck:ci. Sizes / clear: npm run deck:target',
		'[build]',
		`target-dir = ${str(target)}`
	];
	if (wrapper) lines.push(`rustc-wrapper = ${str(wrapper)}`, '', '[env]', `SCCACHE_DIR = ${str(sccacheDir)}`, `SCCACHE_CACHE_SIZE = "${SCCACHE_SIZE}"`, `SCCACHE_IDLE_TIMEOUT = "${SCCACHE_IDLE_SECONDS}"`);
	return `${lines.join('\n')}\n`;
}

/** sccache on PATH or in cargo's bin folder, else null (then builds run without it) */
export function findSccache(env: Record<string, string | undefined> = process.env): string | null {
	const exe = process.platform === 'win32' ? 'sccache.exe' : 'sccache';
	const home = env.CARGO_HOME ?? join(env.USERPROFILE ?? env.HOME ?? '', '.cargo');
	const dirs = [...(env.PATH ?? env.Path ?? '').split(delimiter), join(home, 'bin')].filter(Boolean);
	for (const d of dirs) if (existsSync(join(d, exe))) return join(d, exe);
	return null;
}

export type Kind = 'this' | 'alive' | 'stale' | 'sccache';
export interface Entry {
	name: string;
	path: string;
	kind: Kind;
	bytes: number;
}

/** what a folder in the root is */
export function kindOf(name: string, current: string | null, alive: ReadonlySet<string>): Kind {
	if (name === SCCACHE) return 'sccache';
	if (name === current) return 'this';
	return alive.has(name) ? 'alive' : 'stale';
}

export interface ClearFlags {
	all: boolean;
	legacy: boolean;
	main: boolean;
}

/** the folders a clear deletes: stale worktree dirs; `all` the whole root; `legacy` the worktrees' old deck/target; `main` the main checkout's */
export function toClear(entries: readonly Entry[], flags: ClearFlags, legacy: readonly string[], mainTarget: string | null): string[] {
	const out = entries.filter((e) => flags.all || e.kind === 'stale').map((e) => e.path);
	if (flags.legacy) out.push(...legacy);
	if (flags.main && mainTarget) out.push(mainTarget);
	return out;
}

export function gb(bytes: number): string {
	return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

const git = (cwd: string, args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();

export interface Layout {
	/** the worktree (or checkout) this runs in */
	top: string;
	commonDir: string;
	root: string;
	/** null: the main checkout or a plain clone */
	id: string | null;
}

export function layout(cwd: string): Layout {
	const top = git(cwd, ['rev-parse', '--show-toplevel']);
	const commonDir = resolve(top, git(cwd, ['rev-parse', '--git-common-dir']));
	const id = worktreeId(resolve(top, git(cwd, ['rev-parse', '--git-dir'])), commonDir);
	return { top, commonDir, root: join(commonDir, ROOT), id };
}

/** write this worktree's .cargo/config.toml (if it changed); a config not written here is left alone */
export function ensure(l: Layout, env = process.env): string {
	if (!l.id) return 'main checkout or plain clone: builds stay in deck/target';
	const file = join(l.top, '.cargo', 'config.toml');
	const want = configToml({ target: join(l.root, l.id), wrapper: findSccache(env), sccacheDir: join(l.root, SCCACHE) });
	const have = existsSync(file) ? readFileSync(file, 'utf8') : null;
	if (have !== null && !have.startsWith(HEADER)) return `${slash(file)} was not written by deck-target.ts – left alone, builds go where it says`;
	if (have !== want) {
		mkdirSync(dirname(file), { recursive: true });
		writeFileSync(file, want);
	}
	return `deck builds → ${slash(join(l.root, l.id))}${want.includes('rustc-wrapper') ? ' (sccache)' : ' (no sccache installed)'}`;
}

/** a folder's size: every file once, links not followed, unreadable parts skipped */
export async function sizeOf(dir: string): Promise<number> {
	let total = 0;
	const walk = async (d: string): Promise<void> => {
		let items;
		try {
			items = await readdir(d, { withFileTypes: true });
		} catch {
			return;
		}
		const subs: Promise<void>[] = [];
		for (const it of items) {
			const p = join(d, it.name);
			if (it.isDirectory()) subs.push(walk(p));
			else if (it.isFile()) subs.push(lstat(p).then((s) => void (total += s.size), () => undefined));
		}
		await Promise.all(subs);
	};
	await walk(dir);
	return total;
}

/** the worktrees git knows: the main checkout first */
function worktrees(cwd: string): string[] {
	return git(cwd, ['worktree', 'list', '--porcelain'])
		.split('\n')
		.filter((l) => l.startsWith('worktree '))
		.map((l) => l.slice('worktree '.length));
}

async function main(args: string[]): Promise<void> {
	const here = dirname(fileURLToPath(import.meta.url));
	if (args.includes('--ensure')) {
		// npm's postinstall and deck:ci: never fail an install over this
		try {
			console.log(`deck-target: ${ensure(layout(here))}`);
		} catch (e) {
			console.log(`deck-target: not set up (${(e as Error).message.split('\n')[0]})`);
		}
		return;
	}
	const l = layout(here);
	const clear = args.includes('--clear');
	const flags: ClearFlags = { all: args.includes('--all'), legacy: args.includes('--legacy'), main: args.includes('--main') };
	if (!clear && (flags.all || flags.legacy || flags.main)) throw new Error('--all, --legacy and --main go with --clear');

	const alive = new Set(existsSync(join(l.commonDir, 'worktrees')) ? readdirSync(join(l.commonDir, 'worktrees')) : []);
	const names = existsSync(l.root) ? readdirSync(l.root, { withFileTypes: true }).filter((d) => d.isDirectory()).map((d) => d.name) : [];
	const entries: Entry[] = await Promise.all(
		names.map(async (name) => ({ name, path: join(l.root, name), kind: kindOf(name, l.id, alive), bytes: await sizeOf(join(l.root, name)) }))
	);
	const [mainTop, ...linked] = worktrees(here);
	const legacy = await Promise.all(
		linked.map((w) => join(w, 'deck', 'target')).filter((p) => existsSync(p)).map(async (path) => ({ path, bytes: await sizeOf(path) }))
	);
	const mainTarget = join(mainTop, 'deck', 'target');
	const mainBytes = existsSync(mainTarget) ? await sizeOf(mainTarget) : null;

	const note: Record<Kind, string> = {
		this: 'this worktree',
		alive: 'worktree alive',
		stale: 'stale: its worktree is gone (--clear)',
		sccache: 'the registry crates’ compile cache (--clear --all)'
	};
	const total = entries.reduce((s, e) => s + e.bytes, 0);
	console.log(`deck builds in ${slash(l.root)}: ${gb(total)}${l.id ? '' : ' (this is the main checkout: its own builds stay in deck/target)'}`);
	for (const e of entries.sort((a, b) => b.bytes - a.bytes)) console.log(`  ${e.name.padEnd(28)} ${gb(e.bytes).padStart(9)}  ${note[e.kind]}`);
	if (legacy.length) {
		console.log(`old deck/target of the worktrees (before #526; --clear --legacy): ${gb(legacy.reduce((s, x) => s + x.bytes, 0))}`);
		for (const x of legacy.sort((a, b) => b.bytes - a.bytes)) console.log(`  ${slash(x.path).padEnd(70)} ${gb(x.bytes).padStart(9)}`);
	}
	if (mainBytes !== null) console.log(`main checkout ${slash(mainTarget)}: ${gb(mainBytes)} – the owner's builds, cleared only by --clear --main`);
	try {
		const fs = statfsSync(l.root.slice(0, 3));
		console.log(`free on the drive: ${gb(fs.bavail * fs.bsize)}`);
	} catch {
		// no statfs: no line
	}
	if (!clear) return;

	const paths = toClear(entries, flags, legacy.map((x) => x.path), mainBytes === null ? null : mainTarget);
	if (paths.some((p) => p.endsWith(SCCACHE))) {
		const sccache = findSccache();
		if (sccache) {
			try {
				execFileSync(sccache, ['--stop-server'], { stdio: 'ignore' });
			} catch {
				// not running
			}
		}
	}
	let freed = 0;
	for (const p of paths) {
		const bytes = entries.find((e) => e.path === p)?.bytes ?? legacy.find((x) => x.path === p)?.bytes ?? (p === mainTarget ? (mainBytes ?? 0) : 0);
		try {
			rmSync(p, { recursive: true, force: true, maxRetries: 2 });
			freed += bytes;
			console.log(`deleted ${slash(p)} (${gb(bytes)})`);
		} catch (e) {
			console.log(`could not delete all of ${slash(p)} (a build running there?): ${(e as Error).message.split('\n')[0]}`);
		}
	}
	console.log(paths.length ? `cleared ${gb(freed)}` : 'nothing to clear');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	main(process.argv.slice(2)).catch((e) => {
		console.error(`deck:target – ${(e as Error).message}`);
		process.exit(1);
	});
}
