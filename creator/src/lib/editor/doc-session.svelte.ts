// One editor page's autosave / publish / conflict state: every page that edits documents (the meta pages, the chapter
// editor, the toon page) shares it. Edits save on their own as draft versions (`useAutosave`), the bar's Publish saves
// the page first and has it read again what went live, each save / publish posts its typed Log line and a conflict stays
// as the page's bar (DocNotices) until it is answered.
import { confirmDialog } from '@scutoons/ui';
import { beforeNavigate, goto } from '$app/navigation';
import { m } from '$lib/paraglide/messages.js';
import { docLabel } from '$lib/labels.ts';
import { SaveConflict, SaveError, type ContentDoc } from '$lib/editor/doc.svelte.ts';
import { toast, toaster } from '$lib/toasts.ts';
import type { LogKind } from '@scutoons/content';
import { suggestNote } from '$lib/editor/version-note-words.ts';
import { AUTOSAVE_MS, Autosaver } from '$lib/editor/autosaver.ts';

export interface DocSessionOptions {
	/** the page's files (validated, saved, published) */
	docs: () => ContentDoc[];
	/** files that follow them (saved when dirty, published with the docs – the chapters whose status mirrors the season) */
	companions?: () => ContentDoc[];
	/** before every save / publish (a pending edit written into its doc, the chapter's status mirrored …) */
	beforeSave?: () => void;
	/** after a successful save (e.g. the chapter's image list reloaded) */
	afterSave?: () => void;
	/** errors in the problem list (the page hands them to the bar: publishing asks once more) */
	errorCount?: () => number;
}

export class DocSession {
	conflict = $state<{ doc: ContentDoc; error: SaveConflict } | null>(null);
	/** a save runs (the status bar's "saving…") */
	saving = $state(false);
	/** companions saved on this page (published together with the docs) */
	private savedCompanions: ContentDoc[] = [];
	/** this page's last toast: a new save / publish replaces it (an old error must not outlive a good save) */
	private lastToast: number | null = null;
	/** the next save overwrites a conflicting version (the conflict bar's Overwrite) */
	private force = false;
	private readonly saver: Autosaver;

	constructor(
		private readonly options: DocSessionOptions,
		delay = AUTOSAVE_MS
	) {
		this.saver = new Autosaver(() => this.saveAll(), delay);
	}

	private get docs() {
		return this.options.docs();
	}
	private get companions() {
		return this.options.companions?.() ?? [];
	}
	/** every file the session watches and saves */
	get all(): ContentDoc[] {
		return [...this.docs, ...this.companions];
	}
	get anyDirty() {
		return this.all.some((d) => d.dirty);
	}
	get anySaving() {
		return this.saving || this.docs.some((d) => d.saving);
	}

	/** this page's Log line (the previous toast goes) */
	notify(message: string, type: 'info' | 'success' | 'error' = 'info', kind?: LogKind) {
		this.clearNotice();
		this.lastToast = toast(message, { type, ...(kind ? { kind } : {}) });
	}
	private clearNotice() {
		if (this.lastToast !== null) toaster.dismiss(this.lastToast);
		this.lastToast = null;
	}

	/** saves a moment after the last change (a newer change restarts the wait) */
	schedule() {
		this.saver.schedule();
	}
	/** a save waits for its pause or runs */
	get pending(): boolean {
		return this.saver.pending || this.saver.busy;
	}

	/** saves what is unsaved now (one save at a time, Autosaver); nothing while a conflict waits for its answer; false on a conflict or an error */
	flush(): Promise<boolean> {
		if (this.conflict && !this.force) {
			this.saver.cancel();
			return Promise.resolve(false);
		}
		return this.saver.flush();
	}

