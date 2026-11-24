// The Library's actions on one or several families – the tile's menu, the details, the selection bar and the keys (F2,
// Delete) share them: rename (the name only), delete (refused while used, then Delete anyway), download, copy the path.
import type { AssetFamily } from '@scutoons/media';
import { askDialog, confirmDialog } from '@scutoons/ui';
import { ApiError } from '@scutoons/api';
import { library } from '$lib/editor/library.svelte.ts';
import { confirmDelete } from '$lib/editor/ask.ts';
import { asset } from '$lib/utils/cdn.ts';
import { toast } from '$lib/toasts.ts';
import { m } from '$lib/paraglide/messages.js';

const reason = (e: unknown) => (e instanceof ApiError && e.status === 403 ? m['library.admin_only']() : e instanceof Error ? e.message : String(e));

/** F2: a new name; every URL and every use stay */
export async function renameFamily(family: AssetFamily) {
	const title = await askDialog({ title: m['library.rename_title'](), message: m['library.rename_note'](), label: m['library.name'](), value: family.title, select: true, confirmLabel: m['library.rename']() });
	if (!title?.trim() || title.trim() === family.title) return;
	try {
		await library.rename(family, title.trim());
	} catch (e) {
		toast(m['library.upload_failed']({ reason: reason(e) }), { type: 'error', kind: 'library' });
	}
}

/** Delete: refused while anything uses the files; then Delete anyway, which the API allows for draft uses only */
export async function deleteFamilies(families: AssetFamily[]) {
	if (!families.length) return;
	const what = families.length === 1 ? families[0].title : m['library.n_files']({ count: families.length });
	try {
		const check = await library.deleteCheck(families);
		if (check.free) {
			if (await confirmDelete(m['library.delete_confirm']({ what }))) await library.remove(check.ids, false);
			return;
		}
		if (check.blocking > 0) {
			toast(m['library.delete_refused']({ what, count: check.blocking + check.drafts }), { type: 'error', kind: 'library' });
			return;
		}
		const anyway = await confirmDialog({ title: m['library.delete_used_title'](), message: m['library.delete_used']({ what, count: check.drafts }), confirmLabel: m['library.delete_anyway']() });
		if (anyway) await library.remove(check.ids, true);
	} catch (e) {
		toast(m['library.upload_failed']({ reason: reason(e) }), { type: 'error', kind: 'library' });
	}
}

/** the files the families hand on (their main version), one download each */
export function downloadFamilies(families: AssetFamily[]) {
	for (const f of families) {
		const a = document.createElement('a');
		a.href = asset(f.main);
		a.download = f.main.split('/').pop() ?? f.name;
		a.target = '_blank';
		a.rel = 'noopener';
		a.click();
	}
}

export function copyPath(family: AssetFamily) {
	navigator.clipboard?.writeText(family.main).catch(() => {});
}
