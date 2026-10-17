// "Save chapters on this device" by itself (owner 2026-10-16: "Dann muss eine default Einstellung am Gerät speichern in die
// account settings"): the account page's setting Off / On Wi-Fi / Always, kept on this device (the saves are this
// device's – localStorage, default Off). With it on, a chapter the reader opens and may read is saved in the background
// once its first screen is there – at the reader's quality, without the videos when they are the bulk (as the row's
// dialog offers), never with data saver on. Rules: lib/sw/saved-rules.ts `autoSaveAllowed` / `autoSaveVideos` (tested).
import { browser } from '$app/environment';
import { effectiveQuality } from '@scutoons/content';
import { mediaQuality } from '$lib/media-quality.svelte.ts';
import { whenFirstScreen } from '$lib/lookahead.ts';
import { savedChapters } from '$lib/offline/saved-chapters.svelte.ts';
import { accessUntilOf, AUTO_SAVE_SETTINGS, autoSaveAllowed, autoSaveVideos, planSave, type AutoSave, type ConnectionHint } from '$lib/sw/saved-rules.ts';

const KEY = 'scutoons_auto_save';

function read(): AutoSave {
	try {
		const value = localStorage.getItem(KEY) as AutoSave | null;
		return value && AUTO_SAVE_SETTINGS.includes(value) ? value : 'off';
	} catch {
		return 'off';
	}
}

class AutoSaveSetting {
	value = $state<AutoSave>(browser ? read() : 'off');

	set(value: AutoSave) {
		this.value = value;
		try {
			localStorage.setItem(KEY, value);
		} catch {
			/* private mode: for this page only */
		}
	}
}

export const autoSave = new AutoSaveSetting();

/** the access grants' latest end (device code, account) – what a gated chapter is saved with */
export function accessEnd(membership: { deviceValid: boolean; device?: { expires_at?: string | null } | null }, session: { hasAccess: boolean; me?: { access?: { expires_at?: string | null } | null } | null }): string | null {
	return accessUntilOf([
		...(membership.deviceValid ? [membership.device?.expires_at ?? null] : []),
		...(session.hasAccess ? [session.me?.access?.expires_at ?? null] : [])
	]);
}

/** tried on this page already (a failed or cancelled one isn't tried again until a reload) */
const tried = new Set<string>();

/** The chapter page: save the opened chapter when the setting and the connection allow it. */
export async function autoSaveChapter(chapter: { toon: string; number: number; title: string; label: string; toonTitle: string; locale: string; accessUntil: string | null }) {
	const id = `${chapter.toon}/${chapter.number}`;
	if (!savedChapters.supported || tried.has(id)) return;
	if (savedChapters.get(chapter.toon, chapter.number) || savedChapters.job(chapter.toon, chapter.number)) return;
	if (!autoSaveAllowed(autoSave.value, (navigator as Navigator & { connection?: ConnectionHint }).connection)) return;
	tried.add(id);
	// never in the way of the first screen
	await whenFirstScreen();
	const level = effectiveQuality(mediaQuality.setting ?? 'auto', mediaQuality.auto);
	try {
		const list = await savedChapters.files(chapter.toon, chapter.number, chapter.locale);
		await savedChapters.save(chapter.toon, chapter.number, {
			title: chapter.title,
			label: chapter.label,
			toonTitle: chapter.toonTitle,
			locale: chapter.locale,
			level,
			videos: autoSaveVideos(planSave(list.files, level, true)),
			list,
			accessUntil: chapter.accessUntil
		});
	} catch {
		/* not saved: the season's icon on the toon page can still save it by hand */
	}
}
