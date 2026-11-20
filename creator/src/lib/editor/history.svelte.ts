// The creator's History (#687, after OSCT's #312): the page's stored files, their saved steps, a revert planned by the
// core (deck-core history through the deck's wasm) and stored by the server as a new draft, and this session's undo /
// redo of the person's own saved steps. Every rule – what a step is, what changed, what a revert produces – is the core's.
import type { HistoryDescription as Description, HistoryStep as Step } from '@scutoons/content';
import { createHistory, savedOf, type RevertPlan } from '@scutoons/api';
import { historyCore } from './core-history.ts';
import { contentApi, type ContentVersion } from '$lib/api.ts';
import { shared } from '$lib/utils/shared.ts';
import { confirmDialog } from '@scutoons/ui';
import { m } from '$lib/paraglide/messages.js';
import type { ContentDoc } from './doc.svelte.ts';
import { CollectionDoc } from './world-map-doc.svelte.ts';

export type { Description, RevertPlan, Step };

export { historyCore };

const history = createHistory(contentApi, historyCore);

export const contentOf = history.contentOf;
export const saved = savedOf;

/** one stored file the History lists: a document's language, or the selected entry of a document made of many */
export interface HistoryFile {
	doc: ContentDoc;
	locale: string;
	key: string;
	/** a selected entry of a document made of many: removed through its page, never unpublished here */
	entry: boolean;
}

export function historyFiles(docs: ContentDoc[]): HistoryFile[] {
	return docs.flatMap((doc) => {
		const collection = doc instanceof CollectionDoc ? doc : null;
		const entry = !!collection && collection.historyKey !== collection.spec.key;
		return doc.locales.filter((l) => doc.source[l] === 'store').map((locale) => ({ doc, locale, key: collection?.historyKey ?? doc.key(locale), entry }));
	});
}

/** the version the file's editor stands on */
export function baseOf(file: HistoryFile): number | null {
	if (file.entry && file.doc instanceof CollectionDoc) return file.doc.versions[file.key]?.latest ?? null;
	return file.doc.base[file.locale] ?? null;
}

export interface FileHistory {
	file: HistoryFile;
	versions: ContentVersion[];
	steps: Step[];
}

export async function loadFile(file: HistoryFile): Promise<FileHistory> {
	return { file, ...(await history.load(file.key)) };
}

/** what a step changed, in words (null: nothing before it to compare with) */
export const describeStep = history.describe;

export async function planRevert(file: HistoryFile, before: number, after: number): Promise<RevertPlan | null> {
	const base = baseOf(file);
	return base === null ? null : history.plan(file.key, before, after, base);
}

/** stores a planned revert of version `version`'s step; the new version's id */
export async function applyRevert(file: HistoryFile, version: number, plan: RevertPlan): Promise<number> {
	const id = await history.apply(file.key, version, plan);
	await file.doc.load();
	return id;
}

/** back to version `version`: its document as a new draft */
export async function applyBackTo(file: HistoryFile, version: number, base: number): Promise<number> {
	const id = await history.backTo(file.key, version, base);
	await file.doc.load();
	return id;
}

export type Outcome = 'done' | 'none' | 'cancelled' | 'stale';

/**
 * This session's undo / redo of the person's own saved steps: undo reverts the newest own step not yet undone, redo
 * reverts that revert. A save of anything else makes the redo stale.
 */
class SavedUndo {
	readonly started = Date.now();
	#undone = new Set<number>();
	#made = new Set<number>();
	redos = $state<{ file: HistoryFile; before: number; after: number }[]>([]);

	/** `ask`: the prompt when places changed since stay as they are */
	async undo(files: HistoryFile[], me: string, ask: (plan: RevertPlan) => Promise<boolean>): Promise<Outcome> {
		const found = (await Promise.all(files.map(loadFile))).flatMap((f) =>
			f.steps.filter((s) => s.author === me && s.at_ms >= this.started && s.before !== null && !this.#undone.has(s.last) && !this.#made.has(s.last)).map((step) => ({ file: f.file, step }))
		);
		const pick = found.sort((a, b) => b.step.at_ms - a.step.at_ms)[0];
		if (!pick) return 'none';
		const plan = await planRevert(pick.file, pick.step.before!, pick.step.last);
		if (!plan) return 'none';
		this.#undone.add(pick.step.last);
		if (!plan.applied) return 'none';
		if (plan.kept.length && !(await ask(plan))) {
			this.#undone.delete(pick.step.last);
			return 'cancelled';
		}
		const id = await applyRevert(pick.file, pick.step.last, plan);
		this.#made.add(id);
		this.redos = [...this.redos, { file: pick.file, before: plan.base, after: id }];
		return 'done';
	}

	async redo(ask: (plan: RevertPlan) => Promise<boolean>): Promise<Outcome> {
		const last = this.redos.at(-1);
		if (!last) return 'none';
		if (baseOf(last.file) !== last.after) {
			this.redos = [];
			return 'stale';
		}
		const plan = await planRevert(last.file, last.before, last.after);
		if (!plan?.applied) return 'none';
		if (plan.kept.length && !(await ask(plan))) return 'cancelled';
		await applyRevert(last.file, last.after, plan);
		this.redos = this.redos.slice(0, -1);
		return 'done';
	}
}

export const savedUndo = shared('creator-saved-undo', () => new SavedUndo());

/** places changed since stay as they are: asked before an undo / redo goes on */
const askKept = (plan: RevertPlan) =>
	confirmDialog({ title: m['history.undo_title'](), message: m['history.kept']({ places: plan.kept.slice(0, 6).join(', ') }), confirmLabel: m['history.undo']() });

/** a page's ↶: its unsaved edits first (the documents' own undo; a saved step only once nothing is unsaved), then the person's saved steps of this session */
export function pageUndo(docs: ContentDoc[], me: string): Promise<Outcome> | void {
	const dirty = docs.filter((d) => d.dirty);
	if (dirty.length) return dirty.filter((d) => d.canUndo).sort((a, b) => b.lastEditAt - a.lastEditAt)[0]?.undo();
	return savedUndo.undo(historyFiles(docs), me, askKept);
}

/** a page's ↷: the documents' own redo while there is one, else the last saved undo of this session */
export function pageRedo(docs: ContentDoc[]): Promise<Outcome> | void {
	const local = docs.filter((d) => d.canRedo).sort((a, b) => b.lastEditAt - a.lastEditAt)[0];
	if (docs.some((d) => d.dirty) || (local && !savedUndo.redos.length)) return local?.redo();
	return savedUndo.redo(askKept);
}
