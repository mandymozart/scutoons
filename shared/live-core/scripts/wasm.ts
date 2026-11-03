// Where live_core.wasm is built (P6, #320): wasm32 + simd128 (plan 3.5 – every target browser has it, and without it the
// engine's voice batches are slower than its scalar blocks) in a target folder of its own, so the flag does not rebuild
// the workspace's main target. CARGO_TARGET_DIR is honoured (default deck/target).
import { fileURLToPath } from 'node:url';

export const deckDir = fileURLToPath(new URL('../../../deck/', import.meta.url));

/** the target folder live-core builds into */
export const targetDir = (env: Record<string, string | undefined> = process.env): string => `${env.CARGO_TARGET_DIR ?? `${deckDir}target`}/live-core-simd`;

/** the built module */
export const wasmPath = (env: Record<string, string | undefined> = process.env): string => `${targetDir(env)}/wasm32-unknown-unknown/release/live_core.wasm`;

/** the cargo call and its environment */
export const buildCommand = (env: Record<string, string | undefined> = process.env) => ({
	cmd: 'cargo',
	args: ['build', '--release', '--target', 'wasm32-unknown-unknown', '-p', 'live-core', '--target-dir', targetDir(env)],
	env: { ...env, RUSTFLAGS: '-C target-feature=+simd128' }
});