	/** saves every dirty file as a new draft version, its note what changed; false on a conflict or an error */
	private async saveAll(): Promise<boolean> {
		const force = this.force;
		this.force = false;
		this.options.beforeSave?.();
		const dirty = this.all.filter((d) => d.dirty);
		if (!dirty.length) return true;
		this.conflict = null;
		this.saving = true;
		let note: string | undefined;
		try {
			note = suggestNote(dirty) || undefined;
		} catch (e) {
			// a note never stands in the way of saving
			console.warn('version note suggestion failed', e);
		}
		try {
			for (const doc of dirty) {
				try {
					await doc.save(note, force);
					if (!this.docs.includes(doc) && !this.savedCompanions.includes(doc)) this.savedCompanions.push(doc);
				} catch (e) {
					if (e instanceof SaveConflict) this.conflict = { doc, error: e };
					else this.notify(`${docLabel(doc)}${e instanceof SaveError ? ` (${e.locale})` : ''}: ${e instanceof Error ? e.message : String(e)}`, 'error', 'save');
					return false;
				}
			}
		} finally {
			this.saving = false;
		}
		this.notify(note ? `${m['creator.saved']()} · ${note}` : m['creator.saved'](), 'success', 'save');
		this.options.afterSave?.();
		return true;
	}

	/** before the bar's Publish dialog lists what waits (RULES H18): the page saved first (false: a conflict or an error) */
	async preparePublish(): Promise<boolean> {
		this.options.beforeSave?.();
		return !(this.anyDirty || this.pending) || (await this.flush());
	}

	/** after a publish: the page's files that went live read again */
	async afterPublish(published: string[]): Promise<void> {
		const live = new Set(published);
		for (const doc of [...this.all, ...this.savedCompanions]) if (doc.storedKeys().some((k) => live.has(k))) await doc.load();
		this.savedCompanions = [];
		this.clearNotice();
	}

	/** what the page hands the bar's Publish (PageDoc): save first, its errors asked about once more, read again after */
	get publishHooks() {
		return { prepare: () => this.preparePublish(), published: (r: { published: string[] }) => this.afterPublish(r.published), blocking: this.options.errorCount?.() ?? 0 };
	}

	/** the conflict bar's buttons: load the other version, or overwrite it with this one */
	async resolveConflict(action: 'reload' | 'overwrite'): Promise<void> {
		if (!this.conflict) return;
		const { doc } = this.conflict;
		if (action === 'reload') {
			this.conflict = null;
			await doc.load();
		} else {
			this.force = true;
			await this.flush();
		}
	}
}

/** what a set of documents holds now ('' while one loads) – a change of it is an edit */
const signature = (docs: ContentDoc[]) => (docs.some((d) => d.loading) ? '' : docs.map((d) => JSON.stringify(d.data)).join('\u0000'));

/**
 * The creator's one autosave (call during component init): every edit of the session's documents saves as a draft
 * version a moment (AUTOSAVE_MS) after the last change, one save at a time; leaving the page saves what waits (a conflict still
 * asks before leaving; closing the tab with something unsaved keeps the browser's question). What a load brings in
 * (a repaired value) is not an edit: it saves with the next one.
 */
export function useAutosave(session: DocSession) {
	let seen: string | null = null;
	$effect(() => {
		const now = signature(session.all);
		if (!now) {
			seen = null;
			return;
		}
		if (seen === null) {
			seen = now;
			return;
		}
		if (now === seen) return;
		seen = now;
		session.schedule();
	});
	let leaving = false;
	beforeNavigate((navigation) => {
		if (leaving) return;
		if (navigation.type === 'leave') {
			if (session.anyDirty || session.pending) {
				navigation.cancel();
				void session.flush();
			}
			return;
		}
		if (!session.conflict) {
			if (session.anyDirty || session.pending) void session.flush();
			return;
		}
		navigation.cancel();
		const to = navigation.to?.url;
		if (!to) return;
		confirmDialog({ title: m['creator.leave_title'](), message: m['creator.leave_confirm'](), confirmLabel: m['creator.leave_label']() }).then((ok) => {
			if (!ok) return;
			leaving = true;
			goto(to).finally(() => (leaving = false));
		});
	});
	$effect(() => () => {
		if (session.pending || session.anyDirty) void session.flush();
	});
}
