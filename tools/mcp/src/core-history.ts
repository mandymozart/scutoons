// deck-core's history rules for the MCP (#687): the same wasm module the browser deck loads (space-web's `history_*`, no
// GPU), run in Node. The files are looked up in this order: SCUTOONS_CORE_DIR, a checkout's shared/live-deck/wasm (build:wasm),
// else the viewer's /live/ (what the deck page itself serves) saved once into a cache folder. Nothing of the rules is here.
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { historyView, type HistoryExports, type HistoryView } from '@scutoons/content';
import { BUNDLED, ROOT } from './config.ts';

interface CoreModule extends HistoryExports {
	default(init: { module_or_path: Uint8Array }): Promise<unknown>;
}

const JS = 'space_web.js';
const WASM = 'space_web_bg.wasm';

/** where the core's files are: a folder holding both, or null */
export function coreDir(env: NodeJS.ProcessEnv = process.env, bundled = BUNDLED): string | null {
	const candidates = [env.SCUTOONS_CORE_DIR, bundled ? undefined : resolve(ROOT, 'shared/live-deck/wasm')];
	return candidates.find((d): d is string => !!d && existsSync(resolve(d, JS)) && existsSync(resolve(d, WASM))) ?? null;
}

async function download(viewer: string, fetcher: typeof fetch): Promise<string> {
	if (!viewer.startsWith('https://') && !viewer.startsWith('http://localhost') && !viewer.startsWith('http://127.0.0.1')) throw new Error(`the core is not served over https at ${viewer}`);
	const get = async (file: string) => {
		const r = await fetcher(`${viewer}/live/${file}`);
		if (!r.ok) throw new Error(`${viewer}/live/${file}: ${r.status}`);
		return new Uint8Array(await r.arrayBuffer());
	};
	const js = await get(JS);
	const dir = resolve(tmpdir(), 'scutoons-core', createHash('sha256').update(js).digest('hex').slice(0, 16));
	if (!existsSync(resolve(dir, WASM))) {
		mkdirSync(dir, { recursive: true });
		writeFileSync(resolve(dir, JS), js);
		writeFileSync(resolve(dir, WASM), await get(WASM));
	}
	return dir;
}

/** the core's history rules, loaded once (on the first revert) */
export function coreHistory(viewer: string, fetcher: typeof fetch = fetch): () => Promise<HistoryView> {
	let core: Promise<HistoryView> | null = null;
	return () =>
		(core ??= (async () => {
			const dir = coreDir() ?? (await download(viewer, fetcher));
			const mod = (await import(pathToFileURL(resolve(dir, JS)).href)) as CoreModule;
			await mod.default({ module_or_path: readFileSync(resolve(dir, WASM)) });
			return historyView(mod);
		})().catch((e) => {
			core = null;
			throw e;
		}));
}
