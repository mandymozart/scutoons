// One editor page's save / publish / conflict state (COMPONENTS.md step 11): the meta pages (meta-page.svelte) and the
// chapter editor share it – before, the chapter editor had a copy that had drifted. The page passes its files; the
// session saves the dirty ones (asking for a note first), publishes after one confirm dialog that names exactly what
// is published and in which language (owner 2026-10-03, 4.4.3), and keeps the notice / conflict the page shows
// (DocNotices).
import { askDialog, confirmDialog } from '@scutoons/ui';
import { NEUTRAL_LOCALE } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';
import { fileLabel, fileLabels } from '$lib/labels.ts';
import { SaveConflict, type ContentDoc } from '$lib/editor/doc.svelte.ts';

/**
 * The confirm dialog before publishing (one structure everywhere): the files and their languages (none for the
 * language-neutral entities), after a first one when the problem list has errors. False = cancelled.
 */
export async function confirmPublish(files: ContentDoc[], errorCount = 0): Promise<boolean> {
	if (errorCount && !(await confirmDialog({ title: m['creator.publish_title'](), message: m['creator.publish_errors']({ count: errorCount }), confirmLabel: m['creator.publish']() }))) return false;
	const locales = [...new Set(files.flatMap((d) => [...d.dirtyLocales, ...d.unpublished]))].filter((l) => l !== NEUTRAL_LOCALE).join(', ');
	const file = fileLabels(files.map((d) => d.entity));
	return confirmDialog({
		title: m['creator.publish_title'](),
		message: locales ? m['creator.publish_confirm']({ file, locales }) : m['creator.publish_confirm_neutral']({ file }),
		confirmLabel: m['creator.publish']()
	});
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
	/** errors in the problem list: publishing asks once more */
	errorCount?: () => number;
}

export class DocSession {
	notice = $state<string | null>(null);
	conflict = $state<{ doc: ContentDoc; error: SaveConflict } | null>(null);
	/** the optional note of the next save */
	message = $state('');
	/** companions saved on this page (published together with the docs) */
	private savedCompanions: ContentDoc[] = [];

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

	/** saves every dirty file (the note as the versions' message); false on a conflict or an error (shown as the notice / conflict bar) */
	async saveAll(force = false): Promise<boolean> {
		this.notice = null;
		this.conflict = null;
		this.options.beforeSave?.();
		for (const doc of [...this.docs, ...this.companions]) {
			if (!doc.dirty) continue;
			try {
				await doc.save(this.message.trim() || undefined, force);
				if (!this.docs.includes(doc) && !this.savedCompanions.includes(doc)) this.savedCompanions.push(doc);
			} catch (e) {
				if (e instanceof SaveConflict) this.conflict = { doc, error: e };
				else this.notice = `${fileLabel(doc.entity)}: ${e instanceof Error ? e.message : String(e)}`;
				return false;
			}
		}
		this.message = '';
		this.notice = m['creator.saved']();
		this.options.afterSave?.();
		return true;
	}

	/** "Save draft": never a silent no-op (owner 2026-09-30); without a note, ask for one – it can be skipped (owner 2026-10-03) */
	async saveDraft(): Promise<void> {
		this.options.beforeSave?.();
		if (!this.anyDirty) {
			this.notice = m['creator.nothing_to_save']();
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

	/** "Publish": saves first, then publishes every file with an unpublished version (after the confirm dialog) */
	async publishAll(): Promise<void> {
		this.options.beforeSave?.();
		const follow = [...new Set([...this.companions.filter((d) => d.dirty), ...this.savedCompanions])];
		const files = [...this.docs.filter((d) => d.dirty || d.unpublished.length), ...follow];
		if (!files.length) {
			this.notice = m['creator.nothing_to_publish']();
			return;
		}
		if (!(await confirmPublish(files, this.options.errorCount?.() ?? 0))) return;
		if (!(await this.saveAll())) return;
		try {
			for (const doc of [...this.docs, ...this.savedCompanions]) if (doc.stored.length && doc.unpublished.length) await doc.publish();
			this.savedCompanions = [];
			this.notice = m['creator.published']();
		} catch (e) {
			this.notice = e instanceof Error ? e.message : String(e);
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
