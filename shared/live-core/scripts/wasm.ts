// Where live_core.wasm is built (P6, #320): wasm32 + simd128 (plan 3.5 – every target browser has it, and without it the
// engine's voice batches are slower than its scalar blocks) in a target folder of its own, so the flag does not rebuild
// the workspace's main target. The deck's target folder is wherever cargo puts it (#526): deckTarget().
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const deckDir = fileURLToPath(new URL('../../../deck/', import.meta.url));

let asked: string | undefined;
/** the deck workspace's target folder (#526): CARGO_TARGET_DIR, else cargo's own answer (`cargo metadata`: a worktree's
 * builds live in `<git common dir>/deck-target/<worktree>`, scripts/deck-target.ts), else deck/target. Every script and
 * test that reads a built file asks here – never `deck/target` */
export function deckTarget(env: Record<string, string | undefined> = process.env): string {
	if (env.CARGO_TARGET_DIR) return resolve(deckDir, env.CARGO_TARGET_DIR);
	if (asked === undefined) {
		try {
			const meta = execFileSync('cargo', ['metadata', '--no-deps', '--format-version', '1'], { cwd: deckDir, encoding: 'utf8', maxBuffer: 64 << 20, stdio: ['ignore', 'pipe', 'ignore'] });
			asked = (JSON.parse(meta) as { target_directory: string }).target_directory;
		} catch {
			asked = `${deckDir}target`;
		}
	}
	return asked;
}

/** the target folder live-core builds into */
export const targetDir = (env: Record<string, string | undefined> = process.env): string => `${deckTarget(env)}/live-core-simd`;

/** the built module */
export const wasmPath = (env: Record<string, string | undefined> = process.env): string => `${targetDir(env)}/wasm32-unknown-unknown/release/live_core.wasm`;

/** the cargo call and its environment */
export const buildCommand = (env: Record<string, string | undefined> = process.env) => ({
	cmd: 'cargo',
	args: ['build', '--release', '--target', 'wasm32-unknown-unknown', '-p', 'live-core', '--target-dir', targetDir(env)],
	env: { ...env, RUSTFLAGS: '-C target-feature=+simd128' }
});
