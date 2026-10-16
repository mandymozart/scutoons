// A saved chapter whose version changed is refreshed (PWA phase 3, agents/research/PWA.md §2 "Phase 3", FEATURES 10.74,
// owner 2026-10-16 "alle Pakete angehen"): on a start (root layout, after the access check) and when the browser is
// online again, each saved chapter's file list is asked once more (at most every 30 min); a new version (the chapter's
// ETag) or other files → only the changed files are downloaded (savedChapters.update). In the background when the rules
// allow it (saved-update-rules.ts `refreshMode`: small ones always, bigger ones as the setting "Save chapters on this
// device" would save – never with data saver / tier none); else the account page offers "Update" (`outdated`).
// The list may come from the worker's library cache (up to 10 min old, PWA.md §2 "Phase 2") – a publish shows soon after.
import { browser } from '$app/environment';
import { dataBudget } from '$lib/data-budget.svelte.ts';
import { whenFirstScreen } from '$lib/lookahead.ts';
import { asset } from '$lib/utils/cdn.ts';
import { savedReadable, type ConnectionHint } from '$lib/sw/saved-rules.ts';
import { planRefresh, refreshDue, refreshMode } from '$lib/sw/saved-update-rules.ts';
import { autoSave } from './auto-save.svelte.ts';
import { savedChapters } from './saved-chapters.svelte.ts';

const href = (path: string) => new URL(asset(path)).href;
const connection = () => (navigator as Navigator & { connection?: ConnectionHint }).connection;
const key = (toon: string, chapter: number) => `${toon}/${chapter}`;

/** when each saved chapter was last asked (this page) */
const checked = new Map<string, number>();
let running: Promise<void> | null = null;

function setOutdated(id: string, bytes: number | null) {
	const { [id]: _was, ...rest } = savedChapters.outdated;
	savedChapters.outdated = bytes === null ? rest : { ...rest, [id]: bytes };
}

/** asks for new versions of the saved chapters and refreshes them (or offers it) – one run at a time */
export function checkSavedVersions(): Promise<void> {
	if (!browser || !savedChapters.supported || !navigator.onLine) return Promise.resolve();
	running ??= (async () => {
		try {
			// never in the way of the page's first screen
			await whenFirstScreen();
			await savedChapters.loaded();
			for (const saved of [...savedChapters.list]) {
				const id = key(saved.toon, saved.chapter);
				if (!savedReadable(saved) || savedChapters.job(saved.toon, saved.chapter) || !refreshDue(checked.get(id))) continue;
				checked.set(id, Date.now());
				const list = await savedChapters.files(saved.toon, saved.chapter, saved.locale).catch(() => null);
				if (!list) continue;
				const plan = planRefresh(saved, list, href);
				if (!plan.changed) {
					setOutdated(id, null);
					continue;
				}
				if (refreshMode(plan.bytes, autoSave.value, connection(), dataBudget.tier) === 'ask') setOutdated(id, plan.bytes);
				else await savedChapters.update(saved, list, plan);
			}
		} finally {
			running = null;
		}
	})();
	return running;
}

/** the account page's "Update": one saved chapter takes in its new version now */
export async function updateSaved(toon: string, chapter: number): Promise<'updated' | 'cancelled' | 'failed'> {
	const saved = savedChapters.get(toon, chapter);
	if (!saved) return 'failed';
	try {
		const list = await savedChapters.files(toon, chapter, saved.locale);
		const plan = planRefresh(saved, list, href);
		if (!plan.changed) {
			setOutdated(key(toon, chapter), null);
			return 'updated';
		}
		return await savedChapters.update(saved, list, plan);
	} catch {
		return 'failed';
	}
}

// back online: the saved chapters may have new versions
if (browser) window.addEventListener('online', () => void checkSavedVersions());
