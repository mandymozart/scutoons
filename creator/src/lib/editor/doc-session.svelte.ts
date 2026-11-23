// One editor page's autosave / publish / newer-version state: every page that edits documents (the meta pages, the chapter
// editor, the toon page) shares it. Edits save on their own as draft versions (`useAutosave`), the bar's Publish saves
// the page first and has it read again what went live, each save / publish posts its typed Log line. A newer version on
// the server – asked for on focus and every 30 s, or learnt from a refused save – is the Reload notice until it is answered.
import { confirmDialog } from '@scutoons/ui';
import { untrack } from 'svelte';
import { beforeNavigate, goto } from '$app/navigation';
import { m } from '$lib/paraglide/messages.js';
import { docLabel } from '$lib/labels.ts';
import { newestOf } from '$lib/api.ts';
import { SaveConflict, SaveError, type ContentDoc } from '$lib/editor/doc.svelte.ts';
import { toast, toaster } from '$lib/toasts.ts';
import type { LogKind } from '@scutoons/content';
import { suggestNote } from '$lib/editor/version-note-words.ts';
import { AUTOSAVE_MS, Autosaver } from '$lib/editor/autosaver.ts';
import { shared } from '$lib/utils/shared.ts';

/** how often an open page asks for a newer version while the window is visible (RULES H19; the deck asks as often) */
export const NEWER_CHECK_MS = 30_000;

/** a file of the page whose newest version on the server is not the one held */
export interface NewerVersion {
	doc: ContentDoc;
	locale: string;
	version: number;
}

/** what the Reload notice says: which file, and whether changes of this page would go with a reload */
export interface ReloadNotice {
	doc: ContentDoc;
	locale: string;
	unsaved: boolean;
}

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
	/** files whose newest version on the server is newer than the one held (`checkNewer`) */
	newer = $state.raw<NewerVersion[]>([]);
	private asking = false;
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

	/**
	 * Asks the server whether a file held here has a newer version (304 = the same; nothing while a save runs: its answer
	 * is the new version held). A check that fails says nothing – offline, the refused save stays the backstop.
	 */
	async checkNewer(): Promise<void> {
		if (this.asking || this.anySaving || this.saver.busy) return;
		this.asking = true;
		try {
			const found: NewerVersion[] = [];
			for (const doc of this.all) {
				if (doc.loading) continue;
				for (const locale of doc.stored) {
					const held = doc.base[locale];
					if (held === null || held === undefined) continue;
					try {
						const answer = await newestOf(doc.key(locale), held);
						// a save of this page that landed while it asked is what it holds now, not newer
						if (answer.state === 'newer' && answer.version !== doc.base[locale]) found.push({ doc, locale, version: answer.version });
					} catch {
						// signed out, gone, offline: no notice
					}
				}
			}
			this.newer = found;
		} finally {
			this.asking = false;
		}
	}

	/** the Reload notice: a newer version found, or a save refused because of one */
	get notice(): ReloadNotice | null {
		const first = this.conflict ? { doc: this.conflict.doc, locale: this.conflict.error.locale } : this.newer[0];
		if (!first) return null;
		return { doc: first.doc, locale: first.locale, unsaved: !!this.conflict || this.anyDirty || this.pending };
	}

	/** the notice's Reload: the newer versions are loaded – changes of this page go only after a confirm */
	async reload(): Promise<void> {
		const notice = this.notice;
		if (!notice) return;
		if (notice.unsaved) {
			const ok = await confirmDialog({
				title: m['creator.newer_confirm_title'](),
				message: m['creator.newer_confirm']({ file: docLabel(notice.doc) }),
				confirmLabel: m['creator.reload']()
			});
			if (!ok) return;
		}
		const docs = new Set([...this.newer.map((n) => n.doc), ...(this.conflict ? [this.conflict.doc] : [])]);
		this.saver.cancel();
		this.conflict = null;
		this.newer = [];
		for (const doc of docs) {
			await doc.load();
			doc.clearAutosave();
		}
		// another file of the page with changes of its own still saves them
		if (this.anyDirty) this.schedule();
	}

	/** the notice's Keep mine: this page's changes are saved over the newer version (that one stays in the History) */
	async keepMine(): Promise<void> {
		this.newer = [];
		this.force = true;
		await this.flush();
	}
}

/** the open page's session, while it watches for newer versions (the Reload notice reads it) */
class Watched {
	session = $state.raw<DocSession | null>(null);
}
export const watched = shared('creator-watched-session', () => new Watched());

/**
 * Asks for newer versions of the session's files while the page is mounted (call during component init): when the window
 * gets the focus and every NEWER_CHECK_MS while it is visible (RULES H19).
 */
export function useNewerWatch(session: DocSession) {
	$effect(() => {
		untrack(() => (watched.session = session));
		const ask = () => {
			if (document.visibilityState === 'visible') void session.checkNewer();
		};
		const timer = setInterval(ask, NEWER_CHECK_MS);
		window.addEventListener('focus', ask);
		document.addEventListener('visibilitychange', ask);
		return () => {
			clearInterval(timer);
			window.removeEventListener('focus', ask);
			document.removeEventListener('visibilitychange', ask);
			untrack(() => {
				if (watched.session === session) watched.session = null;
			});
		};
	});
}

/** what a set of documents holds now ('' while one loads) – a change of it is an edit */
const signature = (docs: ContentDoc[]) => (docs.some((d) => d.loading) ? '' : docs.map((d) => JSON.stringify(d.data)).join('\u0000'));

/**
 * The creator's one autosave (call during component init): every edit of the session's documents saves as a draft
 * version a moment (AUTOSAVE_MS) after the last change, one save at a time; leaving the page saves what waits (a conflict still
 * asks before leaving; closing the tab with something unsaved keeps the browser's question). What a load brings in
 * (a repaired value) is not an edit: it saves with the next one. A page that saves on its own also watches for newer versions.
 */
export function useAutosave(session: DocSession) {
	useNewerWatch(session);
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
