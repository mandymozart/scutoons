// deck-core's history rules for the MCP (#687): the same wasm module the browser deck loads (space-web's `history_*`, no
// GPU), run in Node from files on this machine only: SCUTOONS_CORE_DIR, or a checkout's shared/live-deck/wasm (build:wasm).
// Nothing is downloaded and run. Nothing of the rules is here.
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { historyView, type HistoryExports, type HistoryView } from '@scutoons/content';
import { BUNDLED, ROOT } from './config.ts';

interface CoreModule extends HistoryExports {
	default(init: { module_or_path: Uint8Array }): Promise<unknown>;
}

/** no core files on this machine */
export class CoreMissing extends Error {
	constructor() {
		super('the core is not on this machine');
	}
}

const JS = 'space_web.js';
const WASM = 'space_web_bg.wasm';

/** where the core's files are: a folder holding both, or null */
export function coreDir(env: NodeJS.ProcessEnv = process.env, bundled = BUNDLED): string | null {
	const candidates = [env.SCUTOONS_CORE_DIR, bundled ? undefined : resolve(ROOT, 'shared/live-deck/wasm')];
	return candidates.find((d): d is string => !!d && existsSync(resolve(d, JS)) && existsSync(resolve(d, WASM))) ?? null;
}

/** the core's history rules, loaded once (on the first revert) */
export function coreHistory(): () => Promise<HistoryView> {
	let core: Promise<HistoryView> | null = null;
	return () =>
		(core ??= (async () => {
			const dir = coreDir();
			if (!dir) throw new CoreMissing();
			const mod = (await import(pathToFileURL(resolve(dir, JS)).href)) as CoreModule;
			await mod.default({ module_or_path: readFileSync(resolve(dir, WASM)) });
			return historyView(mod);
		})().catch((e) => {
			core = null;
			throw e;
		}));
}
