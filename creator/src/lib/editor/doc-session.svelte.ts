// One editor page's save / publish / conflict state (COMPONENTS.md step 11): the meta pages (meta-page.svelte) and the
// chapter editor share it – before, the chapter editor had a copy that had drifted. The page passes its files; the
// session saves the dirty ones (asking for a note first), publishes through the one checklist of every file with a
// draft at the page's level (publish-tree.ts, BACKLOG 10.60 – owner 2026-10-11), says what happened as a toast (UI-CONCEPTS E.1:
// Saved, Published, Nothing to save, errors – lib/toasts.ts) and keeps the conflict the page shows as a bar (DocNotices).
import { askDialog, confirmDialog } from '@scutoons/ui';
import { publishTree, type PublishScope } from '$lib/editor/publish-tree.ts';
import { m } from '$lib/paraglide/messages.js';
import { docLabel } from '$lib/labels.ts';
import { SaveConflict, SaveError, type ContentDoc } from '$lib/editor/doc.svelte.ts';
import { toast, toaster } from '$lib/toasts.ts';

export interface DocSessionOptions {
	/** the page's files (validated, saved, published) */
	docs: () => ContentDoc[];
	/** files that follow them (saved when dirty, published with the docs – the chapters whose status mirrors the season) */
	companions?: () => ContentDoc[];
	/** before every save / publish (a pending edit written into its doc, the chapter's status mirrored …) */
	beforeSave?: () => void;
	/** after a successful save (e.g. the chapter's image list reloaded) */
	afterSave?: () => void;
	/** errors in the problem list: publishing asks once more */
	errorCount?: () => number;
	/**
	 * the page's level in the tree (BACKLOG 10.60): Publish saves the page, then lists every file below it with a draft
	 * to tick (publish-tree.ts) instead of only the page's own files; `title` = the dialog's title
	 */
	tree?: () => { scope: PublishScope; title: string } | null;
}

export class DocSession {
	conflict = $state<{ doc: ContentDoc; error: SaveConflict } | null>(null);
	/** the optional note of the next save */
	message = $state('');
	/** companions saved on this page (published together with the docs) */
	private savedCompanions: ContentDoc[] = [];
	/** this page's last toast: a new save / publish replaces it (an old error must not outlive a good save) */
	private lastToast: number | null = null;

	constructor(private readonly options: DocSessionOptions) {}

	private get docs() {
		return this.options.docs();
	}
	private get companions() {
		return this.options.companions?.() ?? [];
	}
	get anyDirty() {
		return this.docs.some((d) => d.dirty) || this.companions.some((d) => d.dirty);
	}
	get anySaving() {
		return this.docs.some((d) => d.saving);
	}

	/** this page's toast (the previous one goes) */
	notify(message: string, type: 'info' | 'success' | 'error' = 'info') {
		this.clearNotice();
		this.lastToast = toast(message, { type });
	}
	private clearNotice() {
		if (this.lastToast !== null) toaster.dismiss(this.lastToast);
		this.lastToast = null;
	}

	/** saves every dirty file (the note as the versions' message); false on a conflict or an error (an error toast / the conflict bar) */
	async saveAll(force = false): Promise<boolean> {
		this.clearNotice();
		this.conflict = null;
		this.options.beforeSave?.();
		for (const doc of [...this.docs, ...this.companions]) {
			if (!doc.dirty) continue;
			try {
				await doc.save(this.message.trim() || undefined, force);
				if (!this.docs.includes(doc) && !this.savedCompanions.includes(doc)) this.savedCompanions.push(doc);
			} catch (e) {
				if (e instanceof SaveConflict) this.conflict = { doc, error: e };
				else this.notify(`${docLabel(doc)}${e instanceof SaveError ? ` (${e.locale})` : ''}: ${e instanceof Error ? e.message : String(e)}`, 'error');
				return false;
			}
		}
		this.message = '';
		this.notify(m['creator.saved'](), 'success');
		this.options.afterSave?.();
		return true;
	}

	/** "Save draft": never a silent no-op (owner 2026-09-30); without a note, ask for one – it can be skipped (owner 2026-10-03) */
	async saveDraft(): Promise<void> {
		this.options.beforeSave?.();
		if (!this.anyDirty) {
			this.notify(m['creator.nothing_to_save']());
			return;
		}
		if (!this.message.trim()) {
			const note = await askDialog({
				title: m['creator.note_title'](),
				message: m['creator.note_message'](),
				label: m['creator.note_label'](),
				placeholder: m['creator.note_placeholder'](),
				confirmLabel: m['creator.note_save'](),
				skipLabel: m['creator.note_skip']()
			});
			if (note === null) return;
			this.message = note;
		}
		await this.saveAll();
	}

	/**
	 * "Publish" – one dialog everywhere (owner 2026-10-11: "kein eigenes Publishing-Interface für alles, sondern für alles
	 * gleich"): the page saved first, then the checklist (publish-tree.ts) of every file with a draft at the page's level –
	 * below it (`tree`: a chapter, a season, a toon) or the page's own files.
	 */
	async publishAll(): Promise<void> {
		const tree = this.options.tree?.();
		await this.publishTree(tree?.scope ?? null, tree?.title ?? m['creator.publish_tree_title']({ name: this.docs[0] ? docLabel(this.docs[0]) : '' }));
	}

	/** The page saved first, then the checklist (`scope` null = the page's own files); the page reloads what went live */
	async publishTree(scope: PublishScope | null, title: string): Promise<void> {
		this.options.beforeSave?.();
		if (this.options.errorCount?.() && !(await confirmDialog({ title: m['creator.publish_title'](), message: m['creator.publish_errors']({ count: this.options.errorCount?.() ?? 0 }), confirmLabel: m['creator.publish']() }))) return;
		if ([...this.docs, ...this.companions].some((d) => d.dirty) && !(await this.saveAll())) return;
		try {
			const own = [...this.docs, ...this.companions];
			const result = await publishTree(scope ?? { kind: 'files', toon: own[0]?.toon ?? '', keys: own.flatMap((d) => d.storedKeys()) }, title);
			if (!result) return;
			const live = new Set(result.published);
			for (const doc of [...this.docs, ...this.companions]) if (doc.storedKeys().some((k) => live.has(k))) await doc.load();
			this.savedCompanions = [];
			this.notify(result.message, result.published.length ? 'success' : undefined);
		} catch (e) {
			this.notify(e instanceof Error ? e.message : String(e), 'error');
		}
	}

	/** the conflict bar's buttons: load the other version, or overwrite it with this one */
	async resolveConflict(action: 'reload' | 'overwrite'): Promise<void> {
		if (!this.conflict) return;
		const { doc } = this.conflict;
		this.conflict = null;
		if (action === 'reload') await doc.load();
		else await this.saveAll(true);
	}
}
